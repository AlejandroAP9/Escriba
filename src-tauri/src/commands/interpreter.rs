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

/// Fase 1: publicar una línea de prueba para validar la conectividad end-to-end.
#[tauri::command]
#[specta::specta]
pub fn interpreter_publish_test(text: String) {
    global().publish(text);
}
