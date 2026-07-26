//! Dictado libre: micrófono abierto y cero atajos. El VAD corta cada frase en
//! los silencios y el texto se escribe donde esté tu cursor, frase por frase.
//! Mismo patrón que manos libres de Sesiones, pero con salida al pipeline
//! normal de dictado (Tonos por app incluidos). Riesgo asumido y visible:
//! escribe TODO lo que se hable, por eso solo se activa a mano y se muestra
//! en el tray (icono grabando) y en el pill de modo activo de la app.

use log::error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Manager;

static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::Relaxed)
}

/// Núcleo reutilizable: lo llaman el comando de la UI y el menú del tray.
pub fn set_active(app: &tauri::AppHandle, on: bool) -> Result<bool, String> {
    use crate::managers::audio::AudioRecordingManager;
    use crate::managers::transcription::TranscriptionManager;
    use std::sync::mpsc::RecvTimeoutError;

    let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
    let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());

    if !on {
        if ACTIVE.swap(false, Ordering::Relaxed) {
            tm.stream_router().close_tap();
            let gen = rm.cancel_generation();
            let _ = rm.stop_recording("free_dictation", gen);
            crate::tray::change_tray_icon(app, crate::tray::TrayIconState::Idle);
            crate::tray::update_tray_menu(app, &crate::tray::TrayIconState::Idle, None);
        }
        return Ok(false);
    }

    if ACTIVE.swap(true, Ordering::Relaxed) {
        return Ok(true); // ya activo
    }
    if let Err(e) = rm.try_start_recording(
        "free_dictation",
        crate::audio_toolkit::VadPolicy::Conversational,
    ) {
        ACTIVE.store(false, Ordering::Relaxed);
        return Err(e);
    }
    let rx = tm.stream_router().open_tap();
    // El mic queda abierto: el tray lo dice con el icono de grabación.
    crate::tray::change_tray_icon(app, crate::tray::TrayIconState::Recording);
    crate::tray::update_tray_menu(app, &crate::tray::TrayIconState::Recording, None);

    let app2 = app.clone();
    std::thread::spawn(move || {
        use crate::commands::conversation::{
            emit_turn_phase, is_speaking_native, stop_speaking_native, MIN_SAMPLES, POLL_INTERVAL,
            SILENCE_MS,
        };
        let mut buffer: Vec<f32> = Vec::new();
        let mut last_voice = std::time::Instant::now();
        let mut closing_announced = false;
        loop {
            if !ACTIVE.load(Ordering::Relaxed) {
                break;
            }
            match rx.recv_timeout(POLL_INTERVAL) {
                Ok(frame) => {
                    // Barge-in: hablar corta la lectura en curso, igual que en
                    // manos libres. Y lo capturado mientras la app hablaba se
                    // descarta, porque puede ser eco de la propia locución.
                    if is_speaking_native() {
                        log::debug!("barge-in (dictado libre): se corta la voz");
                        stop_speaking_native();
                        buffer.clear();
                    }
                    buffer.extend_from_slice(&frame);
                    last_voice = std::time::Instant::now();
                    if closing_announced {
                        closing_announced = false;
                        emit_turn_phase(&app2, "listening");
                    }
                }
                Err(RecvTimeoutError::Timeout) => {
                    if buffer.is_empty() {
                        continue;
                    }
                    if !closing_announced {
                        closing_announced = true;
                        emit_turn_phase(&app2, "closing");
                    }
                    if last_voice.elapsed().as_millis() < SILENCE_MS {
                        continue;
                    }
                    closing_announced = false;
                    if buffer.len() < MIN_SAMPLES {
                        buffer.clear();
                        emit_turn_phase(&app2, "listening");
                        continue;
                    }
                    emit_turn_phase(&app2, "transcribing");
                    let samples = std::mem::take(&mut buffer);
                    let app3 = app2.clone();
                    let tm3 = Arc::clone(&app2.state::<Arc<TranscriptionManager>>());
                    tauri::async_runtime::spawn(async move {
                        let segs = tauri::async_runtime::spawn_blocking(move || {
                            crate::studio::pipeline::transcribe_samples(&tm3, &samples, |_| {})
                        })
                        .await;
                        // Vuelve a "escuchando" pase lo que pase, para que un
                        // fallo no deje el indicador clavado en "transcribiendo".
                        emit_turn_phase(&app3, "listening");
                        if let Ok(Ok(segs)) = segs {
                            let text = crate::studio::segments::group_paragraphs(&segs)
                                .join(" ")
                                .trim()
                                .to_string();
                            if text.is_empty() || !ACTIVE.load(Ordering::Relaxed) {
                                return;
                            }
                            // Pipeline normal: interceptores (Sesiones, Traductor)
                            // y Tonos por app deciden igual que con el atajo.
                            let processed = crate::actions::process_transcription_output(
                                &app3,
                                &text,
                                crate::actions::TranscribeMode::Standard,
                            )
                            .await;
                            if processed.interpreter_published || processed.final_text.is_empty() {
                                return;
                            }
                            let final_text = processed.final_text;
                            let app4 = app3.clone();
                            let _ = app3.run_on_main_thread(move || {
                                if let Err(e) = crate::clipboard::paste(final_text, app4.clone()) {
                                    error!("Dictado libre: fallo al pegar: {}", e);
                                    use tauri::Emitter;
                                    let _ = app4.emit("paste-error", ());
                                }
                            });
                        }
                    });
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(true)
}

/// Enciende o apaga el Dictado libre desde la UI.
#[tauri::command]
#[specta::specta]
pub fn free_dictation_set(app: tauri::AppHandle, on: bool) -> Result<bool, String> {
    set_active(&app, on)
}

/// ¿Está activo? (para el pill de modo activo y el estado del tray)
#[tauri::command]
#[specta::specta]
pub fn free_dictation_status() -> bool {
    is_active()
}
