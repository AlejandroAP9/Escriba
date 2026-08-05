//! Modo Traductor: conversación 1-a-1 cara a cara en el mismo computador.
//! Eliges 2 idiomas; hablas cualquiera; la IA detecta cuál es y lo traduce
//! al otro, mostrándolo grande + voz. Reutiliza el dictado y el motor local.

use serde::Serialize;
use specta::Type;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static LISTENING: AtomicBool = AtomicBool::new(false);
static LANGS: Mutex<(String, String)> = Mutex::new((String::new(), String::new()));

/// Últimos turnos (original → traducción) de la sesión EN CURSO, solo en RAM
/// (PRP-006, Fase 6). Dan contexto a la traducción siguiente: sin ellos,
/// "mañana tengo una prueba" en una conversación escolar salía como *proof*.
/// Tope de turnos y de tamaño: el contexto informa, no domina.
const MAX_TURNOS: usize = 3;
const MAX_LARGO_TURNO: usize = 300;
static TURNOS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn recortar(s: &str) -> String {
    let limpio = s.trim();
    if limpio.chars().count() <= MAX_LARGO_TURNO {
        return limpio.to_string();
    }
    limpio.chars().take(MAX_LARGO_TURNO).collect()
}

/// Registra un turno traducido (para el contexto del siguiente).
pub fn push_turno(original: &str, traduccion: &str) {
    if let Ok(mut t) = TURNOS.lock() {
        t.push((recortar(original), recortar(traduccion)));
        let sobra = t.len().saturating_sub(MAX_TURNOS);
        if sobra > 0 {
            t.drain(..sobra);
        }
    }
}

/// Bloque de contexto listo para el prompt, o None sin turnos previos.
pub fn contexto() -> Option<String> {
    let t = TURNOS.lock().ok()?;
    if t.is_empty() {
        return None;
    }
    Some(
        t.iter()
            .map(|(o, tr)| format!("- {} => {}", o, tr))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn limpiar_turnos() {
    if let Ok(mut t) = TURNOS.lock() {
        t.clear();
    }
}

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
    // Otro par de idiomas = otra conversación: el contexto viejo confundiría.
    limpiar_turnos();
}

#[tauri::command]
#[specta::specta]
pub fn translator_set_listening(on: bool) {
    LISTENING.store(on, Ordering::Relaxed);
    if !on {
        // El contexto vive solo mientras la sesión escucha (RAM, jamás disco).
        limpiar_turnos();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Un solo test a propósito: TURNOS es un static global y cargo corre los
    /// tests en hilos paralelos; tres tests mutándolo serían una carrera.
    #[test]
    fn ring_de_turnos_tope_recorte_y_limpieza() {
        limpiar_turnos();
        assert!(contexto().is_none());

        // Tope 3: el primero sale del ring.
        push_turno("hola", "hello");
        push_turno("mañana tengo una prueba", "tomorrow I have a test");
        push_turno("es de matemáticas", "it's about math");
        push_turno("me fue bien", "it went well");
        let ctx = contexto().expect("hay turnos");
        assert!(!ctx.contains("hola"));
        assert!(ctx.contains("prueba"));
        assert!(ctx.contains("me fue bien"));

        // Apagar la escucha limpia el contexto (vive solo en la sesión).
        translator_set_listening(false);
        assert!(contexto().is_none());

        // Cambiar el par de idiomas también limpia.
        push_turno("bonjour", "hola");
        translator_set_langs("es".into(), "pt".into());
        assert!(contexto().is_none());

        // Los turnos larguísimos se recortan para acotar el prompt.
        let largo = "palabra ".repeat(200);
        push_turno(&largo, &largo);
        let ctx = contexto().expect("hay turno");
        assert!(ctx.chars().count() < 700);
        limpiar_turnos();
    }
}
