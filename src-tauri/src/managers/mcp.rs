//! Servidor MCP local (Model Context Protocol): expone las capacidades locales
//! de Escriba —transcribir, traducir, resumir— como tools para agentes de IA
//! (Claude Code, Cursor, Cline). JSON-RPC 2.0 sobre HTTP (transporte
//! "streamable"), escuchando SOLO en 127.0.0.1. 100% local: ni el audio ni el
//! texto salen del equipo. Reusa el patrón del servidor axum del Intérprete.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use log::{info, warn};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Manager};

/// Genera un token hex aleatorio (CSPRNG) para autenticar el servidor local.
/// El token va embebido en la URL que el usuario copia al agente, así que un
/// proceso local cualquiera no puede adivinar el endpoint ni leer el historial.
fn random_token() -> String {
    let mut bytes = [0u8; 24];
    // getrandom nunca debería fallar en desktop; si lo hace, mezclamos el reloj
    // como último recurso para no dejar el servidor sin token.
    if getrandom::getrandom(&mut bytes).is_err() {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        bytes[..16].copy_from_slice(&n.to_le_bytes());
    }
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

const PROTOCOL_VERSION: &str = "2025-06-18";

/// Puerto preferido del servidor MCP. Fijo para que la configuración del
/// agente (`claude mcp add ... --url http://127.0.0.1:5151/mcp`) sobreviva a
/// los reinicios de la app. Si está ocupado, se cae a un puerto efímero.
const PREFERRED_PORT: u16 = 5151;

pub struct McpServer {
    port: AtomicU16,
    /// Token de acceso regenerado en cada `start()`. Va en la ruta (`/mcp/{token}`).
    token: Mutex<Option<String>>,
    shutdown: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    app: Mutex<Option<AppHandle>>,
}

static SERVER: OnceLock<McpServer> = OnceLock::new();

pub fn global() -> &'static McpServer {
    SERVER.get_or_init(|| McpServer {
        port: AtomicU16::new(0),
        token: Mutex::new(None),
        shutdown: Mutex::new(None),
        app: Mutex::new(None),
    })
}

#[derive(Clone, serde::Serialize)]
pub struct McpInfo {
    pub port: u16,
    pub url: String,
}

