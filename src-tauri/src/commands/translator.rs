//! Modo Traductor: conversación 1-a-1 cara a cara en el mismo computador.
//! Eliges 2 idiomas; hablas cualquiera; la IA detecta cuál es y lo traduce
//! al otro, mostrándolo grande + voz. Reutiliza el dictado y el motor local.

use serde::Serialize;
use specta::Type;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static LISTENING: AtomicBool = AtomicBool::new(false);
static LANGS: Mutex<(String, String)> = Mutex::new((String::new(), String::new()));

pub fn is_listening() -> bool {
    LISTENING.load(Ordering::Relaxed)
}

pub fn langs() -> (String, String) {
    LANGS
        .lock()
        .map(|l| l.clone())
        .unwrap_or_else(|_| ("es".to_string(), "en".to_string()))
}

#[derive(Serialize, Clone, Type)]
pub struct TranslatorStatus {
    pub listening: bool,
    pub lang_a: String,
    pub lang_b: String,
}

#[tauri::command]
#[specta::specta]
pub fn translator_set_langs(lang_a: String, lang_b: String) {
    if let Ok(mut l) = LANGS.lock() {
        *l = (lang_a, lang_b);
    }
}

#[tauri::command]
#[specta::specta]
pub fn translator_set_listening(on: bool) {
    LISTENING.store(on, Ordering::Relaxed);
}

#[tauri::command]
#[specta::specta]
pub fn translator_status() -> TranslatorStatus {
    let (a, b) = langs();
    TranslatorStatus {
        listening: is_listening(),
        lang_a: a,
        lang_b: b,
    }
}
