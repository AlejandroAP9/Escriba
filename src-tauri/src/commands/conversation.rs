//! Modo Conversación: una sesión de voz que termina en un documento.
//! Dos usos sobre el mismo núcleo: "converse" (la IA responde cada turno,
//! el frontend lo lee en voz alta) y "listen" (la IA calla y solo acumula:
//! entrevistas, notas habladas, actas). En ambos, al finalizar, el motor
//! local convierte la sesión en un documento limpio. Nada sale del equipo.
//! Sigue el patrón del Traductor: estado estático + interceptor en actions.rs.

use serde::Serialize;
use specta::Type;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Proceso `say` en curso (macOS): se mata antes de hablar de nuevo o al parar.
static SPEAKING: Mutex<Option<std::process::Child>> = Mutex::new(None);

static LISTENING: AtomicBool = AtomicBool::new(false);
/// Modo de la sesión: "converse" (la IA responde) o una variante de escucha
/// ("listen", "interview", "class", "brainstorm") que solo cambia el documento.
static MODE: Mutex<String> = Mutex::new(String::new());
static TURNS: Mutex<Vec<Turn>> = Mutex::new(Vec::new());
static STARTED: Mutex<Option<Instant>> = Mutex::new(None);

#[derive(Serialize, Clone, Type)]
pub struct Turn {
    /// "user" | "assistant"
    pub role: String,
    pub text: String,
    /// Segundos desde el inicio de la sesión (para mostrar mm:ss).
    pub at_secs: u32,
}

#[derive(Serialize, Clone, Type)]
pub struct ConversationStatus {
    pub listening: bool,
    /// "converse" | "listen"
    pub mode: String,
    pub turns: Vec<Turn>,
}

pub fn is_listening() -> bool {
    LISTENING.load(Ordering::Relaxed)
}

pub fn mode() -> String {
    MODE.lock()
        .map(|m| {
            if m.is_empty() {
                "converse".to_string()
            } else {
                m.clone()
            }
        })
        .unwrap_or_else(|_| "converse".to_string())
}

pub fn is_converse_mode() -> bool {
    mode() == "converse"
}

fn elapsed_secs() -> u32 {
    STARTED
        .lock()
        .ok()
        .and_then(|s| *s)
        .map(|t| t.elapsed().as_secs() as u32)
        .unwrap_or(0)
}

/// Registra un turno y lo devuelve (con su marca de tiempo) para emitirlo.
pub fn push_turn(role: &str, text: &str) -> Turn {
    let turn = Turn {
        role: role.to_string(),
        text: text.trim().to_string(),
        at_secs: elapsed_secs(),
    };
    if let Ok(mut t) = TURNS.lock() {
        t.push(turn.clone());
    }
    turn
}

/// Transcripción completa para los prompts del LLM ("Usuario: …\nAsistente: …").
pub fn transcript(user_label: &str, assistant_label: &str) -> String {
    TURNS
        .lock()
        .map(|t| {
            t.iter()
                .map(|turn| {
                    let who = if turn.role == "assistant" {
                        assistant_label
                    } else {
                        user_label
                    };
                    format!("{}: {}", who, turn.text)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn status() -> ConversationStatus {
    ConversationStatus {
        listening: is_listening(),
        mode: mode(),
        turns: TURNS.lock().map(|t| t.clone()).unwrap_or_default(),
    }
}

#[tauri::command]
#[specta::specta]
pub fn conversation_start(mode: String) -> ConversationStatus {
    if let Ok(mut m) = MODE.lock() {
        *m = mode;
    }
    // Sesión nueva solo si no hay una en curso (reanudar conserva los turnos).
    {
        let mut started = STARTED.lock().unwrap();
        if started.is_none() {
            *started = Some(Instant::now());
        }
    }
    LISTENING.store(true, Ordering::Relaxed);
    status()
}

#[tauri::command]
#[specta::specta]
pub fn conversation_stop() -> ConversationStatus {
    LISTENING.store(false, Ordering::Relaxed);
    status()
}

#[tauri::command]
#[specta::specta]
pub fn conversation_status() -> ConversationStatus {
    status()
}

#[tauri::command]
#[specta::specta]
pub fn conversation_reset() -> ConversationStatus {
    LISTENING.store(false, Ordering::Relaxed);
    if let Ok(mut t) = TURNS.lock() {
        t.clear();
    }
    *STARTED.lock().unwrap() = None;
    status()
}

/// Lee un texto en voz alta con la cascada de motores de Escriba:
/// 1) Voz neural incluida (sherpa-onnx + Piper es_MX), si está instalada y la
///    app está en español (la voz es de español; otros idiomas caen al paso 2).
/// 2) `say` de macOS (la voz del sistema, que sí ve las Premium/Mejorada que
///    el webview no expone).
/// 3) `false` → el frontend usa speechSynthesis como último respaldo.
#[tauri::command]
#[specta::specta]
pub async fn conversation_speak(app: tauri::AppHandle, text: String) -> bool {
    // Motor #1: voz neural incluida.
    let app_lang = crate::settings::get_settings(&app).app_language;
    if app_lang.starts_with("es") && crate::managers::tts::installed(&app) {
        let app2 = app.clone();
        let text2 = text.clone();
        let ok = tauri::async_runtime::spawn_blocking(move || {
            crate::managers::tts::speak_blocking(&app2, &text2).is_ok()
        })
        .await
        .unwrap_or(false);
        if ok {
            return true;
        }
    }
    // Motor #2: la voz del sistema de macOS.
    #[cfg(target_os = "macos")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};
        let mut guard = SPEAKING.lock().unwrap();
        // Corta lo que estuviera sonando (respuesta nueva pisa a la anterior).
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        // El texto entra por stdin: sin problemas de comillas ni flags.
        match Command::new("/usr/bin/say")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(stdin) = child.stdin.take() {
                    let mut stdin = stdin;
                    let _ = stdin.write_all(text.as_bytes());
                }
                *guard = Some(child);
                true
            }
            Err(_) => false,
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
        false
    }
}

/// Detiene la lectura en voz alta en curso (cualquier motor).
#[tauri::command]
#[specta::specta]
pub fn conversation_speak_stop() {
    crate::managers::tts::stop();
    if let Ok(mut guard) = SPEAKING.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Estado de la voz neural incluida (para la tarjeta de instalación).
#[tauri::command]
#[specta::specta]
pub fn tts_status(app: tauri::AppHandle) -> bool {
    crate::managers::tts::installed(&app)
}

/// Descarga e instala la voz neural (runtime + voz, ~95 MB, SHA256 pinneado).
/// Emite `tts-setup-progress` durante la descarga.
#[tauri::command]
#[specta::specta]
pub async fn tts_setup(app: tauri::AppHandle) -> Result<(), String> {
    crate::managers::tts::setup(&app).await
}

/// Cierra la sesión y la convierte en documento con el motor local.
/// `converse` → nota limpia de la conversación; `listen` → acta con acuerdos.
#[tauri::command]
#[specta::specta]
pub async fn conversation_finish(app: tauri::AppHandle) -> Result<String, String> {
    LISTENING.store(false, Ordering::Relaxed);
    let text = transcript("Usuario", "Asistente");
    if text.trim().is_empty() {
        return Err("empty".to_string());
    }
    crate::actions::conversation_document(&app, &text, &mode())
        .await
        .ok_or_else(|| "llm_unavailable".to_string())
}
