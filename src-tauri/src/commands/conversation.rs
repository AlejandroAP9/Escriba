//! Modo Conversación: una sesión de voz que termina en un documento.
//! Dos usos sobre el mismo núcleo: "converse" (la IA responde cada turno,
//! el frontend lo lee en voz alta) y "listen" (la IA calla y solo acumula:
//! entrevistas, notas habladas, actas). En ambos, al finalizar, el motor
//! local convierte la sesión en un documento limpio. Nada sale del equipo.
//! Sigue el patrón del Traductor: estado estático + interceptor en actions.rs.

use log::{debug, error, warn};
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

/// PRP-009: arranca (o reanuda) el journal durable con los turnos ya en RAM.
/// El replay al arrancar garantiza que un journal que nace tarde (reanudar
/// tras un acta en el mismo proceso) nazca completo, no con huecos.
fn journal_asegurar() {
    if crate::session_recorder::activo() {
        return;
    }
    let Ok(started) = STARTED.lock() else {
        return;
    };
    let Some(inicio) = started.as_ref() else {
        return; // sin sesión en curso no hay nada que journalizar
    };
    let wall_inicio = crate::session_recorder::ahora_wall_ms()
        .saturating_sub(inicio.elapsed().as_millis() as u64);
    drop(started);
    let turnos: Vec<(String, String, u64)> = TURNS
        .lock()
        .map(|t| {
            t.iter()
                .map(|t| (t.role.clone(), t.text.clone(), u64::from(t.at_secs) * 1000))
                .collect()
        })
        .unwrap_or_default();
    crate::session_recorder::arrancar(&mode(), wall_inicio, &turnos);
    pistas_rearmar();
}

