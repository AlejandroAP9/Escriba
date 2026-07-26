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

/// Copia el texto pendiente al portapapeles y cierra la revisión.
///
/// La auditoría pedía poder EDITAR aquí, pero el overlay es un panel no
/// enfocable a propósito (así la app de destino conserva el foco y el pegado
/// llega a donde el usuario estaba escribiendo). Un campo de texto ahí dentro
/// no podría recibir teclado, y hacer el panel enfocable rompería justo lo que
/// hace útil a la feature.
///
/// Copiar es la salida que sí respeta ese diseño: si el motor entendió mal, el
/// usuario se lleva el texto y lo arregla en su propio editor, sin tener que
/// repetir la frase entera.
#[tauri::command]
#[specta::specta]
pub fn review_copy(app: tauri::AppHandle) {
    let Some(text) = PENDING.lock().ok().and_then(|mut g| g.take()) else {
        return;
    };
    use tauri_plugin_clipboard_manager::ClipboardExt;
    if let Err(e) = app.clipboard().write_text(text) {
        log::error!("Revisión: fallo al copiar: {}", e);
    }
    crate::overlay::hide_recording_overlay(&app);
}

/// Qué hacer con un texto escrito a teclado.
///
/// Solo dos de los cuatro [`crate::actions::TranscribeMode`] tienen sentido por
/// aquí. `Edit` queda fuera a propósito: depende de la selección que captura el
/// Cmd/Ctrl+C sintético al empezar una edición por voz, así que desde un panel
/// de texto siempre encontraría el buffer vacío y caería en "no_selection".
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum TypedTextAction {
    /// Aplica el tono/plantilla configurada, igual que un dictado normal.
    Correct,
    /// Traduce al idioma de `translation_target_language`.
    Translate,
}

/// Alternativa por TECLADO al dictado: procesa un texto escrito con el mismo
/// motor de IA y devuelve el resultado.
///
/// Toda la corrección, traducción y tonos de Escriba pasaban por hablar. Alguien
/// que en ese momento no pueda usar la voz (una oficina compartida, afonía, una
/// discapacidad del habla) no tenía forma de usar el motor local que la app ya
/// tiene instalado, aunque la infraestructura estuviera entera.
///
/// Devuelve el texto en pantalla en vez de pegarlo: cuando la ventana principal
/// tiene el foco, la aplicación de destino ya lo perdió, así que pegar iría al
/// sitio equivocado. El usuario copia el resultado y lo lleva a donde quiera.
#[tauri::command]
#[specta::specta]
pub async fn process_typed_text(
    app: tauri::AppHandle,
    text: String,
    action: TypedTextAction,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("EMPTY".to_string());
    }
    let settings = crate::settings::get_settings(&app);
    if !settings.post_process_enabled {
        return Err("POST_PROCESS_DISABLED".to_string());
    }
    // `Correct` mapea a `Standard` y no a `PostProcess` porque es el modo que
    // ya tenía este camino: los dos recorren la misma rama en
    // `post_process_transcription`, y cambiarlo sería mover comportamiento sin
    // motivo.
    let mode = match action {
        TypedTextAction::Correct => crate::actions::TranscribeMode::Standard,
        TypedTextAction::Translate => crate::actions::TranscribeMode::Translate,
    };
    let processed = crate::actions::post_process_public(&app, &settings, &text, mode).await;
    // Sin post-proceso disponible (motor no instalado, proveedor caído) se
    // devuelve el original: el usuario nunca se queda sin su texto, igual que
    // en el camino de voz.
    Ok(processed.unwrap_or(text))
}
