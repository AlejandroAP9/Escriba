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

/// Manos libres: micrófono abierto y el VAD corta cada intervención sola.
static HANDS_FREE: AtomicBool = AtomicBool::new(false);

/// Audio del sistema: captura lo que suena en el computador (Zoom, Meet, un
/// video) y lo suma a la sesión como turnos de "Otros".
static SYSTEM_AUDIO: AtomicBool = AtomicBool::new(false);

/// Intérprete de reuniones (idea de John Walter): traducir los turnos de
/// "Otros" al idioma del usuario cuando vienen en el idioma de la reunión.
static SYS_TRANSLATE: AtomicBool = AtomicBool::new(false);
static SYS_TRANSLATE_FOREIGN: Mutex<String> = Mutex::new(String::new());

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

/// Documento final + ánimo general de la sesión (idea de Pedro Sánchez,
/// comunidad): Plumín entrega el acta con la carita acorde.
#[derive(Serialize, Clone, Type)]
pub struct SessionDoc {
    pub text: String,
    /// "positivo" | "neutral" | "tenso" (neutral si el modelo no lo marcó).
    pub mood: String,
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
/// El rol "system" son los turnos del audio del sistema (la otra parte de una
/// reunión, un video): entra con su propia etiqueta.
pub fn transcript(user_label: &str, assistant_label: &str, system_label: &str) -> String {
    TURNS
        .lock()
        .map(|t| {
            t.iter()
                .map(|turn| {
                    let who = match turn.role.as_str() {
                        "assistant" => assistant_label,
                        "system" => system_label,
                        _ => user_label,
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
pub fn conversation_stop(app: tauri::AppHandle) -> ConversationStatus {
    use tauri::Manager;
    LISTENING.store(false, Ordering::Relaxed);
    system_audio_off();
    hands_free_off(
        &app.state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>(),
        &app.state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>(),
    );
    status()
}

#[tauri::command]
#[specta::specta]
pub fn conversation_status() -> ConversationStatus {
    status()
}

#[tauri::command]
#[specta::specta]
pub fn conversation_reset(app: tauri::AppHandle) -> ConversationStatus {
    use tauri::Manager;
    LISTENING.store(false, Ordering::Relaxed);
    system_audio_off();
    hands_free_off(
        &app.state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>(),
        &app.state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>(),
    );
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
pub async fn conversation_speak(app: tauri::AppHandle, text: String, engine: String) -> bool {
    speak_native(&app, &text, &engine).await
}

/// Cascada nativa reutilizable (Sesiones y "Tu tinta en voz"): voz incluida →
/// `say` del sistema → false (que el llamador decida su respaldo).
pub async fn speak_native(app: &tauri::AppHandle, text: &str, engine: &str) -> bool {
    // Motor #1: voz neural incluida (salvo que el usuario prefiera la del
    // sistema con el selector de la pantalla).
    let app_lang = crate::settings::get_settings(app).app_language;
    if engine != "system" && app_lang.starts_with("es") && crate::managers::tts::installed(app) {
        let app2 = app.clone();
        let text2 = text.to_string();
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

/// ¿Hay lectura nativa en curso (voz incluida o `say`)?
pub fn is_speaking_native() -> bool {
    if crate::managers::tts::is_playing() {
        return true;
    }
    if let Ok(mut guard) = SPEAKING.lock() {
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => return true,
                _ => {
                    *guard = None;
                }
            }
        }
    }
    false
}

/// Detiene toda lectura nativa en curso.
pub fn stop_speaking_native() {
    crate::managers::tts::stop();
    if let Ok(mut guard) = SPEAKING.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Detiene la lectura en voz alta en curso (cualquier motor).
#[tauri::command]
#[specta::specta]
pub fn conversation_speak_stop() {
    stop_speaking_native();
}

pub fn is_hands_free() -> bool {
    HANDS_FREE.load(Ordering::Relaxed)
}

/// Manos libres para los modos de escucha: abre el micrófono y deja que el
/// VAD corte cada intervención en los silencios; cada segmento se transcribe
/// y entra como turno, sin atajo. (Solo escucha: en Conversar la voz de la
/// respuesta se re-capturaría a sí misma.)
#[tauri::command]
#[specta::specta]
pub fn conversation_hands_free(app: tauri::AppHandle, on: bool) -> Result<bool, String> {
    use crate::managers::audio::AudioRecordingManager;
    use crate::managers::transcription::TranscriptionManager;
    use std::sync::mpsc::RecvTimeoutError;
    use std::sync::Arc;
    use tauri::Manager;

    let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
    let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());

    if !on {
        hands_free_off(&rm, &tm);
        return Ok(false);
    }

    if !is_listening() || is_converse_mode() {
        return Err("hands_free_listen_only".to_string());
    }
    if HANDS_FREE.swap(true, Ordering::Relaxed) {
        return Ok(true); // ya activo
    }

    // Micrófono abierto en modo streaming (VAD con cola post-voz).
    if let Err(e) = rm.try_start_recording("hands_free", crate::audio_toolkit::VadPolicy::Streaming)
    {
        HANDS_FREE.store(false, Ordering::Relaxed);
        return Err(e);
    }
    let rx = tm.stream_router().open_tap();

    // Worker: junta frames de voz; ~0.9 s sin frames nuevos (el VAD calla en
    // silencio) = fin de la intervención → transcribir → turno.
    let app2 = app.clone();
    std::thread::spawn(move || {
        const SILENCE_MS: u128 = 900;
        const MIN_SAMPLES: usize = 16_000 / 2; // 0.5 s: descarta chasquidos
        let mut buffer: Vec<f32> = Vec::new();
        let mut last_voice = std::time::Instant::now();
        loop {
            if !HANDS_FREE.load(Ordering::Relaxed) {
                break;
            }
            match rx.recv_timeout(std::time::Duration::from_millis(250)) {
                Ok(frame) => {
                    buffer.extend_from_slice(&frame);
                    last_voice = std::time::Instant::now();
                }
                Err(RecvTimeoutError::Timeout) => {
                    if buffer.is_empty() || last_voice.elapsed().as_millis() < SILENCE_MS {
                        continue;
                    }
                    if buffer.len() < MIN_SAMPLES {
                        buffer.clear();
                        continue;
                    }
                    let samples = std::mem::take(&mut buffer);
                    let app3 = app2.clone();
                    let tm3 = Arc::clone(&app2.state::<Arc<TranscriptionManager>>());
                    tauri::async_runtime::spawn(async move {
                        let segs = tauri::async_runtime::spawn_blocking(move || {
                            crate::studio::pipeline::transcribe_samples(&tm3, &samples, |_| {})
                        })
                        .await;
                        if let Ok(Ok(segs)) = segs {
                            let text = crate::studio::segments::group_paragraphs(&segs)
                                .join(" ")
                                .trim()
                                .to_string();
                            if !text.is_empty() && is_listening() {
                                let turn = push_turn("user", &text);
                                use tauri::Emitter;
                                let _ = app3.emit("conversation-turn", turn);
                            }
                        }
                    });
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    Ok(true)
}

/// ¿Este equipo puede capturar el audio del sistema? (macOS 13+, Apple Silicon)
#[tauri::command]
#[specta::specta]
pub fn system_audio_supported() -> bool {
    crate::system_audio::supported()
}

/// ¿Está concedido el permiso de Grabación de pantalla? (panel de permisos)
#[tauri::command]
#[specta::specta]
pub fn system_audio_permission() -> bool {
    crate::system_audio::permission_granted()
}

/// Suma a la sesión lo que suena en el computador (la otra parte de una
/// reunión Zoom/Meet, un video). El mismo VAD del micrófono (Silero) separa
/// voz de música/silencio; cada intervención se corta en las pausas (o a los
/// 25 s si nadie pausa, como en un podcast), se transcribe local y entra como
/// turno de "Otros". Solo modos de escucha.
#[tauri::command]
#[specta::specta]
pub fn conversation_system_audio(app: tauri::AppHandle, on: bool) -> Result<bool, String> {
    use crate::audio_toolkit::vad::{
        SileroVad, SmoothedVad, VoiceActivityDetector, VAD_ONSET_FRAMES, VAD_PREFILL_FRAMES,
    };
    use tauri::Manager;

    if !on {
        system_audio_off();
        return Ok(false);
    }

    if !is_listening() || is_converse_mode() {
        return Err("listen_only".to_string());
    }
    if !crate::system_audio::supported() {
        return Err("unsupported".to_string());
    }
    if !crate::system_audio::permission_granted() {
        // Dispara el diálogo del sistema (solo la primera vez); tras conceder
        // el permiso de Grabación de pantalla macOS exige relanzar la app.
        crate::system_audio::request_permission();
        return Err("screen_permission".to_string());
    }

    // VAD propio para este stream (el del micrófono vive en su recorder).
    // Se construye antes de arrancar nada para que un fallo salga por la UI.
    let vad_path = app
        .path()
        .resolve(
            "resources/models/silero_vad_v4.onnx",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|_| "start_failed".to_string())?;
    let silero = SileroVad::new(&vad_path, 0.3).map_err(|_| "start_failed".to_string())?;
    // Cola post-voz corta (15 frames ≈ 450 ms): las pausas de una conversación
    // real cierran el segmento rápido, sin esperar la cola larga del streaming.
    let mut vad = SmoothedVad::new(Box::new(silero), VAD_PREFILL_FRAMES, 15, VAD_ONSET_FRAMES);

    if SYSTEM_AUDIO.swap(true, Ordering::Relaxed) {
        return Ok(true); // ya activo
    }

    if let Err(e) = crate::system_audio::start() {
        SYSTEM_AUDIO.store(false, Ordering::Relaxed);
        return Err(e);
    }

    // Worker: drena el ring buffer del bridge cada ~100 ms y pasa el audio por
    // el VAD en frames de 30 ms. Una intervención termina tras ~0.6 s sin voz
    // (más la cola del VAD ≈ 1 s de pausa real) o al tope de 25 s → turno.
    let app2 = app.clone();
    std::thread::spawn(move || {
        use std::sync::Arc;
        use tauri::Manager;

        const FRAME: usize = 480; // 30 ms @ 16 kHz, el tamaño que espera el VAD
        const SILENCE_MS: u128 = 300; // + cola del VAD (~450 ms) ≈ 0.75 s de pausa real
        const MIN_SAMPLES: usize = 16_000 / 2; // 0.5 s: descarta pitidos sueltos
                                               // Un locutor no pausa: corte duro corto para que el turno aparezca
                                               // rápido (el delay percibido es este tope + lo que tarde el modelo).
        const MAX_SAMPLES: usize = 16_000 * 12;

        let mut chunk = vec![0f32; 16_000];
        let mut pending: Vec<f32> = Vec::new(); // crudo, a la espera del VAD
        let mut buffer: Vec<f32> = Vec::new(); // solo voz (salida del VAD)
        let mut last_voice = std::time::Instant::now();

        loop {
            if !SYSTEM_AUDIO.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            let n = crate::system_audio::read(&mut chunk);
            if n > 0 {
                pending.extend_from_slice(&chunk[..n]);
            }
            // El VAD decide qué es voz; la música y el silencio se descartan.
            let mut offset = 0;
            while offset + FRAME <= pending.len() {
                if let Ok(f) = vad.push_frame(&pending[offset..offset + FRAME]) {
                    if let crate::audio_toolkit::vad::VadFrame::Speech(speech) = f {
                        buffer.extend_from_slice(speech);
                        last_voice = std::time::Instant::now();
                    }
                }
                offset += FRAME;
            }
            pending.drain(..offset);

            if !buffer.is_empty()
                && (last_voice.elapsed().as_millis() >= SILENCE_MS || buffer.len() >= MAX_SAMPLES)
            {
                let samples = std::mem::take(&mut buffer);
                if samples.len() < MIN_SAMPLES {
                    continue;
                }
                let app3 = app2.clone();
                let tm3 = Arc::clone(
                    &app2.state::<Arc<crate::managers::transcription::TranscriptionManager>>(),
                );
                tauri::async_runtime::spawn(async move {
                    let segs = tauri::async_runtime::spawn_blocking(move || {
                        crate::studio::pipeline::transcribe_samples(&tm3, &samples, |_| {})
                    })
                    .await;
                    if let Ok(Ok(segs)) = segs {
                        let mut text = crate::studio::segments::group_paragraphs(&segs)
                            .join(" ")
                            .trim()
                            .to_string();
                        if !text.is_empty() && is_listening() {
                            // Intérprete de reuniones (John Walter): si el
                            // turno viene en el idioma de la reunión, se
                            // traduce al del usuario antes de mostrarse.
                            if SYS_TRANSLATE.load(Ordering::Relaxed) {
                                let foreign = SYS_TRANSLATE_FOREIGN
                                    .lock()
                                    .map(|g| g.clone())
                                    .unwrap_or_default();
                                let mine = crate::settings::get_settings(&app3)
                                    .app_language
                                    .split('-')
                                    .next()
                                    .unwrap_or("es")
                                    .to_string();
                                if let Some(translated) = crate::actions::translate_if_foreign(
                                    &app3, &text, &foreign, &mine,
                                )
                                .await
                                {
                                    text = translated;
                                }
                            }
                            let turn = push_turn("system", &text);
                            use tauri::Emitter;
                            let _ = app3.emit("conversation-turn", turn);
                        }
                    }
                });
            }
        }
    });

    Ok(true)
}

/// Intérprete de reuniones (idea de John Walter, comunidad): con esto activo,
/// los turnos de "Otros" que lleguen en `foreign` se traducen al idioma de la
/// app antes de mostrarse. Requiere el motor local (igual que el Traductor).
#[tauri::command]
#[specta::specta]
pub fn conversation_system_translate(on: bool, foreign: String) -> bool {
    if let Ok(mut g) = SYS_TRANSLATE_FOREIGN.lock() {
        *g = foreign;
    }
    SYS_TRANSLATE.store(on, Ordering::Relaxed);
    on
}

/// Si el Intérprete de reuniones está activo, devuelve el idioma del otro
/// lado. Lo usa el cable de vuelta: tu dictado sale traducido a ese idioma.
pub fn sys_translate_foreign() -> Option<String> {
    if !SYS_TRANSLATE.load(Ordering::Relaxed) {
        return None;
    }
    SYS_TRANSLATE_FOREIGN
        .lock()
        .ok()
        .map(|g| g.clone())
        .filter(|s| !s.is_empty())
}

/// Anexa la traducción del Intérprete al último turno del usuario, para que
/// el acta conserve el par original ⇢ traducción (registro bilingüe).
pub fn append_reply_translation(translated: &str) {
    if let Ok(mut t) = TURNS.lock() {
        if let Some(turn) = t.iter_mut().rev().find(|t| t.role == "user") {
            turn.text = format!("{}\n⇢ {}", turn.text, translated);
        }
    }
}

/// Apaga la captura del audio del sistema (el worker sale en su próximo tick).
fn system_audio_off() {
    if !SYSTEM_AUDIO.swap(false, Ordering::Relaxed) {
        return;
    }
    crate::system_audio::stop();
}

/// Apaga manos libres: cierra el tap y suelta el micrófono (buffer descartado).
fn hands_free_off(
    rm: &crate::managers::audio::AudioRecordingManager,
    tm: &crate::managers::transcription::TranscriptionManager,
) {
    if !HANDS_FREE.swap(false, Ordering::Relaxed) {
        return;
    }
    tm.stream_router().close_tap();
    let gen = rm.cancel_generation();
    let _ = rm.stop_recording("hands_free", gen);
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
pub async fn conversation_finish(app: tauri::AppHandle) -> Result<SessionDoc, String> {
    use tauri::Manager;
    LISTENING.store(false, Ordering::Relaxed);
    system_audio_off();
    hands_free_off(
        &app.state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>(),
        &app.state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>(),
    );
    let text = transcript("Usuario", "Asistente", "Otros");
    if text.trim().is_empty() {
        return Err("empty".to_string());
    }
    let doc = crate::actions::conversation_document(&app, &text, &mode())
        .await
        .ok_or_else(|| "llm_unavailable".to_string())?;

    // Extraer el marcador [[animo:...]] (si el modelo lo emitió) y limpiarlo
    // del texto. Sin marcador: neutral, y el documento queda intacto.
    let mut mood = "neutral".to_string();
    let mut lines: Vec<&str> = doc.lines().collect();
    if let Some(pos) = lines.iter().rposition(|l| {
        let l = l.trim().to_lowercase();
        l.starts_with("[[animo:") || l.starts_with("[[ánimo:")
    }) {
        let marker = lines[pos].trim().to_lowercase();
        if marker.contains("positiv") {
            mood = "positivo".to_string();
        } else if marker.contains("tens") {
            mood = "tenso".to_string();
        }
        lines.remove(pos);
    }
    Ok(SessionDoc {
        text: lines.join("\n").trim().to_string(),
        mood,
    })
}