impl McpServer {
    pub fn is_running(&self) -> bool {
        self.shutdown.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    pub fn port(&self) -> u16 {
        self.port.load(Ordering::Relaxed)
    }

    pub fn info(&self) -> Option<McpInfo> {
        if !self.is_running() {
            return None;
        }
        let port = self.port();
        let token = self.token.lock().ok().and_then(|t| t.clone())?;
        Some(McpInfo {
            port,
            url: format!("http://127.0.0.1:{}/mcp/{}", port, token),
        })
    }

    fn app(&self) -> Option<AppHandle> {
        self.app.lock().ok().and_then(|a| a.clone())
    }

    fn token_matches(&self, given: &str) -> bool {
        self.token
            .lock()
            .map(|t| t.as_deref() == Some(given))
            .unwrap_or(false)
    }

    /// Levanta el servidor MCP en 127.0.0.1 (solo local). Idempotente.
    pub async fn start(&'static self, app: AppHandle) -> Result<McpInfo, String> {
        if let Some(info) = self.info() {
            return Ok(info);
        }
        *self.app.lock().unwrap() = Some(app);

        // Puerto fijo primero (config estable en el agente); efímero de respaldo.
        let listener = match tokio::net::TcpListener::bind(("127.0.0.1", PREFERRED_PORT)).await {
            Ok(l) => l,
            Err(_) => tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .map_err(|e| format!("No se pudo abrir el servidor MCP: {}", e))?,
        };
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();

        // Token de acceso nuevo en cada arranque: sin él, 401.
        let token = random_token();
        *self.token.lock().unwrap() = Some(token.clone());

        let router = Router::new()
            .route("/mcp/:token", post(mcp_handler))
            .layer(middleware::from_fn(guard_local_only))
            .with_state(self);

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        self.port.store(port, Ordering::Relaxed);
        *self.shutdown.lock().unwrap() = Some(stop_tx);

        info!("MCP server live at http://127.0.0.1:{}/mcp/<token>", port);
        tauri::async_runtime::spawn(async move {
            let server = axum::serve(listener, router).with_graceful_shutdown(async {
                let _ = stop_rx.await;
            });
            if let Err(e) = server.await {
                warn!("MCP server error: {}", e);
            }
            info!("MCP server stopped");
        });

        Ok(McpInfo {
            port,
            url: format!("http://127.0.0.1:{}/mcp/{}", port, token),
        })
    }

    pub fn stop(&self) {
        if let Some(tx) = self.shutdown.lock().unwrap().take() {
            let _ = tx.send(());
        }
        self.port.store(0, Ordering::Relaxed);
        *self.token.lock().unwrap() = None;
    }
}

/// Middleware anti DNS-rebinding: solo acepta requests cuyo `Host` sea loopback
/// y cuyo `Origin` (si viene) sea `null`/loopback. Un sitio web malicioso que
/// re-bindee su dominio a 127.0.0.1 mandaría un Origin/Host de su dominio → 403.
async fn guard_local_only(
    headers: HeaderMap,
    req: axum::extract::Request,
    next: Next,
) -> axum::response::Response {
    fn host_is_loopback(h: &str) -> bool {
        let host = h.rsplit_once(':').map(|(h, _)| h).unwrap_or(h);
        host == "127.0.0.1" || host == "localhost" || host == "[::1]" || host == "::1"
    }
    if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
        if !host_is_loopback(host) {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    if let Some(origin) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        let ok = origin == "null"
            || origin
                .rsplit_once("//")
                .map(|(_, hostport)| host_is_loopback(hostport))
                .unwrap_or(false);
        if !ok {
            return StatusCode::FORBIDDEN.into_response();
        }
    }
    next.run(req).await
}

/// Definición de las tools MCP que Escriba expone a los agentes.
fn tool_definitions() -> Value {
    json!([
        {
            "name": "transcribe",
            "description": "Transcribe un archivo de audio o video LOCAL a texto con el motor de Escriba (100% local, sin nube). Devuelve el texto completo.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Ruta absoluta al archivo de audio/video (mp3, m4a, wav, mp4, mov, opus, flac, ...)."
                    }
                },
                "required": ["path"]
            }
        },
        {
            "name": "translate",
            "description": "Traduce un texto al idioma destino (código ISO, p.ej. 'en', 'es', 'lt', 'pt') con el motor local de Escriba.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Texto a traducir." },
                    "target_lang": { "type": "string", "description": "Código ISO del idioma destino." }
                },
                "required": ["text", "target_lang"]
            }
        },
        {
            "name": "summarize",
            "description": "Resume un texto (transcripción, notas, artículo) con el motor local de Escriba.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Texto a resumir." }
                },
                "required": ["text"]
            }
        },
        {
            "name": "polish",
            "description": "Corrige y pule un texto con el motor local de Escriba: elimina muletillas y repeticiones, arregla puntuación y mayúsculas. Conserva idioma y significado.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Texto a corregir." }
                },
                "required": ["text"]
            }
        },
        {
            "name": "list_dictations",
            "description": "Lista los dictados recientes del historial de Escriba (fecha + texto). Útil para resumir o retomar lo que el usuario dictó. Todo local.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Máximo de dictados a devolver (por defecto 10, tope 50)." },
                    "query": { "type": "string", "description": "Filtro opcional: solo dictados que contengan este texto." }
                }
            }
        },
        {
            "name": "usage_stats",
            "description": "Estadísticas de uso de Escriba: transcripciones, palabras, racha y minutos ahorrados.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

/// Punto de entrada JSON-RPC 2.0. Un mensaje por request (modo sin estado).
/// El token de la ruta autentica al cliente: sin él (o incorrecto), 401.
async fn mcp_handler(
    State(server): State<&'static McpServer>,
    Path(token): Path<String>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
    if !server.token_matches(&token) {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = req.get("id").cloned();

    // Notificaciones (sin id): el cliente no espera respuesta.
    let Some(id) = id else {
        return StatusCode::ACCEPTED.into_response();
    };

    let result: Result<Value, (i64, String)> = match method {
        "initialize" => {
            let client_ver = req
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(|v| v.as_str())
                .unwrap_or(PROTOCOL_VERSION)
                .to_string();
            Ok(json!({
                "protocolVersion": client_ver,
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "escriba", "version": env!("CARGO_PKG_VERSION") }
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => call_tool(server, req.get("params")).await,
        other => Err((-32601, format!("Método no soportado: {}", other))),
    };

    let body = match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err((code, message)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        }
    };
    Json(body).into_response()
}

/// Ejecuta una tool. Los errores de la tool se devuelven como resultado con
/// `isError: true` (convención MCP) para que el agente vea el mensaje.
async fn call_tool(
    server: &'static McpServer,
    params: Option<&Value>,
) -> Result<Value, (i64, String)> {
    let params = params.ok_or((-32602, "Faltan params".to_string()))?;
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let app = server
        .app()
        .ok_or((-32603, "App no disponible".to_string()))?;

    let text_result: Result<String, String> = match name {
        "transcribe" => {
            let path = args
                .get("path")
                .and_then(|p| p.as_str())
                .unwrap_or("")
                .to_string();
            transcribe_file(&app, path).await
        }
        "translate" => {
            let text = args
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            let target = args
                .get("target_lang")
                .and_then(|t| t.as_str())
                .unwrap_or("en")
                .to_string();
            crate::actions::translate_text(&app, &text, &target)
                .await
                .ok_or_else(|| "No se pudo traducir (¿motor local disponible?)".to_string())
        }
        "polish" => {
            let text = args
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            crate::actions::polish_text(&app, &text)
                .await
                .ok_or_else(|| "No se pudo corregir (¿motor local disponible?)".to_string())
        }
        "list_dictations" => list_dictations(&app, &args).await,
        "usage_stats" => usage_stats_text(&app),
        "summarize" => {
            let text = args
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("")
                .to_string();
            crate::actions::summarize_text(&app, &text)
                .await
                .ok_or_else(|| "No se pudo resumir (¿motor local disponible?)".to_string())
        }
        other => return Err((-32602, format!("Tool desconocida: {}", other))),
    };

    match text_result {
        Ok(text) => Ok(json!({ "content": [ { "type": "text", "text": text } ] })),
        Err(e) => Ok(json!({ "content": [ { "type": "text", "text": e } ], "isError": true })),
    }
}

/// Decodifica y transcribe un archivo local, reusando el pipeline del Estudio.
/// El path se restringe al home del usuario y al app-data dir (canonicalizado):
/// así el tool no lee archivos del sistema ni de otros usuarios, y un único
/// mensaje de error genérico evita usarlo como oráculo de existencia.
async fn transcribe_file(app: &AppHandle, path: String) -> Result<String, String> {
    // Mensaje único para "no existe", "formato no soportado" y "fuera de límites":
    // el que llama no puede distinguir qué archivos existen en el disco.
    const GENERIC_ERR: &str =
        "No se pudo transcribir: archivo no accesible o formato no soportado.";
    let path_buf = PathBuf::from(&path);

    if !crate::studio::decode::supported_extension(&path_buf) {
        return Err(GENERIC_ERR.to_string());
    }
    let canonical = std::fs::canonicalize(&path_buf).map_err(|_| GENERIC_ERR.to_string())?;

    // Raíces consentidas: home del usuario + app-data dir.
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(home) = app.path().home_dir() {
        if let Ok(home) = std::fs::canonicalize(home) {
            roots.push(home);
        }
    }
    if let Ok(data) = crate::portable::app_data_dir(app) {
        if let Ok(data) = std::fs::canonicalize(data) {
            roots.push(data);
        }
    }
    if !roots.iter().any(|r| canonical.starts_with(r)) {
        return Err(GENERIC_ERR.to_string());
    }
    let path_buf = canonical;

    let tm = app
        .state::<Arc<crate::managers::transcription::TranscriptionManager>>()
        .inner()
        .clone();
    let segments = tauri::async_runtime::spawn_blocking(
        move || -> Result<Vec<crate::studio::segments::Segment>, String> {
            let samples = crate::studio::decode::decode_to_16k_mono(&path_buf)?;
            crate::studio::pipeline::transcribe_samples(&tm, &samples, |_| {})
        },
    )
    .await
    .map_err(|e| format!("La transcripción falló: {}", e))??;
    let paragraphs = crate::studio::segments::group_paragraphs(&segments);
    Ok(paragraphs.join("\n\n"))
}

/// Dictados recientes del historial (fecha local + texto), con filtro opcional.
async fn list_dictations(app: &AppHandle, args: &Value) -> Result<String, String> {
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .min(50) as usize;
    let query = args
        .get("query")
        .and_then(|q| q.as_str())
        .map(|s| s.to_lowercase());

    let hm = app
        .state::<Arc<crate::managers::history::HistoryManager>>()
        .inner()
        .clone();
    let page = hm
        .get_history_entries(None, Some(50))
        .await
        .map_err(|e| e.to_string())?;

    let mut lines = Vec::new();
    for entry in page.entries {
        let text = entry
            .post_processed_text
            .clone()
            .unwrap_or_else(|| entry.transcription_text.clone());
        if text.trim().is_empty() {
            continue;
        }
        if let Some(q) = &query {
            if !text.to_lowercase().contains(q) {
                continue;
            }
        }
        let when = chrono::DateTime::from_timestamp(entry.timestamp, 0)
            .map(|d| {
                d.with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_default();
        lines.push(format!("[{}] {}", when, text.trim()));
        if lines.len() >= limit {
            break;
        }
    }

    if lines.is_empty() {
        return Ok("Sin dictados que coincidan.".to_string());
    }
    Ok(lines.join("\n"))
}

/// Estadísticas de uso en texto plano para el agente.
fn usage_stats_text(app: &AppHandle) -> Result<String, String> {
    let hm = app.state::<Arc<crate::managers::history::HistoryManager>>();
    let s = hm.get_usage_stats().map_err(|e| e.to_string())?;
    Ok(format!(
        "Transcripciones totales: {}\nPalabras totales: {}\nPalabras últimos 30 días: {}\nDías activos últimos 30: {}\nRacha actual: {} días\nMinutos ahorrados (vs teclear a 40 ppm): {}",
        s.total_transcriptions,
        s.total_words,
        s.words_last_30_days,
        s.active_days_last_30,
        s.current_streak_days,
        s.minutes_saved
    ))
}
