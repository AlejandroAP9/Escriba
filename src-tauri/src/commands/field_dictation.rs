//! Dictado en un campo de la propia app (botón de micrófono). Reusa el pipeline
//! de dictado global (coordinador + modelo), pero en vez de pegar en la app
//! enfocada, emite el texto como evento para que el campo que lo pidió lo
//! inserte. Una sola grabación a la vez (la impone el coordinador).

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

static CAPTURING: AtomicBool = AtomicBool::new(false);

/// Consume la marca: devuelve si el próximo texto va a un campo y la limpia.
/// Se llama una sola vez por transcripción, así nunca persiste de más.
pub fn take_capturing() -> bool {
    CAPTURING.swap(false, Ordering::Relaxed)
}

/// Limpia la marca sin emitir (para paths que no llegan a procesar texto, p.ej.
/// grabación sin audio).
pub fn clear_capturing() {
    CAPTURING.store(false, Ordering::Relaxed);
}

/// Emite el texto dictado al campo que lo pidió.
pub fn emit_result(app: &AppHandle, text: &str) {
    let _ = app.emit("field-dictation-result", text.to_string());
}

/// Alterna la grabación para un campo. El primer llamado arranca; el segundo
/// para y procesa (el coordinador trabaja en modo toggle). Marca el desvío a
/// campo para que el texto no se pegue en la app enfocada.
#[tauri::command]
#[specta::specta]
pub fn field_dictation_toggle(app: AppHandle) {
    CAPTURING.store(true, Ordering::Relaxed);
    crate::signal_handle::send_transcription_input(&app, "transcribe", "field");
}
