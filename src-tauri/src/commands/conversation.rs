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

/// Cada cuánto despierta el worker a comprobar si el turno ya cerró.
///
/// Antes eran 250 ms, que se sumaban enteros al retardo del fin de turno en el
/// peor caso. 50 ms es imperceptible en costo (el worker solo mira un canal) y
/// recorta hasta 200 ms de la espera.
pub(crate) const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

/// Silencio que cierra un turno en los modos conversacionales.
pub(crate) const SILENCE_MS: u128 = 900;

/// Mínimo de audio para considerar que hubo una intervención (0,5 s a 16 kHz).
/// Por debajo de esto son chasquidos, y los modelos tipo Whisper alucinan
/// frases enteras sobre fragmentos así.
pub(crate) const MIN_SAMPLES: usize = 16_000 / 2;

/// Avisa a la interfaz de en qué fase va el turno conversacional.
///
/// Durante la cuenta de silencio la interfaz seguía diciendo "escuchando", así
/// que el usuario no distinguía entre "te sigo oyendo" y "ya terminé, estoy
/// esperando", y tendía a repetirse o a hablar encima.
pub(crate) fn emit_turn_phase(app: &tauri::AppHandle, phase: &str) {
    use tauri::Emitter;
    let _ = app.emit("conversation-turn-phase", phase.to_string());
}

/// Proceso `say` en curso (macOS): se mata antes de hablar de nuevo o al parar.
static SPEAKING: Mutex<Option<std::process::Child>> = Mutex::new(None);

/// Manos libres: micrófono abierto y el VAD corta cada intervención sola.
static HANDS_FREE: AtomicBool = AtomicBool::new(false);

/// Audio del sistema: captura lo que suena en el computador (Zoom, Meet, un
/// video) y lo suma a la sesión como turnos de "Otros".
static SYSTEM_AUDIO: AtomicBool = AtomicBool::new(false);
/// El worker del audio del sistema está vivo. Al apagar, el flag SYSTEM_AUDIO
/// se pone en false pero el worker tarda hasta ~100 ms en salir (duerme por
/// iteración); re-encender en esa ventana arrancaba un segundo worker con el
/// primero aún vivo, y los dos se repartían el buffer (auditoría #4). Con esto
/// el arranque espera a que el anterior termine.
static SYSTEM_AUDIO_WORKER: AtomicBool = AtomicBool::new(false);

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
    /// Los tres modos vivían SOLO como estáticos en el backend y como
    /// `useState(false)` en la vista, sin nada que los reconciliara: este struct
    /// se diseñó para los turnos y nunca se le pidió responder por los modos.
    ///
    /// El síntoma era de privacidad: salías de Sesiones con el audio del sistema
    /// encendido, volvías, y el interruptor aparecía apagado mientras el backend
    /// seguía capturando. Y `conversation_stop` los apaga los tres pero devolvía
    /// un estado que no los mencionaba, así que la interfaz tampoco se enteraba.
    pub hands_free: bool,
    pub system_audio: bool,
    pub sys_translate: bool,
}

pub fn is_listening() -> bool {
    LISTENING.load(Ordering::Relaxed)
}

