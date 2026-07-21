//! "Revisar antes de pegar" (benchmark 17-jul + el "game over" de John Walter
//! en la comunidad: dictar sin que corregir sea un quilombo). Opcional y
//! apagado por defecto: el flujo rápido de Escriba no cambia. Encendido, el
//! dictado normal se muestra en el overlay antes de tocar tu documento:
//! Pegar, Descartar, o dictar una corrección con el mismo atajo (reusa la
//! maquinaria de edición por voz sobre el texto pendiente).

use std::sync::Mutex;
use tauri::Manager;

static PENDING: Mutex<Option<String>> = Mutex::new(None);

/// Texto esperando revisión (None si no hay revisión activa).
pub fn pending_text() -> Option<String> {
    PENDING.lock().ok().and_then(|g| g.clone())
}

/// Deja `text` pendiente y muestra el panel de revisión en el overlay.
pub fn set_pending(app: &tauri::AppHandle, text: String) {
    if let Ok(mut g) = PENDING.lock() {
        *g = Some(text.clone());
    }
    crate::overlay::show_review_overlay(app, &text);
}

/// Limpia el texto pendiente de revisión. Público para que la cancelación
/// global (atajo de cancelar) lo borre: si no, el próximo dictado se trataría
/// como una corrección del texto fantasma (auditoría #3).
pub fn clear_pending() {
    if let Ok(mut g) = PENDING.lock() {
        *g = None;
    }
}

fn clear() {
    clear_pending();
}

/// Pegar el texto pendiente donde está el cursor y cerrar la revisión.
/// El overlay es un panel sin foco (NSPanel/focusable=false), así que la app
/// destino sigue siendo la del usuario; el pequeño delay da margen igual.
#[tauri::command]
#[specta::specta]
pub fn review_confirm(app: tauri::AppHandle) {
    let Some(text) = PENDING.lock().ok().and_then(|mut g| g.take()) else {
        return;
    };
    crate::overlay::hide_recording_overlay(&app);
    // El respiro de 150 ms (para que el overlay se oculte antes de pegar) va en
    // un hilo aparte: dormirlo en el hilo principal congelaba la app en cada
    // pegado (auditoría #16). Solo el pegado vuelve al hilo principal.
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        let app3 = app2.clone();
        let _ = app2.run_on_main_thread(move || {
            if let Err(e) = crate::clipboard::paste(text, app3.clone()) {
                log::error!("Revisión: fallo al pegar: {}", e);
                use tauri::Emitter;
                let _ = app3.emit("paste-error", ());
            }
        });
    });
}

/// Descartar el texto pendiente sin pegar nada.
#[tauri::command]
#[specta::specta]
pub fn review_discard(app: tauri::AppHandle) {
    clear();
    crate::overlay::hide_recording_overlay(&app);
}
