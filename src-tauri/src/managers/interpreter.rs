//! Intérprete en vivo (Capacidad B): el guía levanta una sala; el Mac sirve
//! una web en la LAN; los asistentes se unen por navegador y reciben la
//! traducción por SSE. Fase 1: esqueleto de conectividad (broadcast de texto,
//! sin audio todavía). 100% local: el servidor solo escucha con sala activa.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use log::{info, warn};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;

const VISITOR_HTML: &str = include_str!("../../interpreter/visitor.html");

/// Una línea publicada por el guía. En Fase 1 es el texto de prueba; en fases
/// siguientes traerá el texto origen y luego las traducciones por idioma.
#[derive(Clone, serde::Serialize)]
pub struct InterpreterLine {
    /// Texto en el idioma de origen (lo que dijo/escribió el guía).
    pub source: String,
    /// Traducciones listas por idioma ISO. El SSE de cada oyente elige la suya.
    pub translations: std::collections::HashMap<String, String>,
    pub seq: u64,
}

pub struct InterpreterServer {
    tx: broadcast::Sender<InterpreterLine>,
    room_code: Mutex<Option<String>>,
    port: AtomicU16,
    listeners: AtomicU32,
    seq: std::sync::atomic::AtomicU64,
    /// Idiomas que los oyentes están pidiendo (para traducir solo esos).
    active_langs: Mutex<std::collections::HashMap<String, u32>>,
    shutdown: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    source_lang: Mutex<String>,
    listening: std::sync::atomic::AtomicBool,
}

static SERVER: OnceLock<InterpreterServer> = OnceLock::new();

pub fn global() -> &'static InterpreterServer {
    SERVER.get_or_init(|| {
        let (tx, _) = broadcast::channel(64);
        InterpreterServer {
            tx,
            room_code: Mutex::new(None),
            port: AtomicU16::new(0),
            listeners: AtomicU32::new(0),
            seq: std::sync::atomic::AtomicU64::new(0),
            active_langs: Mutex::new(std::collections::HashMap::new()),
            shutdown: Mutex::new(None),
            source_lang: Mutex::new("es".to_string()),
            listening: std::sync::atomic::AtomicBool::new(false),
        }
    })
}

#[derive(Clone, serde::Serialize)]
pub struct RoomInfo {
    pub code: String,
    pub url: String,
    pub qr_svg: String,
    pub port: u16,
}

impl InterpreterServer {
    pub fn set_source_lang(&self, lang: String) {
        if let Ok(mut l) = self.source_lang.lock() {
            *l = lang;
        }
    }

    pub fn source_lang(&self) -> String {
        self.source_lang
            .lock()
            .map(|l| l.clone())
            .unwrap_or_else(|_| "es".to_string())
    }

