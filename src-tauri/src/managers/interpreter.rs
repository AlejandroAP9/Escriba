//! Intérprete en vivo (Capacidad B): el guía levanta una sala; el Mac sirve
//! una web en la LAN; los asistentes se unen por navegador y reciben la
//! traducción por SSE. Fase 1: esqueleto de conectividad (broadcast de texto,
//! sin audio todavía). 100% local: el servidor solo escucha con sala activa.

use axum::{
    extract::{ConnectInfo, Query, State},
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use log::{info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU16, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::broadcast;
use tokio_stream::StreamExt;

const VISITOR_HTML: &str = include_str!("../../interpreter/visitor.html");

/// Rate-limit del acceso a la sala: máx. intentos fallidos por IP antes de 429.
const MAX_FAILED_ATTEMPTS: u32 = 10;
/// Tope de oyentes SSE simultáneos: una sala de tour/clase real no pasa de
/// decenas; el límite evita que alguien en la red agote conexiones del server.
const MAX_LISTENERS: u32 = 64;
/// Ventana de bloqueo tras superar el tope.
const LOCKOUT: Duration = Duration::from_secs(30);

/// Token hex aleatorio (CSPRNG) que va en la URL del QR. Sin él, el código de
/// 4 dígitos no basta para entrar: mata el brute-force de 10.000 combinaciones.
fn random_token() -> String {
    let mut bytes = [0u8; 16];
    if getrandom::getrandom(&mut bytes).is_err() {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        bytes.copy_from_slice(&n.to_le_bytes());
    }
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Código de 4 dígitos para mostrar al humano, ahora desde un CSPRNG (antes se
/// derivaba del reloj, lo que lo hacía predecible).
fn random_code() -> String {
    let mut bytes = [0u8; 4];
    if getrandom::getrandom(&mut bytes).is_err() {
        // Mismo respaldo que random_token: sin esto, un fallo del CSPRNG
        // dejaría los bytes en cero y el código sería siempre "0000".
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u32)
            .unwrap_or(0);
        bytes.copy_from_slice(&n.to_le_bytes());
    }
    let n = u32::from_le_bytes(bytes);
    format!("{:04}", n % 10_000)
}

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
    /// Token de la sala (va en la URL del QR). El acceso exige código + token.
    room_token: Mutex<Option<String>>,
    /// Intentos fallidos por IP para el rate-limit del brute-force.
    failed: Mutex<HashMap<IpAddr, (u32, Instant)>>,
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
            room_token: Mutex::new(None),
            failed: Mutex::new(HashMap::new()),
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

        let code = random_code();
        let token = random_token();
        let listener = tokio::net::TcpListener::bind("0.0.0.0:0")
            .await
            .map_err(|e| format!("No se pudo abrir el servidor local: {}", e))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();

        let ip = local_ip_address::local_ip()
            .map_err(|e| format!("No se pudo detectar la IP de la red: {}", e))?;
        let url = format!("http://{}:{}/?room={}&t={}", ip, port, code, token);

        let app = Router::new()
            .route("/", get(visitor_page))
            .route("/events", get(sse_handler))
            .route("/join", get(join_handler))
            .with_state(self);

        let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
        *self.room_code.lock().unwrap() = Some(code.clone());
        *self.room_token.lock().unwrap() = Some(token.clone());
        self.failed.lock().unwrap().clear();
        self.port.store(port, Ordering::Relaxed);
        self.listeners.store(0, Ordering::Relaxed);
        self.active_langs.lock().unwrap().clear();
        *self.shutdown.lock().unwrap() = Some(stop_tx);

        // Log sin el token (secreto): solo el código de display.
        info!("Interpreter room {} live on port {}", code, port);
        tauri::async_runtime::spawn(async move {
            let make = app.into_make_service_with_connect_info::<SocketAddr>();
            let server = axum::serve(listener, make).with_graceful_shutdown(async {
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
        *self.room_token.lock().unwrap() = None;
        self.failed.lock().unwrap().clear();
        self.listeners.store(0, Ordering::Relaxed);
        self.active_langs.lock().unwrap().clear();
    }

    fn current_room(&self) -> Option<RoomInfo> {
        let code = self.room_code.lock().unwrap().clone()?;
        let token = self.room_token.lock().unwrap().clone()?;
        let port = self.port.load(Ordering::Relaxed);
        let ip = local_ip_address::local_ip().ok()?;
        let url = format!("http://{}:{}/?room={}&t={}", ip, port, code, token);
        let qr_svg = qr_svg(&url);
        Some(RoomInfo {
            code,
            url,
            qr_svg,
            port,
        })
    }

    /// Valida código + token de la sala. Ambos deben coincidir para entrar.
    fn access_ok(&self, code: &str, token: &str) -> bool {
        let code_ok = self
            .room_code
            .lock()
            .map(|c| c.as_deref() == Some(code))
            .unwrap_or(false);
        let token_ok = self
            .room_token
            .lock()
            .map(|t| t.as_deref() == Some(token))
            .unwrap_or(false);
        code_ok && token_ok
    }

    /// Rate-limit por IP: true si la IP está bloqueada (superó el tope dentro
    /// de la ventana). Limpia entradas expiradas de paso.
    fn is_locked(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut map = match self.failed.lock() {
            Ok(m) => m,
            Err(_) => return false,
        };
        map.retain(|_, (_, when)| now.duration_since(*when) < LOCKOUT);
        map.get(&ip)
            .map(|(count, when)| {
                *count >= MAX_FAILED_ATTEMPTS && now.duration_since(*when) < LOCKOUT
            })
            .unwrap_or(false)
    }

    /// Registra un intento fallido de acceso desde esta IP.
    fn record_failure(&self, ip: IpAddr) {
        let now = Instant::now();
        if let Ok(mut map) = self.failed.lock() {
            let entry = map.entry(ip).or_insert((0, now));
            // Reinicia la ventana si el último fallo ya expiró.
            if now.duration_since(entry.1) >= LOCKOUT {
                *entry = (0, now);
            }
            entry.0 += 1;
            entry.1 = now;
        }
    }

    /// Limpia el historial de fallos de una IP tras un acceso correcto.
    fn clear_failures(&self, ip: IpAddr) {
        if let Ok(mut map) = self.failed.lock() {
            map.remove(&ip);
        }
    }
}

#[derive(Deserialize)]
struct EventsQuery {
    room: String,
    /// Token secreto de la sala (viene en la URL del QR).
    #[serde(default)]
    t: String,
    #[serde(default)]
    lang: String,
}

async fn visitor_page() -> Html<&'static str> {
    Html(VISITOR_HTML)
}

async fn join_handler(
    State(server): State<&'static InterpreterServer>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    let ip = addr.ip();
    if server.is_locked(ip) {
        return (StatusCode::TOO_MANY_REQUESTS, "demasiados intentos").into_response();
    }
    if !server.access_ok(&q.room, &q.t) {
        server.record_failure(ip);
        return (StatusCode::FORBIDDEN, "sala no encontrada").into_response();
    }
    server.clear_failures(ip);
    // El conteo de oyentes e idiomas lo lleva el stream SSE (con guard que
    // descuenta al desconectar). /join solo valida el acceso.
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    let ip = addr.ip();
    if server.is_locked(ip) {
        return (StatusCode::TOO_MANY_REQUESTS, "demasiados intentos").into_response();
    }
    if !server.access_ok(&q.room, &q.t) {
        server.record_failure(ip);
        return (StatusCode::FORBIDDEN, "sala no encontrada").into_response();
    }
    server.clear_failures(ip);
    if server.listeners() >= MAX_LISTENERS {
        return (StatusCode::SERVICE_UNAVAILABLE, "sala llena").into_response();
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