/// Hay una sesión capturando el audio del computador ahora mismo.
///
/// Lo consulta el camino del dictado para no silenciar ni pausar lo que esa
/// sesión está grabando (`audio.rs::apply_mute`, `actions.rs` con
/// `pause_media_on_dictate`).
pub fn system_audio_active() -> bool {
    SYSTEM_AUDIO.load(Ordering::Relaxed)
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

/// Constructor único del estado. Lo usan `conversation_start`, `_status`,
/// `_stop` y `_reset`, así que añadir un campo aquí lo cubre en los cuatro
/// caminos de retorno, incluido el de `conversation_stop`, que era la segunda
/// vía por la que la interfaz se quedaba desincronizada.
fn status() -> ConversationStatus {
    ConversationStatus {
        listening: is_listening(),
        mode: mode(),
        turns: TURNS.lock().map(|t| t.clone()).unwrap_or_default(),
        // `is_hands_free()` existía desde hacía tiempo y el compilador avisaba de
        // que nadie la llamaba: el accesor correcto estaba escrito y sin conectar.
        hands_free: is_hands_free(),
        system_audio: SYSTEM_AUDIO.load(Ordering::Relaxed),
        sys_translate: SYS_TRANSLATE.load(Ordering::Relaxed),
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
    // La voz del Intérprete es un sink de rodio, no un proceso hijo, así que
    // hay que preguntarle aparte. Faltaba, y era precisamente el camino donde
    // el barge-in y la guarda de eco tenían que funcionar: el Intérprete habla
    // por los altavoces con el micrófono abierto en manos libres.
    if crate::audio_feedback::is_interpreter_playing() {
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
    // Los TRES motores, o el barge-in corta unos y deja sonando el otro.
    crate::audio_feedback::stop_interpreter_playback();
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
    // Corta también la voz del Intérprete que esté sonando hacia la llamada
    // (auditoría #18): Pausar/Descartar la frenan de inmediato.
    crate::audio_feedback::stop_interpreter_playback();
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
    if let Err(e) = rm.try_start_recording(
        "hands_free",
        crate::audio_toolkit::VadPolicy::Conversational,
    ) {
        HANDS_FREE.store(false, Ordering::Relaxed);
        return Err(e);
    }
    let rx = tm.stream_router().open_tap();

    // Worker: junta frames de voz; ~0.9 s sin frames nuevos (el VAD calla en
    // silencio) = fin de la intervención → transcribir → turno.
    let app2 = app.clone();
    std::thread::spawn(move || {
        let mut buffer: Vec<f32> = Vec::new();
        let mut last_voice = std::time::Instant::now();
        // Para no repetir el aviso de "cerrando turno" en cada vuelta del bucle.
        let mut closing_announced = false;
        loop {
            if !HANDS_FREE.load(Ordering::Relaxed) {
                break;
            }
            match rx.recv_timeout(POLL_INTERVAL) {
                Ok(frame) => {
                    // BARGE-IN: si la app está hablando y el usuario arranca,
                    // se corta la voz. Antes no había ninguna ruta que hiciera
                    // esto, así que hablar encima no servía de nada: había que
                    // esperar a que la locución terminara sola.
                    if is_speaking_native() {
                        log::debug!("barge-in: el usuario habló, se corta la voz");
                        stop_speaking_native();
                        // Lo capturado mientras hablaba la app puede ser eco de
                        // la propia locución, así que se descarta y el turno
                        // arranca limpio desde esta trama.
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
                    // Avisar en cuanto empieza la cuenta de silencio: durante la
                    // espera la interfaz decía "escuchando", así que el usuario
                    // no distinguía "te sigo oyendo" de "ya terminé, espero".
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
                        if let Ok(Ok(segs)) = segs {
                            let text = crate::studio::segments::group_paragraphs(&segs)
                                .join(" ")
                                .trim()
                                .to_string();
                            if !text.is_empty() && is_listening() {
                                let turn = push_turn("user", &text);
                                use tauri::Emitter;
                                let _ = app3.emit("conversation-turn", turn);
                                // Manos libres también pasa por el Intérprete:
                                // tu voz sale traducida sin tocar una tecla.
                                crate::actions::interpreter_reply_flow(&app3, text);
                            }
                        }
                        emit_turn_phase(&app3, "listening");
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

    // Espera a que un worker anterior (recién apagado) termine de salir antes
    // de arrancar el nuevo, para que no queden dos leyendo el mismo buffer
    // (auditoría #4). Tope generoso (~1 s) por si el modelo estaba ocupado.
    let mut waited = 0;
    while SYSTEM_AUDIO_WORKER.load(Ordering::Relaxed) && waited < 100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        waited += 1;
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

        // Marca el worker como vivo mientras corre; el guard lo desmarca al
        // salir por cualquier vía (auditoría #4), para que el próximo arranque
        // sepa cuándo el buffer ya está libre.
        struct AliveGuard;
        impl Drop for AliveGuard {
            fn drop(&mut self) {
                SYSTEM_AUDIO_WORKER.store(false, Ordering::Relaxed);
            }
        }
        SYSTEM_AUDIO_WORKER.store(true, Ordering::Relaxed);
        let _alive = AliveGuard;

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

        let mut alive_checks: u32 = 0;
        loop {
            if !SYSTEM_AUDIO.load(Ordering::Relaxed) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            // Cada ~1 s comprobamos que la captura siga viva: si el stream se
            // cayó solo (permiso revocado, coreaudiod reiniciado), avisamos al
            // frontend para que apague el toggle en vez de fingir que escucha
            // (auditoría #10). Damos margen inicial para que arranque.
            alive_checks += 1;
            if alive_checks >= 10 && !crate::system_audio::alive() {
                use tauri::Emitter;
                let _ = app2.emit("system-audio-died", ());
                SYSTEM_AUDIO.store(false, Ordering::Relaxed);
                break;
            }
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
pub fn append_reply_translation(original: &str, translated: &str) {
    if let Ok(mut t) = TURNS.lock() {
        // Engancha al turno cuyo texto coincide con el original y aún no tiene
        // traducción: en manos libres, dos frases pueden traducirse fuera de
        // orden, así que no basta con "el último turno". Del más reciente al
        // más viejo para acertar la instancia correcta si el texto se repite.
        let target = original.trim();
        if let Some(turn) = t
            .iter_mut()
            .rev()
            .find(|t| t.role == "user" && t.text.trim() == target)
        {
            turn.text = format!("{}\n⇢ {}", turn.text, translated);
        } else if let Some(turn) = t.iter_mut().rev().find(|t| t.role == "user") {
            // Respaldo: si no calza (post-proceso cambió el texto), al último.
            turn.text = format!("{}\n⇢ {}", turn.text, translated);
        }
    }
}

/// Voz del Intérprete por un dispositivo de salida concreto (el remate de la
/// idea de John Walter): renderiza el texto con la mejor voz instalada del
/// idioma y lo reproduce en ese dispositivo. Con un micrófono virtual
/// (BlackHole) elegido aquí y como micrófono de la reunión, la otra persona
/// escucha tu dictado ya traducido. Solo macOS (como el audio del sistema).
#[tauri::command]
#[specta::specta]
pub async fn conversation_speak_via(
    app: tauri::AppHandle,
    text: String,
    lang: String,
    device: String,
    gender: String,
) -> bool {
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            speak_via_blocking(&app, &text, &lang, &device, &gender)
        })
        .await
        .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, text, lang, device, gender);
        false
    }
}

/// La mejor voz de `say` instalada para un idioma: Premium > Enhanced > resto.
/// (`say -v ?` lista "Nombre (Premium)  es_ES  # frase"; los nombres pueden
/// llevar espacios, así que el locale se busca desde el final.)
/// Nombres de voces de `say` por género (es/en y comunes). Un nombre fuera
/// de las listas no suma ni resta: la calidad sigue mandando.
#[cfg(target_os = "macos")]
const FEMALE_VOICES: [&str; 22] = [
    "Samantha",
    "Ava",
    "Allison",
    "Susan",
    "Zoe",
    "Karen",
    "Moira",
    "Tessa",
    "Fiona",
    "Kate",
    "Serena",
    "Nicky",
    "Vicki",
    "Victoria",
    "Anna",
    "Mónica",
    "Monica",
    "Francisca",
    "Paulina",
    "Angélica",
    "Luciana",
    "Amélie",
];
#[cfg(target_os = "macos")]
const MALE_VOICES: [&str; 20] = [
    "Alex", "Daniel", "Fred", "Tom", "Oliver", "Aaron", "Arthur", "Evan", "Nathan", "Reed", "Lee",
    "Gordon", "Rishi", "Jorge", "Diego", "Juan", "Carlos", "Eddy", "Thomas", "Xander",
];

#[cfg(target_os = "macos")]
fn best_voice_for(lang: &str, gender: &str) -> Option<String> {
    use std::process::Command;
    let out = Command::new("/usr/bin/say")
        .args(["-v", "?"])
        .output()
        .ok()?;
    let listing = String::from_utf8_lossy(&out.stdout).to_string();
    let base = lang.split(['-', '_']).next().unwrap_or(lang).to_lowercase();
    let mut best: Option<(i32, String)> = None;
    for line in listing.lines() {
        let head = match line.find('#') {
            Some(h) => line[..h].trim_end(),
            None => line.trim_end(),
        };
        let Some(loc_start) = head.rfind(char::is_whitespace) else {
            continue;
        };
        let locale = head[loc_start..].trim().to_lowercase().replace('_', "-");
        let name = head[..loc_start].trim_end();
        if name.is_empty() || !locale.starts_with(&base) {
            continue;
        }
        // Entre voces sin calidad marcada, las clásicas de dictado le ganan
        // a las de novedad (Albert, Bells, Boing...): suenan a persona.
        const KNOWN_GOOD: [&str; 12] = [
            "Samantha", "Alex", "Ava", "Allison", "Susan", "Zoe", "Evan", "Nathan", "Daniel",
            "Karen", "Moira", "Tessa",
        ];
        let mut score: i32 = if name.contains("Premium") {
            30
        } else if name.contains("Enhanced") || name.contains("Mejorada") {
            20
        } else if KNOWN_GOOD.iter().any(|g| name == *g) {
            15
        } else {
            10
        };
        // La preferencia de género pesa más que la calidad; la voz del
        // género opuesto queda al final pero disponible como último recurso.
        let base_name = name.split(" (").next().unwrap_or(name);
        let is_female = FEMALE_VOICES.iter().any(|f| base_name == *f);
        let is_male = MALE_VOICES.iter().any(|m| base_name == *m);
        match gender {
            "f" if is_female => score += 100,
            "f" if is_male => score -= 100,
            "m" if is_male => score += 100,
            "m" if is_female => score -= 100,
            _ => {}
        }
        if best.as_ref().map(|(s, _)| score > *s).unwrap_or(true) {
            best = Some((score, name.to_string()));
        }
    }
    best.map(|(_, n)| n)
}

/// Renderiza la traducción a WAV y la reproduce en el dispositivo elegido
/// (cadena vacía = salida por defecto). Bloqueante: llamar en spawn_blocking.
/// Preferencia de motor: voz neural incluida (sherpa-onnx, si está instalada
/// para el idioma y se pidió voz femenina —la neural es femenina—); si no,
/// la voz del sistema (`say`), que honra el selector de género.
#[cfg(target_os = "macos")]
fn speak_via_blocking(
    app: &tauri::AppHandle,
    text: &str,
    lang: &str,
    device: &str,
    gender: &str,
) -> bool {
    use std::process::{Command, Stdio};
    // Archivo temporal ÚNICO por locución (auditoría #7): en manos libres dos
    // frases seguidas ya no se pisan el WAV mientras una se reproduce.
    //
    // Va dentro de un directorio temporal propio en vez de directamente en
    // /tmp: el nombre anterior (`escriba-interpreter-<pid>-<n>.wav`) era
    // adivinable, así que otro proceso podía dejar un symlink ahí y desviar la
    // escritura de `say`. El directorio se crea con nombre aleatorio y permisos
    // 0700, y se borra entero al salir de la función.
    let Ok(tmp_dir) = tempfile::Builder::new()
        .prefix("escriba-interpreter-")
        .tempdir()
    else {
        log::warn!("interpreter voice: no se pudo crear el directorio temporal");
        return false;
    };
    static UTTERANCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = UTTERANCE.fetch_add(1, Ordering::Relaxed);
    let wav = tmp_dir.path().join(format!("{}.wav", n));
    let selected = if device.is_empty() {
        None
    } else {
        Some(device.to_string())
    };

    // Motor #1: voz neural incluida (natural, "todo incorporado"). Solo para
    // voz femenina, porque la voz Piper incluida es femenina.
    if gender != "m" && crate::managers::tts::installed_lang(app, lang) {
        let _ = std::fs::remove_file(&wav);
        if crate::managers::tts::synth_to_wav(app, text, lang, &wav).is_ok() {
            log::info!("interpreter voice: neural incluida ({})", lang);
            let ok =
                crate::audio_feedback::play_interpreter_voice(&wav, selected.clone(), 1.0).is_ok();
            let _ = std::fs::remove_file(&wav);
            if ok {
                return true;
            }
        }
    }

    // Motor #2: la voz del sistema (`say`), con el selector de género.
    use std::io::Write;
    let mut cmd = Command::new("/usr/bin/say");
    if let Some(voice) = best_voice_for(lang, gender) {
        log::info!("interpreter voice: usando '{}'", voice);
        cmd.args(["-v", &voice]);
    }
    // El texto entra por stdin (sin líos de comillas); sale como WAV crudo.
    cmd.arg("-o")
        .arg(&wav)
        .args(["--file-format=WAVE", "--data-format=LEI16@22050"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let Ok(mut child) = cmd.spawn() else {
        log::warn!("interpreter voice: say no arrancó");
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
    }
    if !child.wait().map(|s| s.success()).unwrap_or(false) {
        log::warn!("interpreter voice: say falló al renderizar");
        return false;
    }
    log::info!(
        "interpreter voice: {} chars ({}) → dispositivo {:?}",
        text.chars().count(),
        lang,
        selected.as_deref().unwrap_or("default")
    );
    let result = crate::audio_feedback::play_interpreter_voice(&wav, selected, 1.0);
    if let Err(e) = &result {
        log::warn!("interpreter voice: reproducción falló: {}", e);
    } else {
        log::info!("interpreter voice: reproducción completa");
    }
    let _ = std::fs::remove_file(&wav);
    result.is_ok()
}

/// Apaga la captura del audio del sistema (el worker sale en su próximo tick).
/// Apaga TAMBIÉN el Intérprete de reuniones: traducir solo tiene sentido con
/// el audio del sistema activo, y dejarlo armado entre sesiones hacía que un
/// dictado posterior se tradujera y se leyera solo (auditoría #1).
fn system_audio_off() {
    // El Intérprete se desarma siempre, aunque el audio del sistema ya
    // estuviera apagado (p. ej. al descartar o crear el documento).
    SYS_TRANSLATE.store(false, Ordering::Relaxed);
    if let Ok(mut g) = SYS_TRANSLATE_FOREIGN.lock() {
        g.clear();
    }
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

/// ¿Está lista la voz neural incluida para este idioma? (Intérprete de
/// reuniones: inglés y español tienen voz; otros idiomas usan la del sistema.)
#[tauri::command]
#[specta::specta]
pub fn interpreter_voice_status(app: tauri::AppHandle, lang: String) -> bool {
    crate::managers::tts::installed_lang(&app, &lang)
}

/// Descarga la voz neural incluida para el idioma del Intérprete (runtime
/// compartido + voz, SHA256 pinneado). Emite `tts-setup-progress`.
#[tauri::command]
#[specta::specta]
pub async fn interpreter_voice_setup(app: tauri::AppHandle, lang: String) -> Result<(), String> {
    crate::managers::tts::setup_lang(&app, &lang).await
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