/// PRP-009 Fase 4: arma las pistas de los canales YA encendidos. Los
/// apagados van por sus embudos (`hands_free_off`/`system_audio_off`); armar
/// dos veces es un no-op, así que llamarlo de más no cuesta nada.
fn pistas_rearmar() {
    if !crate::session_recorder::activo() {
        return;
    }
    let at = u64::from(elapsed_secs()) * 1000;
    if is_hands_free() {
        crate::session_recorder::pista_armar("mic", at);
    }
    if SYSTEM_AUDIO.load(Ordering::Relaxed) {
        crate::session_recorder::pista_armar("sys", at);
    }
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
    // PRP-009: el turno queda durable al llegar. Si el journal no está activo
    // (reanudación tras un acta), el replay de journal_asegurar lo incluye.
    if crate::session_recorder::activo() {
        crate::session_recorder::turno(&turn.role, &turn.text, u64::from(turn.at_secs) * 1000);
    } else if LISTENING.load(Ordering::Relaxed) {
        journal_asegurar();
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
    // PRP-009: journal durable desde el primer segundo de la sesión.
    journal_asegurar();
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
    // PRP-009: reset es descarte explícito; el journal y su carpeta se van ya.
    crate::session_recorder::cierre_descarte();
    if let Ok(mut t) = TURNS.lock() {
        t.clear();
    }
    *STARTED.lock().unwrap() = None;
    status()
}

/// Pliega los turnos de un journal honrando `Reinicio`: una re-transcripción
/// invalida todo turno anterior a su marca. Puro, y estable ante releerlo
/// (recuperar dos veces da lo mismo).
fn plegar_turnos_de_journal(eventos: &[crate::session_recorder::EventoSesion]) -> Vec<Turn> {
    let mut turns: Vec<Turn> = Vec::new();
    for e in eventos {
        match e {
            crate::session_recorder::EventoSesion::Turno { role, text, at_ms } => {
                turns.push(Turn {
                    role: role.clone(),
                    text: text.clone(),
                    at_secs: (*at_ms / 1000) as u32,
                });
            }
            crate::session_recorder::EventoSesion::Reinicio { .. } => turns.clear(),
            _ => {}
        }
    }
    turns
}

/// PRP-009 (Fase 2): una sesión recuperada del journal, lista para la
/// pantalla de Sesiones. El efecto de reconexión del frontend hace el resto.
#[derive(Serialize, Clone, Type)]
pub struct RecoveredSession {
    pub mode: String,
    pub turns: Vec<Turn>,
    pub doc: Option<SessionDoc>,
}

#[tauri::command]
#[specta::specta]
pub fn session_recovery_list() -> Vec<crate::session_recorder::ResumenPendiente> {
    crate::session_recorder::listar_pendientes()
}

/// Recupera una sesión pendiente: repuebla los estáticos de la sesión y
/// reengancha el journal para que los turnos nuevos sigan en el MISMO
/// archivo. Con una sesión en curso no pisa nada (recuperar es cosa del
/// arranque); llamarlo dos veces da `session_in_progress`, no duplicados.
#[tauri::command]
#[specta::specta]
pub fn session_recover(id: String) -> Result<RecoveredSession, String> {
    let en_curso = STARTED.lock().map(|s| s.is_some()).unwrap_or(false)
        && TURNS.lock().map(|t| !t.is_empty()).unwrap_or(false);
    if en_curso {
        return Err("session_in_progress".to_string());
    }
    let (eventos, _cola_rota) = crate::session_recorder::cargar_pendiente(&id)?;
    // Las colas rotas por el kill se truncan aquí, no en la captura.
    crate::session_recorder::sanar_pistas(&id);
    let mut modo = String::new();
    let mut doc: Option<SessionDoc> = None;
    let mut duracion_ms = 0u64;
    for e in &eventos {
        match e {
            crate::session_recorder::EventoSesion::Inicio { modo: m, .. } => {
                modo = m.clone();
            }
            crate::session_recorder::EventoSesion::Turno { at_ms, .. } => {
                duracion_ms = duracion_ms.max(*at_ms);
            }
            crate::session_recorder::EventoSesion::Documento {
                doc: d,
                animo,
                at_ms,
            } => {
                duracion_ms = duracion_ms.max(*at_ms);
                doc = Some(SessionDoc {
                    text: d.clone(),
                    mood: animo.clone(),
                });
            }
            crate::session_recorder::EventoSesion::Reinicio { at_ms, .. } => {
                duracion_ms = duracion_ms.max(*at_ms);
            }
            // Los eventos de pista no aportan turnos; el audio se sana aparte.
            crate::session_recorder::EventoSesion::Pista { .. } => {}
            crate::session_recorder::EventoSesion::Cierre { .. } => {
                return Err("session_closed".to_string());
            }
        }
    }
    if modo.is_empty() {
        return Err("journal sin evento de inicio".to_string());
    }
    let turns = plegar_turnos_de_journal(&eventos);
    // Reenganche best-effort: si falla, la sesión vive igual en RAM y el
    // arranque perezoso de push_turn abrirá un journal nuevo con replay.
    if let Err(e) = crate::session_recorder::reanudar(&id) {
        log::warn!("Sesión {id} recuperada sin reenganchar el journal: {e}");
    }
    if let Ok(mut m) = MODE.lock() {
        *m = modo.clone();
    }
    if let Ok(mut t) = TURNS.lock() {
        *t = turns.clone();
    }
    if let Ok(mut s) = STARTED.lock() {
        // checked_sub: en una máquina recién arrancada, restar una sesión
        // larga al reloj monótono puede no ser representable. Antes que un
        // panic, los mm:ss nuevos parten de cero.
        *s = Some(
            Instant::now()
                .checked_sub(std::time::Duration::from_millis(duracion_ms))
                .unwrap_or_else(Instant::now),
        );
    }
    Ok(RecoveredSession {
        mode: modo,
        turns,
        doc,
    })
}

/// Recupera una sesión RECONSTRUYENDO los turnos desde el audio grabado
/// (PRP-009: "la recuperación ofrece re-transcribir"). La base de tiempo de
/// cada segmento sale de su k-ésimo evento `pista{inicio}` del journal; los
/// turnos del journal quedan como respaldo si el audio no entrega nada.
#[tauri::command]
#[specta::specta]
pub async fn session_recover_retranscribe(
    app: tauri::AppHandle,
    id: String,
) -> Result<RecoveredSession, String> {
    use tauri::Manager;
    let en_curso = STARTED.lock().map(|s| s.is_some()).unwrap_or(false)
        && TURNS.lock().map(|t| !t.is_empty()).unwrap_or(false);
    if en_curso {
        return Err("session_in_progress".to_string());
    }
    let (eventos, _cola_rota) = crate::session_recorder::cargar_pendiente(&id)?;
    // Colas rotas por el kill: se truncan aquí, nunca en la captura.
    crate::session_recorder::sanar_pistas(&id);

    let mut modo = String::new();
    let mut doc: Option<SessionDoc> = None;
    let turnos_journal: Vec<Turn> = plegar_turnos_de_journal(&eventos);
    let mut bases: std::collections::HashMap<String, Vec<u64>> = Default::default();
    for e in &eventos {
        match e {
            crate::session_recorder::EventoSesion::Inicio { modo: m, .. } => modo = m.clone(),
            crate::session_recorder::EventoSesion::Turno { .. } => {}
            crate::session_recorder::EventoSesion::Reinicio { .. } => {}
            crate::session_recorder::EventoSesion::Documento {
                doc: d,
                animo,
                at_ms: _,
            } => {
                doc = Some(SessionDoc {
                    text: d.clone(),
                    mood: animo.clone(),
                });
            }
            crate::session_recorder::EventoSesion::Pista {
                pista,
                evento,
                at_ms,
                ..
            } if evento == "inicio" => {
                bases.entry(pista.clone()).or_default().push(*at_ms);
            }
            crate::session_recorder::EventoSesion::Pista { .. } => {}
            crate::session_recorder::EventoSesion::Cierre { .. } => {
                return Err("session_closed".to_string());
            }
        }
    }
    if modo.is_empty() {
        return Err("journal sin evento de inicio".to_string());
    }

    let segmentos = crate::session_recorder::listar_segmentos(&id);
    let tm = std::sync::Arc::clone(
        &app.state::<std::sync::Arc<crate::managers::transcription::TranscriptionManager>>(),
    );
    // Cada segmento degrada por su cuenta: uno corrupto no aborta el comando
    // (revisión del 30-ago). Una pista solo se considera CUBIERTA si todos
    // sus segmentos transcribieron; donde falte cobertura, mandan los turnos
    // del journal.
    let (audio_por_pista, pistas_fallidas) = tauri::async_runtime::spawn_blocking(
        move || -> (std::collections::HashMap<String, Vec<Turn>>, std::collections::HashSet<String>) {
            let mut audio: std::collections::HashMap<String, Vec<Turn>> = Default::default();
            let mut fallidas: std::collections::HashSet<String> = Default::default();
            for ruta in segmentos {
                let nombre = ruta
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let (pista, indice) = match nombre
                    .strip_suffix(".escaud2")
                    .and_then(|n| n.split_once('-'))
                    .and_then(|(p, i)| i.parse::<usize>().ok().map(|i| (p.to_string(), i)))
                {
                    Some(v) => v,
                    None => continue,
                };
                let resultado = crate::recording_crypto::escaud2_read_samples(&ruta)
                    .map_err(|e| e.to_string())
                    .and_then(|samples| {
                        if samples.is_empty() {
                            return Ok(Vec::new());
                        }
                        crate::studio::pipeline::transcribe_samples(&tm, &samples, |_| {})
                    });
                match resultado {
                    Ok(segs) => {
                        let base_ms = bases
                            .get(&pista)
                            .and_then(|v| v.get(indice))
                            .copied()
                            .unwrap_or(0);
                        let role = if pista == "mic" { "user" } else { "system" };
                        let destino = audio.entry(pista).or_default();
                        for sg in segs {
                            let text = sg.text.trim();
                            if !text.is_empty() {
                                destino.push(Turn {
                                    role: role.to_string(),
                                    text: text.to_string(),
                                    at_secs: ((base_ms + (sg.start_s * 1000.0) as u64) / 1000)
                                        as u32,
                                });
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Segmento {nombre} sin transcribir: {e}");
                        fallidas.insert(pista);
                    }
                }
            }
            (audio, fallidas)
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    let roles_cubiertos: std::collections::HashSet<&str> = audio_por_pista
        .iter()
        .filter(|(p, turnos)| !pistas_fallidas.contains(*p) && !turnos.is_empty())
        .map(|(p, _)| if p == "mic" { "user" } else { "system" })
        .collect();
    let turns = if roles_cubiertos.is_empty() {
        turnos_journal // el audio no dio nada: mejor el journal que vacío
    } else {
        let mut mezcla: Vec<Turn> = turnos_journal
            .iter()
            .filter(|t| !roles_cubiertos.contains(t.role.as_str()))
            .cloned()
            .collect();
        for (pista, turnos) in &audio_por_pista {
            if !pistas_fallidas.contains(pista) {
                mezcla.extend(turnos.iter().cloned());
            }
        }
        mezcla.sort_by_key(|t| t.at_secs);
        mezcla
    };
    let duracion_ms = turns
        .iter()
        .map(|t| u64::from(t.at_secs) * 1000)
        .max()
        .unwrap_or(0);
    if let Err(e) = crate::session_recorder::reanudar(&id) {
        log::warn!("Sesión {id} re-transcrita sin reenganchar el journal: {e}");
    }
    // Durable ANTES de devolver éxito: sin este snapshot, otro crash volvía
    // a cargar la transcripción vieja del journal (revisión del 30-ago).
    if !roles_cubiertos.is_empty() {
        let tuplas: Vec<(String, String, u64)> = turns
            .iter()
            .map(|t| (t.role.clone(), t.text.clone(), u64::from(t.at_secs) * 1000))
            .collect();
        crate::session_recorder::turnos_reemplazar(&tuplas, duracion_ms);
    }
    if let Ok(mut m) = MODE.lock() {
        *m = modo.clone();
    }
    if let Ok(mut t) = TURNS.lock() {
        *t = turns.clone();
    }
    if let Ok(mut s) = STARTED.lock() {
        *s = Some(
            Instant::now()
                .checked_sub(std::time::Duration::from_millis(duracion_ms))
                .unwrap_or_else(Instant::now),
        );
    }
    Ok(RecoveredSession {
        mode: modo,
        turns,
        doc,
    })
}

/// Solo lectura: el acta de una sesión pendiente, para exportarla desde el
/// diálogo de recuperación sin tocar el estado de la sesión en curso.
#[tauri::command]
#[specta::specta]
pub fn session_recovery_doc(id: String) -> Result<SessionDoc, String> {
    let (eventos, _) = crate::session_recorder::cargar_pendiente(&id)?;
    eventos
        .iter()
        .rev()
        .find_map(|e| match e {
            crate::session_recorder::EventoSesion::Documento { doc, animo, .. } => {
                Some(SessionDoc {
                    text: doc.clone(),
                    mood: animo.clone(),
                })
            }
            _ => None,
        })
        .ok_or_else(|| "sin documento".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn session_recovery_discard(id: String) -> Result<(), String> {
    crate::session_recorder::descartar_pendiente(&id)
}

/// Confirmación durable de una sesión pendiente exportada desde el diálogo.
#[tauri::command]
#[specta::specta]
pub fn session_recovery_confirm(id: String) -> Result<(), String> {
    crate::session_recorder::confirmar_pendiente(&id)
}

/// Confirmación durable de la sesión ACTIVA (el usuario exportó el acta a
/// Obsidian). Jamás se llama automáticamente al generar el acta: esa es la
/// condición de la revisión del 30-ago.
#[tauri::command]
#[specta::specta]
pub fn session_doc_confirm() {
    crate::session_recorder::cierre_documento();
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

/// Cascada nativa reutilizable. Juglar solo participa en lecturas
/// deliberadas ("Tu tinta en voz", leer selección): genera con la voz
/// clonada y tarda minutos en equipos de 16 GB, así que las Sesiones
/// conversacionales nunca lo usan. Si Juglar falla o excede el umbral, el
/// respaldo es la voz incluida; `say` queda como último recurso.
pub async fn speak_native(app: &tauri::AppHandle, text: &str, engine: &str) -> bool {
    let app_lang = crate::settings::get_settings(app).app_language;

    // Juglar expone una API local. La identidad `escriba` permite asignarle
    // una voz desde Juglar sin compartir datos ni credenciales en la nube.
    // speak_with_juglar solo devuelve éxito con el audio ya generado y
    // sonando en Juglar; cualquier otro resultado sigue la cascada normal.
    if engine == "juglar" && speak_with_juglar(Some(app), text, &app_lang).await {
        return true;
    }

    // Motor incluido: elegido directamente, o como respaldo cuando Juglar
    // no respondió a tiempo.
    if (engine == "included" || engine == "juglar")
        && app_lang.starts_with("es")
        && crate::managers::tts::installed(app)
    {
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

/// Tope de espera para que Juglar tenga el audio listo. En el hardware de
/// referencia (M4 Air 16 GB) una frase corta tarda ~90 s de generación, y
/// una lectura deliberada de varios párrafos puede multiplicarlo; pasado
/// este umbral la voz incluida responde mejor que seguir en silencio.
const JUGLAR_DEADLINE_SECS: u64 = 300;

/// Turno de lectura vigente. Como esperar a Juglar dura minutos, dos
/// peticiones seguidas se solapan: sin esto, la primera terminaría de
/// generar y empezaría a sonar encima (o después) de la segunda. Cada
/// petición toma un turno; la que descubre que ya no es la vigente cancela
/// su generación y se retira. Pararla a mano también avanza el contador.
static JUGLAR_EPOCH: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Descarta la lectura de Juglar en curso: la próxima vuelta del sondeo la
/// verá superada, cancelará en Juglar y no reproducirá nada.
pub fn cancel_pending_juglar_read() {
    JUGLAR_EPOCH.fetch_add(1, Ordering::SeqCst);
}

/// Fases que la interfaz muestra mientras Juglar trabaja. Una espera de
/// minutos sin señal se lee como que la app se colgó; con la fase a la
/// vista el usuario sabe si está descargando/cargando el modelo (lento la
/// primera vez) o ya generando, y puede decidir cancelar.
fn emit_juglar_phase(app: Option<&tauri::AppHandle>, phase: &str) {
    use tauri::Emitter;
    if let Some(app) = app {
        let _ = app.emit("juglar-read-phase", phase.to_string());
    }
}

/// URL local de Juglar. Las pruebas apuntan el núcleo a un servidor falso.
const JUGLAR_BASE_URL: &str = "http://127.0.0.1:17493";

async fn speak_with_juglar(app: Option<&tauri::AppHandle>, text: &str, app_lang: &str) -> bool {
    let handle = app.cloned();
    juglar_read(
        JUGLAR_BASE_URL,
        text,
        app_lang,
        std::sync::Arc::new(move |phase: &str| emit_juglar_phase(handle.as_ref(), phase)),
        JUGLAR_DEADLINE_SECS,
        std::time::Duration::from_secs(1),
    )
    .await
}

/// Cierra la fase "playing" cuando el audio ya habría terminado de sonar.
///
/// Reproduce Juglar, no Escriba, así que no llega ningún aviso de fin: sin
/// esto la interfaz se quedaría en "Reproduciendo" para siempre. Se usa la
/// duración del audio que informa la propia generación.
fn schedule_idle_after_playback(
    phase: std::sync::Arc<dyn Fn(&str) + Send + Sync>,
    secs: f64,
    my_epoch: u64,
) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs_f64(secs)).await;
        // Una lectura posterior ya habrá puesto su propia fase: cerrarla
        // aquí la borraría de la pantalla.
        if JUGLAR_EPOCH.load(Ordering::SeqCst) == my_epoch {
            phase("idle");
        }
    });
}

/// Núcleo de la lectura con Juglar, con las dependencias inyectadas para
/// poder probarlo: a dónde hablar, cómo publicar las fases y con qué
/// tiempos. `speak_with_juglar` le pasa los valores reales.
async fn juglar_read(
    base_url: &str,
    text: &str,
    app_lang: &str,
    phase: std::sync::Arc<dyn Fn(&str) + Send + Sync>,
    deadline_secs: u64,
    poll_interval: std::time::Duration,
) -> bool {
    let my_epoch = JUGLAR_EPOCH.fetch_add(1, Ordering::SeqCst) + 1;
    let superseded = || JUGLAR_EPOCH.load(Ordering::SeqCst) != my_epoch;
    // Cierra la fase pase lo que pase: si se sale por error o por respaldo,
    // la interfaz no puede quedarse con "Generando…" para siempre.
    let finish = |value: bool| {
        phase("idle");
        value
    };

    let client = match reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_millis(600))
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(_) => return false,
    };
    let language = app_lang.split(['-', '_']).next().unwrap_or("es");

    phase("requesting");

    // /speak encola y responde en milisegundos con status "generating"; el
    // audio existe recién cuando la generación termina. Dar por hablado el
    // texto con el 200 del POST producía 90 s de silencio y ningún respaldo.
    let generation_id = match client
        .post(format!("{base_url}/speak"))
        .header("X-Juglar-Client-Id", "escriba")
        .json(&serde_json::json!({ "text": text, "language": language }))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => {
            match response.json::<serde_json::Value>().await {
                Ok(body) => match body.get("id").and_then(|v| v.as_str()) {
                    Some(id) => id.to_string(),
                    None => return finish(false),
                },
                Err(_) => return finish(false),
            }
        }
        Ok(response) => {
            log::warn!("Juglar rechazó la lectura: HTTP {}", response.status());
            return finish(false);
        }
        Err(error) => {
            log::debug!("Juglar no está disponible; se usará la voz incluida: {error}");
            return finish(false);
        }
    };

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(deadline_secs);
    let status_url = format!("{base_url}/generate/{generation_id}");
    let mut poll_failures = 0u8;
    let cancel_url = format!("{base_url}/generate/{generation_id}/cancel");
    let cancel_in_juglar = |client: reqwest::Client, url: String| async move {
        let _ = client
            .post(url)
            .header("X-Juglar-Client-Id", "escriba")
            .send()
            .await;
    };
    loop {
        tokio::time::sleep(poll_interval).await;

        // Llegó una lectura más nueva (o el usuario paró): esta ya no manda.
        // Devuelve true para que el respaldo no lea un texto viejo encima de
        // lo que el usuario acaba de pedir.
        if superseded() {
            log::debug!("Lectura en Juglar superada por una más reciente; se cancela");
            cancel_in_juglar(client.clone(), cancel_url.clone()).await;
            return finish(true);
        }

        let snapshot = match client
            .get(&status_url)
            .header("X-Juglar-Client-Id", "escriba")
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => r.json::<serde_json::Value>().await.ok(),
            _ => None,
        };
        let Some(snapshot) = snapshot else {
            // Tres fallos seguidos = el servidor se cayó a mitad de la
            // generación; un fallo aislado puede ser solo carga.
            poll_failures += 1;
            if poll_failures >= 3 {
                log::warn!("Juglar dejó de responder durante la generación; respaldo local");
                // El corte pudo ser pasajero: sin cancelar, Juglar terminaría
                // la generación y la reproduciría encima de la voz incluida
                // que está por sonar. Es de mejor esfuerzo, como en el resto
                // de las salidas.
                cancel_in_juglar(client.clone(), cancel_url.clone()).await;
                return finish(false);
            }
            continue;
        };
        poll_failures = 0;

        match snapshot.get("status").and_then(|v| v.as_str()) {
            // Al completarse, la burbuja de Juglar reproduce el audio
            // (fuente "rest"): recién aquí la lectura está de verdad hecha.
            Some("completed") => {
                // Quien reproduce es Juglar, así que Escriba no recibe aviso
                // del final; sin esto la interfaz se quedaría en
                // "Reproduciendo" para siempre. La duración del audio viene en
                // la propia instantánea: se programa el cierre y se devuelve
                // el control enseguida.
                let secs = snapshot
                    .get("duration")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    .clamp(0.0, 600.0);
                phase("playing");
                schedule_idle_after_playback(phase.clone(), secs, my_epoch);
                return true;
            }
            // Juglar distingue la carga del modelo de la síntesis; la
            // primera es la lenta cuando el modelo está frío.
            Some("loading_model") => phase("loading_model"),
            Some("generating") => phase("generating"),
            Some("failed") => {
                let error = snapshot
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                log::warn!("Juglar no pudo generar la lectura: {error}");
                return finish(false);
            }
            _ => {}
        }

        if std::time::Instant::now() >= deadline {
            log::warn!(
                "Juglar superó el umbral de {deadline_secs} s; se cancela y se usa la voz incluida"
            );
            // Libera la cola de Juglar; si la cancelación falla no cambia
            // la decisión de caer al respaldo.
            cancel_in_juglar(client.clone(), cancel_url.clone()).await;
            return finish(false);
        }
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
    // Una lectura de Juglar puede estar generándose sin sonar todavía:
    // pararla es descartar ese turno, no solo callar lo que ya suena.
    cancel_pending_juglar_read();
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
        return Ok(true); // ya activo (pista incluida)
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
                    // Se guarda antes de mover `samples` al hilo: sirve para
                    // decir en el log cuánto audio se le dio al motor cuando
                    // devuelve vacío, que es la diferencia entre "no te oyó" y
                    // "te oyó y no supo transcribirlo".
                    let samples_len = samples.len();
                    let app3 = app2.clone();
                    let tm3 = Arc::clone(&app2.state::<Arc<TranscriptionManager>>());
                    tauri::async_runtime::spawn(async move {
                        let segs = tauri::async_runtime::spawn_blocking(move || {
                            crate::studio::pipeline::transcribe_samples(&tm3, &samples, |_| {})
                        })
                        .await;
                        // Los tres modos de fallar se distinguen a propósito.
                        //
                        // Antes esto era `if let Ok(Ok(segs))` y punto: un error
                        // del motor, un texto vacío y una sesión pausada se
                        // tragaban igual, sin registrar nada. Como el
                        // `emit_turn_phase("listening")` de abajo corre pase lo
                        // que pase, el indicador seguía ciclando y desde fuera
                        // los tres se veían idénticos: "aparece que escucha,
                        // aparece el cambio de turno, y no sale texto" (QA de
                        // Flor, 29-jul). Sin una sola línea en el log, la causa
                        // solo se podía conjeturar.
                        use tauri::Emitter;
                        match segs {
                            Err(e) => {
                                error!("manos libres: la tarea de transcripción murió: {e}");
                                let _ = app3.emit("conversation-turn-error", "engine");
                            }
                            Ok(Err(e)) => {
                                error!("manos libres: el motor falló al transcribir el turno: {e}");
                                let _ = app3.emit("conversation-turn-error", "engine");
                            }
                            Ok(Ok(segs)) => {
                                let text = crate::studio::segments::group_paragraphs(&segs)
                                    .join(" ")
                                    .trim()
                                    .to_string();
                                if text.is_empty() {
                                    // El motor respondió sin devolver palabras.
                                    // Pasa con modelos que no digieren fragmentos
                                    // cortos por esta vía: el turno se cierra, el
                                    // indicador sigue, y el acta nunca crece.
                                    warn!(
                                        "manos libres: turno de {:.1}s transcrito a texto VACÍO \
                                         (modelo que no digiere este fragmento por la vía del Estudio)",
                                        samples_len as f32 / 16_000.0
                                    );
                                    let _ = app3.emit("conversation-turn-error", "empty");
                                } else if !is_listening() {
                                    debug!(
                                        "manos libres: turno descartado, la sesión ya no escucha"
                                    );
                                } else {
                                    let turn = push_turn("user", &text);
                                    let _ = app3.emit("conversation-turn", turn);
                                    // Manos libres también pasa por el Intérprete:
                                    // tu voz sale traducida sin tocar una tecla.
                                    crate::actions::interpreter_reply_flow(&app3, text);
                                }
                            }
                        }
                        emit_turn_phase(&app3, "listening");
                    });
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    // PRP-009: con el canal recién encendido, la pista se arma si hay journal.
    pistas_rearmar();
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
        return Ok(true); // ya activo (pista incluida)
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
                // El store(false) de arriba hace que system_audio_off retorne
                // temprano después: la pista se desarma AQUÍ o no se desarma
                // nunca (revisión del 30-ago).
                crate::session_recorder::pista_desarmar("sys");
                break;
            }
            let n = crate::system_audio::read(&mut chunk);
            if n > 0 {
                // PRP-009: la pista cruda (PRE-VAD) de la sesión. Solo
                // try_send: jamás bloquea este worker.
                crate::session_recorder::pista_sys(&chunk[..n]);
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

    // PRP-009: con el canal recién encendido, la pista se arma si hay journal.
    pistas_rearmar();
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
    engine: String,
) -> bool {
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            speak_via_blocking(&app, &text, &lang, &device, &gender, &engine)
        })
        .await
        .unwrap_or(false)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, text, lang, device, gender, engine);
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
    engine: &str,
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
    if engine == "included" && gender != "m" && crate::managers::tts::installed_lang(app, lang) {
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
    // PRP-009: la pista se desarma en el embudo, venga de donde venga el
    // apagado (toggle, stop, reset, finish). El worker drena y finaliza.
    crate::session_recorder::pista_desarmar("sys");
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
    // DESPUÉS de drenar: stop_recording todavía empuja audio por el raw tap,
    // y desarmar antes lo convertía en no-op perdiendo la cola (revisión del
    // 30-ago). El worker recibe hasta la última muestra y recién ahí finaliza.
    crate::session_recorder::pista_desarmar("mic");
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
    let final_text = lines.join("\n").trim().to_string();
    // PRP-009: el acta se cifra al journal, pero NO se cierra aquí. Un kill
    // entre esta línea y que React reciba el resultado debe dejar la sesión
    // recuperable CON su acta; el cierre llega por confirmación explícita
    // (Fase 2) o por descarte del usuario (reset, que borra la carpeta).
    crate::session_recorder::documento(&final_text, &mood, u64::from(elapsed_secs()) * 1000);
    Ok(SessionDoc {
        text: final_text,
        mood,
    })
}

#[cfg(test)]
mod juglar_read_tests {
    //! Camino crítico de la lectura con Juglar contra un servidor falso.
    //!
    //! Lo que se protege aquí es la corrección que motivó todo: `/speak`
    //! responde en milisegundos pero el audio tarda minutos, así que el
    //! éxito solo puede declararse con la generación terminada, y toda
    //! salida tiene que cerrar la fase y soltar la cola de Juglar.

    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};

    /// JUGLAR_EPOCH es global: dos pruebas en paralelo se supersederían
    /// entre sí. Se serializan.
    static EPOCH_GUARD: Mutex<()> = Mutex::new(());

    /// Toma el turno serializado ignorando el envenenamiento: si una prueba
    /// falla, las demás deben seguir informando lo suyo en vez de caer todas
    /// con un pánico prestado.
    fn epoch_guard() -> std::sync::MutexGuard<'static, ()> {
        EPOCH_GUARD.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Qué responde el Juglar falso a cada `GET /generate/{id}`, en orden.
    /// `None` = responder 500, para simular un corte.
    struct Script {
        statuses: Vec<Option<&'static str>>,
    }

    struct FakeJuglar {
        base_url: String,
        hits: Arc<Mutex<Vec<String>>>,
    }

    impl FakeJuglar {
        fn start(script: Script) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            let hits = Arc::new(Mutex::new(Vec::new()));
            let hits_thread = hits.clone();

            std::thread::spawn(move || {
                let mut step = 0usize;
                for stream in listener.incoming() {
                    let Ok(mut stream) = stream else { break };
                    let Some(request_line) = read_request_line(&mut stream) else {
                        continue;
                    };
                    hits_thread.lock().unwrap().push(request_line.clone());

                    if request_line.contains("/speak") {
                        respond(&mut stream, 200, r#"{"id":"gen-1","status":"generating"}"#);
                    } else if request_line.contains("/cancel") {
                        respond(&mut stream, 200, r#"{"message":"cancelled"}"#);
                    } else {
                        let status = script.statuses.get(step).copied().flatten();
                        step += 1;
                        match status {
                            Some(s) => respond(
                                &mut stream,
                                200,
                                &format!(
                                    r#"{{"id":"gen-1","status":"{s}","duration":0.01,"error":null}}"#
                                ),
                            ),
                            // Sin entrada en el guion: sigue "generating"
                            // para que la prueba de timeout pueda vencer.
                            None if step > script.statuses.len() => respond(
                                &mut stream,
                                200,
                                r#"{"id":"gen-1","status":"generating","duration":null}"#,
                            ),
                            None => respond(&mut stream, 500, r#"{"detail":"boom"}"#),
                        }
                    }
                }
            });

            FakeJuglar { base_url, hits }
        }

        fn cancelled(&self) -> bool {
            self.hits
                .lock()
                .unwrap()
                .iter()
                .any(|h| h.contains("/cancel"))
        }
    }

    fn read_request_line(stream: &mut TcpStream) -> Option<String> {
        let mut reader = BufReader::new(stream.try_clone().ok()?);
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        Some(line.trim().to_string())
    }

    fn respond(stream: &mut TcpStream, code: u16, body: &str) {
        let _ = write!(
            stream,
            "HTTP/1.1 {code} OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.flush();
    }

    /// Corre el núcleo contra el servidor falso y devuelve
    /// (resultado, fases emitidas).
    async fn run(fake: &FakeJuglar, deadline_secs: u64) -> (bool, Vec<String>) {
        let phases = Arc::new(Mutex::new(Vec::new()));
        let sink = phases.clone();
        let ok = juglar_read(
            &fake.base_url,
            "hola",
            "es",
            Arc::new(move |p: &str| sink.lock().unwrap().push(p.to_string())),
            deadline_secs,
            std::time::Duration::from_millis(20),
        )
        .await;
        let seen = phases.lock().unwrap().clone();
        (ok, seen)
    }

    #[tokio::test]
    async fn completa_y_recorre_las_fases() {
        let _guard = epoch_guard();
        let fake = FakeJuglar::start(Script {
            statuses: vec![Some("loading_model"), Some("generating"), Some("completed")],
        });
        let (ok, phases) = run(&fake, 30).await;
        assert!(ok, "una generación completada es una lectura exitosa");
        assert_eq!(
            phases,
            vec!["requesting", "loading_model", "generating", "playing"],
            "la interfaz debe poder distinguir carga de síntesis"
        );
    }

    #[tokio::test]
    async fn generacion_fallida_cae_al_respaldo_y_cierra_la_fase() {
        let _guard = epoch_guard();
        let fake = FakeJuglar::start(Script {
            statuses: vec![Some("generating"), Some("failed")],
        });
        let (ok, phases) = run(&fake, 30).await;
        assert!(!ok, "si Juglar falla debe hablar la voz incluida");
        assert_eq!(
            phases.last().unwrap(),
            "idle",
            "la fase no puede quedar viva"
        );
    }

    #[tokio::test]
    async fn el_umbral_cancela_y_cierra_la_fase() {
        let _guard = epoch_guard();
        // Nunca completa: el guion se agota y sigue "generating".
        let fake = FakeJuglar::start(Script {
            statuses: vec![Some("generating"); 2],
        });
        let (ok, phases) = run(&fake, 0).await;
        assert!(!ok, "pasado el umbral responde la voz incluida");
        assert_eq!(phases.last().unwrap(), "idle");
        assert!(fake.cancelled(), "hay que soltar la cola de Juglar");
    }

    #[tokio::test]
    async fn tres_consultas_fallidas_cancelan_antes_del_respaldo() {
        let _guard = epoch_guard();
        // Tres 500 seguidos: el servidor se cayó a mitad de la generación.
        let fake = FakeJuglar::start(Script {
            statuses: vec![None, None, None],
        });
        let (ok, phases) = run(&fake, 30).await;
        assert!(!ok);
        assert_eq!(phases.last().unwrap(), "idle");
        assert!(
            fake.cancelled(),
            "si el corte fue pasajero, sin cancelar Juglar hablaría encima del respaldo"
        );
    }

    #[tokio::test]
    async fn una_lectura_mas_nueva_descarta_la_anterior() {
        let _guard = epoch_guard();
        let fake = FakeJuglar::start(Script {
            statuses: vec![Some("generating"); 50],
        });
        let phases = Arc::new(Mutex::new(Vec::new()));
        let sink = phases.clone();
        let url = fake.base_url.clone();
        let vieja = tokio::spawn(async move {
            juglar_read(
                &url,
                "texto viejo",
                "es",
                Arc::new(move |p: &str| sink.lock().unwrap().push(p.to_string())),
                30,
                std::time::Duration::from_millis(20),
            )
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        cancel_pending_juglar_read(); // llega una lectura más nueva

        let ok = vieja.await.unwrap();
        assert!(
            ok,
            "devuelve éxito para que el respaldo no lea el texto viejo encima del nuevo"
        );
        assert_eq!(phases.lock().unwrap().last().unwrap(), "idle");
        assert!(fake.cancelled(), "la lectura descartada libera la cola");
    }

    #[tokio::test]
    async fn parar_a_mano_detiene_una_lectura_que_aun_no_suena() {
        let _guard = epoch_guard();
        let fake = FakeJuglar::start(Script {
            statuses: vec![Some("loading_model"); 50],
        });
        let phases = Arc::new(Mutex::new(Vec::new()));
        let sink = phases.clone();
        let url = fake.base_url.clone();
        let lectura = tokio::spawn(async move {
            juglar_read(
                &url,
                "texto",
                "es",
                Arc::new(move |p: &str| sink.lock().unwrap().push(p.to_string())),
                30,
                std::time::Duration::from_millis(20),
            )
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(60)).await;
        // Lo que hace conversation_speak_stop: descartar el turno vigente.
        cancel_pending_juglar_read();

        assert!(lectura.await.unwrap());
        assert!(
            fake.cancelled(),
            "parar debe cancelar también lo que se está generando en silencio"
        );
    }
}

#[cfg(test)]
mod plegado_tests {
    use super::plegar_turnos_de_journal;
    use crate::session_recorder::EventoSesion;

    #[test]
    fn el_plegado_honra_el_reinicio_de_la_retranscripcion() {
        let eventos = vec![
            EventoSesion::Turno {
                role: "user".into(),
                text: "transcripcion vieja".into(),
                at_ms: 1_000,
            },
            EventoSesion::Turno {
                role: "assistant".into(),
                text: "respuesta vieja".into(),
                at_ms: 2_000,
            },
            EventoSesion::Reinicio {
                motivo: "retranscripcion".into(),
                at_ms: 3_000,
            },
            EventoSesion::Turno {
                role: "user".into(),
                text: "transcripcion nueva".into(),
                at_ms: 1_000,
            },
        ];
        // Dos pasadas: recuperar dos veces da exactamente lo mismo.
        for _ in 0..2 {
            let turns = plegar_turnos_de_journal(&eventos);
            assert_eq!(turns.len(), 1);
            assert_eq!(turns[0].text, "transcripcion nueva");
        }
    }
}