    pub fn set_listening(&self, on: bool) {
        self.listening
            .store(on, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_listening(&self) -> bool {
        self.listening.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_running(&self) -> bool {
        self.room_code.lock().map(|c| c.is_some()).unwrap_or(false)
    }

    pub fn listeners(&self) -> u32 {
        self.listeners.load(Ordering::Relaxed)
    }

    pub fn active_languages(&self) -> Vec<String> {
        self.active_langs
            .lock()
            .map(|m| {
                m.iter()
                    .filter(|(_, c)| **c > 0)
                    .map(|(k, _)| k.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Publica una línea (source + traducciones) a los oyentes conectados.
    pub fn publish_line(
        &self,
        source: String,
        translations: std::collections::HashMap<String, String>,
    ) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(InterpreterLine {
            source,
            translations,
            seq,
        });
    }

    /// Levanta el servidor en la LAN y devuelve el código + QR. Idempotente:
    /// si ya hay sala, devuelve la existente.
    pub async fn start(&'static self) -> Result<RoomInfo, String> {
        if let Some(existing) = self.current_room() {
            return Ok(existing);
        }

        let code = pseudo_room_code();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("No se pudo abrir el servidor local: {}", e))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();

        let ip = local_ip_address::local_ip()
            .map_err(|e| format!("No se pudo detectar la IP de la red: {}", e))?;
        let url = format!("http://{}:{}/?room={}", ip, port, code);

        let app = Router::new()
            .route("/", get(visitor_page))
            .route("/events", get(sse_handler))
            .route("/join", get(join_handler))
            .with_state(self);

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        *self.room_code.lock().unwrap() = Some(code.clone());
        self.port.store(port, Ordering::Relaxed);
        self.listeners.store(0, Ordering::Relaxed);
        self.active_langs.lock().unwrap().clear();
        *self.shutdown.lock().unwrap() = Some(stop_tx);

        info!("Interpreter room {} live at {}", code, url);
        tauri::async_runtime::spawn(async move {
            let server = axum::serve(listener, app).with_graceful_shutdown(async {
                let _ = stop_rx.await;
            });
            if let Err(e) = server.await {
                warn!("Interpreter server error: {}", e);
            }
            info!("Interpreter server stopped");
        });

        let qr_svg = qr_svg(&url);
        Ok(RoomInfo {
            code,
            url,
            qr_svg,
            port,
        })
    }

    pub fn stop(&self) {
        self.listening
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(tx) = self.shutdown.lock().unwrap().take() {
            let _ = tx.send(());
        }
        *self.room_code.lock().unwrap() = None;
        self.listeners.store(0, Ordering::Relaxed);
        self.active_langs.lock().unwrap().clear();
    }

    fn current_room(&self) -> Option<RoomInfo> {
        let code = self.room_code.lock().unwrap().clone()?;
        let port = self.port.load(Ordering::Relaxed);
        let ip = local_ip_address::local_ip().ok()?;
        let url = format!("http://{}:{}/?room={}", ip, port, code);
        let qr_svg = qr_svg(&url);
        Some(RoomInfo {
            code,
            url,
            qr_svg,
            port,
        })
    }

    fn code_matches(&self, given: &str) -> bool {
        self.room_code
            .lock()
            .map(|c| c.as_deref() == Some(given))
            .unwrap_or(false)
    }
}

#[derive(Deserialize)]
struct EventsQuery {
    room: String,
    #[serde(default)]
    lang: String,
}

async fn visitor_page() -> Html<&'static str> {
    Html(VISITOR_HTML)
}

async fn join_handler(
    State(server): State<&'static InterpreterServer>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    if !server.code_matches(&q.room) {
        return (StatusCode::FORBIDDEN, "sala no encontrada").into_response();
    }
    // El conteo de oyentes e idiomas lo lleva el stream SSE (con guard que
    // descuenta al desconectar). /join solo valida el codigo.
    (StatusCode::OK, "ok").into_response()
}

/// Guard: incrementa oyente + idioma al conectar el SSE y los descuenta al
/// soltar el stream (el navegador cierra la conexion al cambiar de idioma o
/// cerrar la pestaña). Sin esto los contadores solo crecerian.
struct ListenerGuard {
    server: &'static InterpreterServer,
    lang: String,
}

impl ListenerGuard {
    fn new(server: &'static InterpreterServer, lang: String) -> Self {
        server.listeners.fetch_add(1, Ordering::Relaxed);
        if !lang.is_empty() {
            *server
                .active_langs
                .lock()
                .unwrap()
                .entry(lang.clone())
                .or_insert(0) += 1;
        }
        ListenerGuard { server, lang }
    }
}

impl Drop for ListenerGuard {
    fn drop(&mut self) {
        let prev = self.server.listeners.load(Ordering::Relaxed);
        if prev > 0 {
            self.server.listeners.store(prev - 1, Ordering::Relaxed);
        }
        if !self.lang.is_empty() {
            if let Ok(mut m) = self.server.active_langs.lock() {
                if let Some(c) = m.get_mut(&self.lang) {
                    *c = c.saturating_sub(1);
                    if *c == 0 {
                        m.remove(&self.lang);
                    }
                }
            }
        }
    }
}

async fn sse_handler(
    State(server): State<&'static InterpreterServer>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    if !server.code_matches(&q.room) {
        return (StatusCode::FORBIDDEN, "sala no encontrada").into_response();
    }
    let lang = q.lang.clone();
    let guard = ListenerGuard::new(server, lang.clone());
    let rx = server.tx.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
        move |result| -> Option<Result<Event, Infallible>> {
            // Capturado por la clausura: vive tanto como el stream y descuenta
            // al soltarse (cambio de idioma / cierre de pestaña).
            let _keep = &guard;
            match result {
                Ok(line) => {
                    let text = line
                        .translations
                        .get(&lang)
                        .cloned()
                        .unwrap_or_else(|| line.source.clone());
                    #[derive(serde::Serialize)]
                    struct VisitorLine {
                        text: String,
                        seq: u64,
                    }
                    Some(Ok(Event::default()
                        .json_data(VisitorLine {
                            text,
                            seq: line.seq,
                        })
                        .unwrap_or_else(|_| Event::default().data(line.source))))
                }
                Err(_) => None,
            }
        },
    );

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// Código de sala pseudo-aleatorio sin depender de `rand` (no está en deps):
/// derivado del nanosegundo del reloj. Suficiente para una sala efímera.
fn pseudo_room_code() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("{:04}", nanos % 10_000)
}

fn qr_svg(url: &str) -> String {
    use qrcode::render::svg;
    use qrcode::QrCode;
    match QrCode::new(url.as_bytes()) {
        Ok(code) => code
            .render::<svg::Color>()
            .min_dimensions(220, 220)
            .quiet_zone(true)
            .build(),
        Err(_) => String::new(),
    }
}
