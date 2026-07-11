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
use std::collections::HashSet;
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
    pub text: String,
    pub seq: u64,
}

pub struct InterpreterServer {
    tx: broadcast::Sender<InterpreterLine>,
    room_code: Mutex<Option<String>>,
    port: AtomicU16,
    listeners: AtomicU32,
    seq: std::sync::atomic::AtomicU64,
    /// Idiomas que los oyentes están pidiendo (para traducir solo esos).
    active_langs: Mutex<HashSet<String>>,
    shutdown: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
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
            active_langs: Mutex::new(HashSet::new()),
            shutdown: Mutex::new(None),
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
    pub fn is_running(&self) -> bool {
        self.room_code.lock().map(|c| c.is_some()).unwrap_or(false)
    }

    pub fn listeners(&self) -> u32 {
        self.listeners.load(Ordering::Relaxed)
    }

    pub fn active_languages(&self) -> Vec<String> {
        self.active_langs
            .lock()
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Publica una línea a todos los oyentes conectados.
    pub fn publish(&self, text: String) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let _ = self.tx.send(InterpreterLine { text, seq });
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
    if !q.lang.is_empty() {
        server.active_langs.lock().unwrap().insert(q.lang.clone());
    }
    (StatusCode::OK, "ok").into_response()
}

async fn sse_handler(
    State(server): State<&'static InterpreterServer>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    if !server.code_matches(&q.room) {
        return (StatusCode::FORBIDDEN, "sala no encontrada").into_response();
    }
    if !q.lang.is_empty() {
        server.active_langs.lock().unwrap().insert(q.lang.clone());
    }
    server.listeners.fetch_add(1, Ordering::Relaxed);

    let rx = server.tx.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
        move |result| -> Option<Result<Event, Infallible>> {
            match result {
                Ok(line) => Some(Ok(Event::default()
                    .json_data(&line)
                    .unwrap_or_else(|_| Event::default().data(line.text)))),
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
