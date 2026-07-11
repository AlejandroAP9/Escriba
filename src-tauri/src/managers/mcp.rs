//! Servidor MCP local (Model Context Protocol): expone las capacidades locales
//! de Escriba —transcribir, traducir, resumir— como tools para agentes de IA
//! (Claude Code, Cursor, Cline). JSON-RPC 2.0 sobre HTTP (transporte
//! "streamable"), escuchando SOLO en 127.0.0.1. 100% local: ni el audio ni el
//! texto salen del equipo. Reusa el patrón del servidor axum del Intérprete.

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use log::{info, warn};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{AppHandle, Manager};

const PROTOCOL_VERSION: &str = "2025-06-18";

pub struct McpServer {
    port: AtomicU16,
    shutdown: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    app: Mutex<Option<AppHandle>>,
}

static SERVER: OnceLock<McpServer> = OnceLock::new();

pub fn global() -> &'static McpServer {
    SERVER.get_or_init(|| McpServer {
        port: AtomicU16::new(0),
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
        Some(McpInfo {
            port,
            url: format!("http://127.0.0.1:{}/mcp", port),
        })
    }

    fn app(&self) -> Option<AppHandle> {
        self.app.lock().ok().and_then(|a| a.clone())
    }

    /// Levanta el servidor MCP en 127.0.0.1 (solo local). Idempotente.
    pub async fn start(&'static self, app: AppHandle) -> Result<McpInfo, String> {
        if let Some(info) = self.info() {
            return Ok(info);
        }
        *self.app.lock().unwrap() = Some(app);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| format!("No se pudo abrir el servidor MCP: {}", e))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();

        let router = Router::new()
            .route("/mcp", post(mcp_handler))
            .with_state(self);

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        self.port.store(port, Ordering::Relaxed);
        *self.shutdown.lock().unwrap() = Some(stop_tx);

        info!("MCP server live at http://127.0.0.1:{}/mcp", port);
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
            url: format!("http://127.0.0.1:{}/mcp", port),
        })
    }

    pub fn stop(&self) {
        if let Some(tx) = self.shutdown.lock().unwrap().take() {
            let _ = tx.send(());
        }
        self.port.store(0, Ordering::Relaxed);
    }
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
        }
    ])
}

/// Punto de entrada JSON-RPC 2.0. Un mensaje por request (modo sin estado).
async fn mcp_handler(
    State(server): State<&'static McpServer>,
    Json(req): Json<Value>,
) -> impl IntoResponse {
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
async fn transcribe_file(app: &AppHandle, path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    if !crate::studio::decode::supported_extension(&path_buf) {
        return Err("Formato de archivo no soportado".to_string());
    }
    if !path_buf.exists() {
        return Err(format!("El archivo no existe: {}", path));
    }
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
