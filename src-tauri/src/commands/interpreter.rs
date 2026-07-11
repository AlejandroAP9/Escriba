//! Comandos del Intérprete en vivo (guía).

use crate::managers::interpreter::{global, RoomInfo};
use serde::Serialize;
use specta::Type;

#[derive(Serialize, Clone, Type)]
pub struct InterpreterRoom {
    pub code: String,
    pub url: String,
    pub qr_svg: String,
    pub port: u16,
}

impl From<RoomInfo> for InterpreterRoom {
    fn from(r: RoomInfo) -> Self {
        Self {
            code: r.code,
            url: r.url,
            qr_svg: r.qr_svg,
            port: r.port,
        }
    }
}

#[derive(Serialize, Clone, Type)]
pub struct InterpreterStatus {
    pub running: bool,
    pub listeners: u32,
    pub active_languages: Vec<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn interpreter_start() -> Result<InterpreterRoom, String> {
    global().start().await.map(Into::into)
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_stop() {
    global().stop();
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_status() -> InterpreterStatus {
    let s = global();
    InterpreterStatus {
        running: s.is_running(),
        listeners: s.listeners(),
        active_languages: s.active_languages(),
    }
}

/// Publica una línea del guía: traduce a cada idioma de oyente activo con el
/// motor local y la emite. En Fase 1/2 el texto viene del input de prueba o
/// del micrófono; el idioma de origen se asume el de la app.
#[tauri::command]
#[specta::specta]
pub async fn interpreter_publish(app: tauri::AppHandle, text: String, source_lang: String) {
    publish_translated(&app, text, &source_lang).await;
}

/// Traduce `text` a cada idioma activo (una vez por idioma) y publica la línea.
pub async fn publish_translated(app: &tauri::AppHandle, text: String, source_lang: &str) {
    use std::collections::HashMap;
    let server = global();
    let mut translations: HashMap<String, String> = HashMap::new();
    for lang in server.active_languages() {
        if lang == source_lang {
            translations.insert(lang, text.clone());
            continue;
        }
        if let Some(t) = crate::actions::translate_text(app, &text, &lang).await {
            translations.insert(lang, t);
        }
    }
    server.publish_line(text, translations);
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_set_source_lang(lang: String) {
    global().set_source_lang(lang);
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_set_listening(on: bool) {
    global().set_listening(on);
}
