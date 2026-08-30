//! Journal durable de sesiones (PRP-009, Fase 1).
//!
//! Cada sesión activa escribe `sessions/<id>/journal.jsonl`: una línea por
//! evento, y la línea ENTERA es el evento JSON cifrado con la API estricta
//! (`esc1:`). Un crash a mitad de reunión deja este archivo como única
//! fuente de verdad; la recuperación (Fase 2) lo lee con `parsear_journal`.
//!
//! Principios no negociables (premortem del PRP-009):
//! - Fail-closed todo-o-nada: sin cifrado estricto no se crea NI el
//!   directorio. Jamás texto claro en disco, jamás "temporalmente".
//! - Append-only: nunca se reescribe una línea sellada. La recuperación
//!   tolera la cola rota (kill a mitad de write) descartándola.
//! - El reloj es relativo a la sesión (`at_ms` desde el inicio): el `Instant`
//!   de conversation.rs no se serializa; la hora de pared solo etiqueta el
//!   inicio. Un cambio de hora del sistema no mueve ningún `mm:ss`.

use log::{info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Versión del formato del journal. Sube si cambia el esquema de eventos.
const VERSION: u32 = 1;

/// Raíz `sessions/` bajo el directorio de datos. Se fija una vez al arrancar
/// la app; sin init (tests, CLI headless) el grabador queda inerte.
static RAIZ: OnceLock<PathBuf> = OnceLock::new();

/// Grabador activo (una sesión a la vez, igual que conversation.rs).
static ACTIVO: Mutex<Option<Grabador>> = Mutex::new(None);

/// El aviso de "sin cifrado, sesión solo en RAM" se dice UNA vez por proceso.
static AVISADO_SIN_CIFRADO: OnceLock<()> = OnceLock::new();

/// Un evento del journal. `tag` interno para que cada línea se autodescriba.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "tipo", rename_all = "snake_case")]
pub enum EventoSesion {
    /// Primera línea SIEMPRE. `wall_ms` etiqueta el inicio real de la sesión
    /// (no el del journal: un journal puede nacer tarde, en reanudación).
    Inicio {
        wall_ms: u64,
        modo: String,
        version: u32,
    },
    Turno {
        role: String,
        text: String,
        at_ms: u64,
    },
    /// El acta generada. Va ANTES del cierre: `cierre` significa "documento
    /// durable" o "descarte explícito", nunca "la captura paró".
    Documento {
        doc: String,
        animo: String,
        at_ms: u64,
    },
    /// Vida de una pista de audio: `inicio` al armarse, `hueco` cuando el
    /// canal descartó muestras (el tiempo NO se comprime: el worker rellena
    /// silencio), `corte` si la pista murió por error, `fin` al cierre limpio.
    Pista {
        pista: String,
        evento: String,
        at_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        muestras_perdidas: Option<u64>,
    },
    Cierre {
        motivo: String,
    },
}

struct Grabador {
    dir: PathBuf,
    archivo: File,
}

/// Fija la raíz `sessions/`. Llamar una vez desde el setup de lib.rs.
pub fn init(data_dir: &Path) {
    let _ = RAIZ.set(data_dir.join("sessions"));
}

pub fn ahora_wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// ID aleatorio de sesión: 32 hex. El formato ES el contrato de validación
/// de los comandos de recuperación (Fase 2): nada fuera de `[0-9a-f]{32}`.
fn id_nuevo() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::getrandom(&mut bytes).map_err(|_| "sin CSPRNG para el id".to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Serializa y cifra un evento como línea del journal (sin `\n`).
/// Núcleo puro: el cifrador entra como parámetro para testear sin llavero.
fn linea_de_evento(
    evento: &EventoSesion,
    cifrador: &dyn Fn(&str) -> Result<String, String>,
) -> Result<String, String> {
    let json = serde_json::to_string(evento).map_err(|e| e.to_string())?;
    cifrador(&json)
}

/// Lee un journal completo con tolerancia a cola rota.
///
/// Devuelve los eventos válidos y si hubo cola descartada. La ÚLTIMA línea
/// ilegible (kill a mitad de write) se descarta en silencio; una línea
/// ilegible en el MEDIO corta ahí (lo anterior se conserva): un journal
/// append-only no puede tener huecos legales, así que lo que siga es
/// inatribuible y no se inventa.
pub fn parsear_journal(
    contenido: &str,
    descifrador: &dyn Fn(&str) -> Option<String>,
) -> (Vec<EventoSesion>, bool) {
    let mut eventos = Vec::new();
    let mut cola_rota = false;
    for linea in contenido.lines() {
        if linea.trim().is_empty() {
            continue;
        }
        let Some(json) = descifrador(linea) else {
            cola_rota = true;
            break;
        };
        match serde_json::from_str::<EventoSesion>(&json) {
            Ok(e) => eventos.push(e),
            Err(_) => {
                cola_rota = true;
                break;
            }
        }
    }
    (eventos, cola_rota)
}

/// Cifrador real: la API estricta del llavero. Jamás degrada a claro.
fn cifrador_real(texto: &str) -> Result<String, String> {
    crate::history_crypto::cifrar_campo_estricto(texto)
}

/// Por qué falló el arranque del journal: el cifrado (fail-closed, aviso
/// único) o el disco (se avisa cada vez, puede ser transitorio).
#[derive(Debug)]
enum FalloArranque {
    Cifrado(String),
    Disco(String),
}

/// Núcleo del arranque con raíz y cifrador inyectables (testeable sin
/// llavero ni estado global). Contrato todo-o-nada: TODO se cifra antes de
/// tocar el disco; un fallo en CUALQUIER línea del lote (no solo la primera)
/// aborta sin dejar ni el directorio, y un fallo de disco limpia lo que
/// hubiera quedado a medias.
fn arrancar_nucleo(
    raiz: &Path,
    id: &str,
    modo: &str,
    wall_ms_inicio: u64,
    turnos_previos: &[(String, String, u64)],
    cifrador: &dyn Fn(&str) -> Result<String, String>,
) -> Result<Grabador, FalloArranque> {
    // 1) Cifrar TODO antes de tocar el disco.
    let mut lineas = Vec::with_capacity(1 + turnos_previos.len());
    let inicio = EventoSesion::Inicio {
        wall_ms: wall_ms_inicio,
        modo: modo.to_string(),
        version: VERSION,
    };
    let todo_cifrado = (|| -> Result<(), String> {
        lineas.push(linea_de_evento(&inicio, cifrador)?);
        for (role, text, at_ms) in turnos_previos {
            lineas.push(linea_de_evento(
                &EventoSesion::Turno {
                    role: role.clone(),
                    text: text.clone(),
                    at_ms: *at_ms,
                },
                cifrador,
            )?);
        }
        Ok(())
    })();
    if let Err(e) = todo_cifrado {
        return Err(FalloArranque::Cifrado(e));
    }

    // 2) Recién ahora, disco.
    let dir = raiz.join(id);
    let resultado = (|| -> std::io::Result<File> {
        fs::create_dir_all(&dir)?;
        let mut archivo = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(dir.join("journal.jsonl"))?;
        for l in &lineas {
            archivo.write_all(l.as_bytes())?;
            archivo.write_all(b"\n")?;
        }
        archivo.sync_data()?;
        Ok(archivo)
    })();
    match resultado {
        Ok(archivo) => Ok(Grabador { dir, archivo }),
        Err(e) => {
            // Sin directorio a medias: si algo quedó, fuera.
            let _ = fs::remove_dir_all(&dir);
            Err(FalloArranque::Disco(e.to_string()))
        }
    }
}

/// Arranca el journal de la sesión si no hay uno activo.
///
/// `turnos_previos` repite los turnos ya en RAM (reanudación tras un acta en
/// el mismo proceso): el journal nuevo nace completo, no con huecos.
pub fn arrancar(modo: &str, wall_ms_inicio: u64, turnos_previos: &[(String, String, u64)]) {
    let Some(raiz) = RAIZ.get() else {
        return; // CLI headless o tests sin init: inerte a propósito.
    };
    let mut guard = match ACTIVO.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    if guard.is_some() {
        return;
    }
    let id = match id_nuevo() {
        Ok(id) => id,
        Err(e) => {
            warn!("Journal de sesión sin id: {e}");
            return;
        }
    };
    match arrancar_nucleo(
        raiz,
        &id,
        modo,
        wall_ms_inicio,
        turnos_previos,
        &cifrador_real,
    ) {
        Ok(grabador) => {
            info!("Journal de sesión activo: {id}");
            *guard = Some(grabador);
            drop(guard);
            eventos_pista_abrir();
        }
        Err(FalloArranque::Cifrado(e)) => {
            if AVISADO_SIN_CIFRADO.set(()).is_ok() {
                warn!("Journal de sesión desactivado (fail-closed): {e}. La sesión sigue en RAM.");
            }
        }
        Err(FalloArranque::Disco(e)) => {
            warn!("Journal de sesión no pudo crearse: {e}");
        }
    }
}

/// ¿Hay journal activo? (para el arranque perezoso desde push_turn).
pub fn activo() -> bool {
    ACTIVO.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// Apendea un evento al journal activo con fsync. Si el disco o el cifrado
/// fallan a mitad de sesión, el grabador se apaga (lo escrito queda, que es
/// recuperable) y la sesión sigue en RAM: jamás se bloquea ni se degrada a
/// claro.
fn apendear(evento: &EventoSesion) {
    let mut guard = match ACTIVO.lock() {
        Ok(g) => g,
        Err(_) => return,
    };
    let Some(grabador) = guard.as_mut() else {
        return;
    };
    let resultado = linea_de_evento(evento, &cifrador_real).and_then(|linea| {
        grabador
            .archivo
            .write_all(linea.as_bytes())
            .and_then(|()| grabador.archivo.write_all(b"\n"))
            .and_then(|()| grabador.archivo.sync_data())
            .map_err(|e| e.to_string())
    });
    if let Err(e) = resultado {
        warn!("Journal de sesión apagado a mitad ({e}); lo escrito queda para recuperación.");
        *guard = None;
    }
}

pub fn turno(role: &str, text: &str, at_ms: u64) {
    apendear(&EventoSesion::Turno {
        role: role.to_string(),
        text: text.to_string(),
        at_ms,
    });
}

/// El acta, durable ANTES del cierre (condición de la revisión del 30-ago).
pub fn documento(doc: &str, animo: &str, at_ms: u64) {
    apendear(&EventoSesion::Documento {
        doc: doc.to_string(),
        animo: animo.to_string(),
        at_ms,
    });
}

/// Cierre por documento CONFIRMADO por el frontend (revisión del 30-ago: el
/// acta generada NO cierra el journal; un kill entre generarla y que React
/// la reciba debe dejar la sesión recuperable con su acta). Este cierre lo
/// dispara el comando de confirmación de la Fase 2; hasta entonces, los
/// journals con `documento` y sin `cierre` son exactamente lo que la
/// recuperación ofrece.
pub fn cierre_documento() {
    pistas_desarmar_todas();
    eventos_pista_cerrar();
    apendear(&EventoSesion::Cierre {
        motivo: "documento".to_string(),
    });
    let _ = ACTIVO.lock().map(|mut g| *g = None);
}

/// Descarte explícito del usuario (reset): el journal y el directorio se
/// eliminan ya. No es la retención de la Fase 5: es la voluntad del usuario.
pub fn cierre_descarte() {
    pistas_desarmar_todas();
    eventos_pista_cerrar();
    let dir = {
        let mut guard = match ACTIVO.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        guard.take().map(|g| g.dir)
    };
    if let Some(dir) = dir {
        // Auditable a propósito: borrar una sesión es la única operación
        // destructiva del módulo y no puede ser invisible en el log.
        info!("Sesión descartada por reset: {}", dir.display());
        if let Err(e) = fs::remove_dir_all(&dir) {
            warn!("No se pudo borrar la sesión descartada: {e}");
        }
    }
}

// ──────────────────────── Pistas de audio (Fase 4) ────────────────────────
//
// Fronteras acordadas (revisión del 30-ago):
// - el camino de captura SOLO hace try_send: jamás lee, recupera ni
//   finaliza contenedores, y jamás espera al disco;
// - un error del worker corta ESA pista y lo deja escrito; el journal sigue;
// - el contador de muestras avanza también al descartar: un descarte es un
//   hueco de silencio en la pista, nunca tiempo comprimido.

const MUESTRAS_POR_SEGUNDO: u64 = 16_000;
/// Tope del canal: ~32 trozos en vuelo. Lleno → se descarta (y queda hueco).
const CAPACIDAD_CANAL: usize = 32;

struct Trozo {
    /// Offset (en muestras desde el inicio de la pista) de la primera muestra.
    inicio: u64,
    muestras: Vec<f32>,
}

struct ProductorPista {
    tx: std::sync::mpsc::SyncSender<Trozo>,
    /// Avanza SIEMPRE, se envíe o se descarte: es el reloj de la pista.
    /// Compartido con el worker para que un descarte al FINAL (canal lleno y
    /// pista desarmada justo después) también se vuelva silencio + hueco
    /// (revisión del 30-ago: antes ese descarte terminal desaparecía).
    offset: std::sync::Arc<std::sync::atomic::AtomicU64>,
    worker: Option<std::thread::JoinHandle<()>>,
}

static PISTA_MIC: Mutex<Option<ProductorPista>> = Mutex::new(None);
static PISTA_SYS: Mutex<Option<ProductorPista>> = Mutex::new(None);

/// Canal de eventos de pista → journal, con worker PROPIO. Un worker de
/// audio bloqueado en su disco jamás arrastra al journal, y un journal lento
/// jamás bloquea al audio: los eventos van por try_send (lleno → warn, el
/// audio sigue). Prometido por el PRP; lo cerró la revisión del 30-ago.
static EVENTOS_PISTA: Mutex<
    Option<(
        std::sync::mpsc::SyncSender<EventoSesion>,
        std::thread::JoinHandle<()>,
    )>,
> = Mutex::new(None);

/// Encola un evento de pista al journal sin bloquear jamás al que llama.
fn registrar_pista(evento: EventoSesion) {
    let Ok(guard) = EVENTOS_PISTA.lock() else {
        return;
    };
    let Some((tx, _)) = guard.as_ref() else {
        return;
    };
    if let Err(e) = tx.try_send(evento) {
        warn!("Evento de pista sin registrar (canal de journal saturado o cerrado): {e}");
    }
}

fn eventos_pista_abrir() {
    let Ok(mut guard) = EVENTOS_PISTA.lock() else {
        return;
    };
    if guard.is_some() {
        return;
    }
    let (tx, rx) = std::sync::mpsc::sync_channel::<EventoSesion>(64);
    if let Ok(handle) = std::thread::Builder::new()
        .name("escriba-journal-pistas".into())
        .spawn(move || {
            for evento in rx {
                apendear(&evento);
            }
        })
    {
        *guard = Some((tx, handle));
    }
}

/// Cierra el canal y espera a que los eventos pendientes lleguen al journal.
/// SIEMPRE después de desarmar las pistas (sus `fin` viajan por aquí) y
/// antes de escribir `cierre`.
fn eventos_pista_cerrar() {
    let par = {
        let Ok(mut guard) = EVENTOS_PISTA.lock() else {
            return;
        };
        guard.take()
    };
    if let Some((tx, handle)) = par {
        drop(tx);
        let _ = handle.join();
    }
}

fn slot_de(pista: &str) -> &'static Mutex<Option<ProductorPista>> {
    if pista == "mic" {
        &PISTA_MIC
    } else {
        &PISTA_SYS
    }
}

/// Núcleo del empuje (testeable): avanza el offset pase lo que pase y jamás
/// bloquea. `Disconnected` = worker muerto: la pista queda desarmada.
fn empujar_en(guard: &mut Option<ProductorPista>, samples: &[f32]) {
    let Some(p) = guard.as_mut() else { return };
    let inicio = p
        .offset
        .fetch_add(samples.len() as u64, std::sync::atomic::Ordering::SeqCst);
    match p.tx.try_send(Trozo {
        inicio,
        muestras: samples.to_vec(),
    }) {
        Ok(()) => {}
        Err(std::sync::mpsc::TrySendError::Full(_)) => {} // hueco: lo ve el worker
        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
            *guard = None;
        }
    }
}

/// Tap del micrófono (lo instala managers/audio como raw tap del recorder).
/// Sin pista armada es un no-op de un lock corto: el dictado normal pasa por
/// aquí y no debe pagar nada.
pub fn pista_mic(samples: &[f32]) {
    if let Ok(mut g) = PISTA_MIC.lock() {
        empujar_en(&mut g, samples);
    }
}

/// Tap del audio del sistema (lo llama el worker de Sesiones tras el read).
pub fn pista_sys(samples: &[f32]) {
    if let Ok(mut g) = PISTA_SYS.lock() {
        empujar_en(&mut g, samples);
    }
}

/// Siguiente nombre de segmento libre: una sesión recuperada no reabre su
/// contenedor (create_new), sigue en `mic-1`, `mic-2`…
fn siguiente_segmento(dir: &Path, pista: &str) -> PathBuf {
    for n in 0u32.. {
        let ruta = dir.join(format!("{pista}-{n}.escaud2"));
        if !ruta.exists() {
            return ruta;
        }
    }
    unreachable!("u32 agotado buscando segmento")
}

/// Núcleo del worker (testeable con writer y registro inyectados): rellena
/// huecos con silencio para que el tiempo jamás se comprima, y muere cortando
/// SU pista si el disco falla; el journal sigue por su lado.
fn correr_pista_nucleo(
    pista: &str,
    rx: std::sync::mpsc::Receiver<Trozo>,
    mut writer: crate::recording_crypto::Escaud2Writer,
    at_ms_base: u64,
    offset_final: std::sync::Arc<std::sync::atomic::AtomicU64>,
    registrar: &dyn Fn(EventoSesion),
) {
    let at_de = |muestras: u64| at_ms_base + muestras * 1000 / MUESTRAS_POR_SEGUNDO;
    let mut escritas: u64 = 0;
    let corte = |escritas: u64, e: &dyn std::fmt::Display| {
        warn!("Pista {pista} cortada: {e}");
        registrar(EventoSesion::Pista {
            pista: pista.to_string(),
            evento: "corte".to_string(),
            at_ms: at_de(escritas),
            muestras_perdidas: None,
        });
    };
    for trozo in rx {
        if trozo.inicio > escritas {
            let hueco = trozo.inicio - escritas;
            let silencio = vec![0f32; 16_384];
            let mut faltan = hueco;
            while faltan > 0 {
                let n = faltan.min(silencio.len() as u64) as usize;
                if let Err(e) = writer.append_samples(&silencio[..n]) {
                    corte(escritas, &e);
                    return;
                }
                faltan -= n as u64;
            }
            registrar(EventoSesion::Pista {
                pista: pista.to_string(),
                evento: "hueco".to_string(),
                at_ms: at_de(escritas),
                muestras_perdidas: Some(hueco),
            });
            escritas = trozo.inicio;
        }
        if let Err(e) = writer.append_samples(&trozo.muestras) {
            corte(escritas, &e);
            return;
        }
        escritas += trozo.muestras.len() as u64;
    }
    // Canal cerrado. Si el productor avanzó más de lo que llegó (descartes
    // al FINAL de la pista), ese tramo también es silencio + hueco: el
    // reloj compartido es la verdad, no el último trozo recibido.
    let producido = offset_final.load(std::sync::atomic::Ordering::SeqCst);
    if producido > escritas {
        let hueco = producido - escritas;
        let silencio = vec![0f32; 16_384];
        let mut faltan = hueco;
        while faltan > 0 {
            let n = faltan.min(silencio.len() as u64) as usize;
            if let Err(e) = writer.append_samples(&silencio[..n]) {
                corte(escritas, &e);
                return;
            }
            faltan -= n as u64;
        }
        registrar(EventoSesion::Pista {
            pista: pista.to_string(),
            evento: "hueco".to_string(),
            at_ms: at_de(escritas),
            muestras_perdidas: Some(hueco),
        });
        escritas = producido;
    }
    // Cierre limpio con footer.
    let final_at = at_de(escritas);
    if let Err(e) = writer.finalize() {
        corte(escritas, &e);
        return;
    }
    registrar(EventoSesion::Pista {
        pista: pista.to_string(),
        evento: "fin".to_string(),
        at_ms: final_at,
        muestras_perdidas: None,
    });
}

/// Arma una pista si hay journal activo y no estaba armada. `at_ms_base` es
/// el reloj de sesión en el momento del armado.
pub fn pista_armar(pista: &'static str, at_ms_base: u64) {
    let dir = {
        let Ok(guard) = ACTIVO.lock() else { return };
        let Some(g) = guard.as_ref() else { return };
        g.dir.clone()
    };
    let Ok(mut slot) = slot_de(pista).lock() else {
        return;
    };
    if slot.is_some() {
        return;
    }
    let ruta = siguiente_segmento(&dir, pista);
    let writer = match crate::recording_crypto::Escaud2Writer::create(&ruta) {
        Ok(w) => w,
        Err(e) => {
            // Fail-closed de la pista, no de la sesión: el journal sigue.
            warn!("Pista {pista} no pudo armarse: {e}");
            return;
        }
    };
    let (tx, rx) = std::sync::mpsc::sync_channel::<Trozo>(CAPACIDAD_CANAL);
    let offset = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let offset_worker = std::sync::Arc::clone(&offset);
    let handle = std::thread::Builder::new()
        .name(format!("escriba-pista-{pista}"))
        .spawn(move || {
            correr_pista_nucleo(
                pista,
                rx,
                writer,
                at_ms_base,
                offset_worker,
                &registrar_pista,
            )
        })
        .ok();
    apendear(&EventoSesion::Pista {
        pista: pista.to_string(),
        evento: "inicio".to_string(),
        at_ms: at_ms_base,
        muestras_perdidas: None,
    });
    *slot = Some(ProductorPista {
        tx,
        offset,
        worker: handle,
    });
}

/// Desarma una pista: cierra el canal y espera al worker (drena, finaliza y
/// deja el `fin` en el journal). Idempotente.
pub fn pista_desarmar(pista: &str) {
    let productor = {
        let Ok(mut slot) = slot_de(pista).lock() else {
            return;
        };
        slot.take()
    };
    if let Some(p) = productor {
        drop(p.tx);
        if let Some(h) = p.worker {
            let _ = h.join();
        }
    }
}

fn pistas_desarmar_todas() {
    pista_desarmar("mic");
    pista_desarmar("sys");
}

// ─────────────────────────── Recuperación (Fase 2) ───────────────────────────

/// Resumen de una sesión pendiente (journal sin `cierre`) para el diálogo de
/// recuperación. Nunca lleva rutas: el id es el único mango que ve la webview.
#[derive(Serialize, Clone, Type)]
pub struct ResumenPendiente {
    pub id: String,
    pub wall_ms: u64,
    pub modo: String,
    pub turnos: u32,
    pub duracion_ms: u64,
    pub tiene_documento: bool,
    /// El kill rompió la última línea: se recuperó todo lo anterior.
    pub cola_rota: bool,
    /// Hay al menos un segmento de audio (mic-N/sys-N) para reprocesar.
    pub tiene_audio: bool,
}

/// Descifrador real (llave del llavero). `None` = línea ilegible.
fn descifrador_real(valor: &str) -> Option<String> {
    match crate::history_crypto::leer_campo(valor) {
        crate::history_crypto::CampoLeido::Descifrado(s) => Some(s),
        _ => None,
    }
}

/// Un id de sesión válido es EXACTAMENTE 32 hex minúsculas (el formato que
/// produce `id_nuevo`). Todo lo demás se rechaza antes de mirar el disco:
/// sin puntos, sin barras, sin traversal posible por construcción.
fn id_valido(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Resuelve y valida el directorio de una sesión bajo `raiz`. Defensa en
/// profundidad sobre el formato del id: rechaza symlinks (el directorio y el
/// journal deben ser reales) y exige contención canónica bajo la raíz.
fn dir_validada_en(raiz: &Path, id: &str) -> Result<PathBuf, String> {
    if !id_valido(id) {
        return Err("id de sesión inválido".to_string());
    }
    let dir = raiz.join(id);
    let meta = fs::symlink_metadata(&dir).map_err(|_| "la sesión no existe".to_string())?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err("la sesión no es un directorio real".to_string());
    }
    let journal = dir.join("journal.jsonl");
    let jmeta = fs::symlink_metadata(&journal).map_err(|_| "sesión sin journal".to_string())?;
    if jmeta.file_type().is_symlink() || !jmeta.is_file() {
        return Err("el journal no es un archivo real".to_string());
    }
    let canon = dir
        .canonicalize()
        .map_err(|_| "ruta no canonicalizable".to_string())?;
    let raiz_canon = raiz
        .canonicalize()
        .map_err(|_| "raíz no canonicalizable".to_string())?;
    if !canon.starts_with(&raiz_canon) {
        return Err("la sesión queda fuera de la raíz".to_string());
    }
    Ok(dir)
}

fn dir_validada(id: &str) -> Result<PathBuf, String> {
    let raiz = RAIZ
        .get()
        .ok_or_else(|| "sin raíz de sesiones".to_string())?;
    dir_validada_en(raiz, id)
}

/// Resumen de un journal parseado; `None` si está cerrado (no se ofrece).
fn resumen_de_eventos(eventos: &[EventoSesion], cola_rota: bool) -> Option<ResumenPendiente> {
    if eventos
        .iter()
        .any(|e| matches!(e, EventoSesion::Cierre { .. }))
    {
        return None;
    }
    let (wall_ms, modo) = eventos.iter().find_map(|e| match e {
        EventoSesion::Inicio { wall_ms, modo, .. } => Some((*wall_ms, modo.clone())),
        _ => None,
    })?;
    let mut turnos = 0u32;
    let mut duracion_ms = 0u64;
    let mut tiene_documento = false;
    for e in eventos {
        match e {
            EventoSesion::Turno { at_ms, .. } => {
                turnos += 1;
                duracion_ms = duracion_ms.max(*at_ms);
            }
            EventoSesion::Documento { at_ms, .. } => {
                tiene_documento = true;
                duracion_ms = duracion_ms.max(*at_ms);
            }
            _ => {}
        }
    }
    Some(ResumenPendiente {
        id: String::new(), // lo pone el escaneo, que conoce el nombre real
        wall_ms,
        modo,
        turnos,
        duracion_ms,
        tiene_documento,
        cola_rota,
        tiene_audio: false, // lo pone el escaneo, que ve el directorio
    })
}

/// Lee y parsea el journal de una sesión validada.
fn cargar(id: &str) -> Result<(Vec<EventoSesion>, bool), String> {
    let dir = dir_validada(id)?;
    let contenido = fs::read_to_string(dir.join("journal.jsonl")).map_err(|e| e.to_string())?;
    Ok(parsear_journal(&contenido, &descifrador_real))
}

/// Escanea `sessions/` y devuelve las sesiones pendientes (sin `cierre`).
/// Entradas con nombre inválido, symlinks o journals ilegibles se saltan con
/// un warn: el escaneo jamás revienta el arranque.
pub fn listar_pendientes() -> Vec<ResumenPendiente> {
    let Some(raiz) = RAIZ.get() else {
        return Vec::new();
    };
    let Ok(entradas) = fs::read_dir(raiz) else {
        return Vec::new(); // sin carpeta sessions/ todavía: nada pendiente
    };
    let mut pendientes = Vec::new();
    for entrada in entradas.flatten() {
        let nombre = entrada.file_name().to_string_lossy().to_string();
        if dir_validada_en(raiz, &nombre).is_err() {
            continue;
        }
        let Ok(contenido) = fs::read_to_string(entrada.path().join("journal.jsonl")) else {
            warn!("Sesión {nombre}: journal ilegible, se salta");
            continue;
        };
        let (eventos, cola_rota) = parsear_journal(&contenido, &descifrador_real);
        if let Some(mut resumen) = resumen_de_eventos(&eventos, cola_rota) {
            resumen.tiene_audio = fs::read_dir(entrada.path())
                .map(|d| {
                    d.flatten()
                        .any(|f| f.file_name().to_string_lossy().ends_with(".escaud2"))
                })
                .unwrap_or(false);
            resumen.id = nombre;
            pendientes.push(resumen);
        }
    }
    // Más reciente primero: si hay varias, la de ayer va arriba.
    pendientes.sort_by_key(|p| std::cmp::Reverse(p.wall_ms));
    pendientes
}

/// Carga los eventos de una sesión pendiente para recuperarla (solo lectura).
pub fn cargar_pendiente(id: &str) -> Result<(Vec<EventoSesion>, bool), String> {
    cargar(id)
}

/// Sana las pistas de una sesión pendiente: trunca colas rotas por el kill.
/// Se llama desde el comando de recuperación, JAMÁS desde la captura.
pub fn sanar_pistas(id: &str) {
    let Some(raiz) = RAIZ.get() else { return };
    for ruta in segmentos_validados_en(raiz, id) {
        if let Err(e) = crate::recording_crypto::escaud2_recover(&ruta) {
            warn!("Pista {} no recuperable: {e}", ruta.display());
        }
    }
}

/// ¿Nombre estricto de segmento? `mic-0.escaud2`, `sys-12.escaud2`... y nada
/// más: ni symlinks con nombre bonito, ni archivos ajenos.
fn nombre_de_segmento_valido(nombre: &str) -> bool {
    let Some(resto) = nombre
        .strip_prefix("mic-")
        .or_else(|| nombre.strip_prefix("sys-"))
    else {
        return false;
    };
    let Some(indice) = resto.strip_suffix(".escaud2") else {
        return false;
    };
    !indice.is_empty() && indice.len() <= 6 && indice.bytes().all(|b| b.is_ascii_digit())
}

/// Segmentos de una sesión con las MISMAS garantías que el journal: id
/// validado, contención bajo la raíz, nombre estricto y archivo regular
/// (un symlink jamás es una pista; truncarlo seguiría el enlace).
fn segmentos_validados_en(raiz: &Path, id: &str) -> Vec<PathBuf> {
    let Ok(dir) = dir_validada_en(raiz, id) else {
        return Vec::new();
    };
    let Ok(entradas) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut rutas: Vec<PathBuf> = entradas
        .flatten()
        .filter(|f| {
            let nombre = f.file_name().to_string_lossy().to_string();
            nombre_de_segmento_valido(&nombre)
                && fs::symlink_metadata(f.path())
                    .map(|m| m.is_file() && !m.file_type().is_symlink())
                    .unwrap_or(false)
        })
        .map(|f| f.path())
        .collect();
    rutas.sort();
    rutas
}

/// Segmentos válidos de una sesión pendiente, para re-transcribir.
pub fn listar_segmentos(id: &str) -> Vec<PathBuf> {
    RAIZ.get()
        .map(|raiz| segmentos_validados_en(raiz, id))
        .unwrap_or_default()
}

/// Reengancha el journal de una sesión recuperada: los turnos nuevos siguen
/// apendándose al MISMO archivo. Si ya hay un grabador activo, no pisa nada.
pub fn reanudar(id: &str) -> Result<(), String> {
    let dir = dir_validada(id)?;
    let mut guard = ACTIVO.lock().map_err(|_| "lock envenenado".to_string())?;
    if guard.is_some() {
        return Err("ya hay un journal activo".to_string());
    }
    let archivo = OpenOptions::new()
        .append(true)
        .open(dir.join("journal.jsonl"))
        .map_err(|e| e.to_string())?;
    info!("Journal de sesión reenganchado: {id}");
    *guard = Some(Grabador { dir, archivo });
    Ok(())
}

/// Descarta una sesión pendiente: borra su carpeta entera. Se niega si esa
/// carpeta es la del journal ACTIVO (para eso está el reset).
pub fn descartar_pendiente(id: &str) -> Result<(), String> {
    let dir = dir_validada(id)?;
    if let Ok(guard) = ACTIVO.lock() {
        if guard.as_ref().is_some_and(|g| g.dir == dir) {
            return Err("esa sesión está activa; descártala desde la sesión".to_string());
        }
    }
    info!(
        "Sesión pendiente descartada desde recuperación: {}",
        dir.display()
    );
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())
}

/// Confirmación durable de una sesión pendiente (el usuario exportó el acta
/// desde el diálogo de recuperación): apendea `cierre{documento}` sin tocar
/// nada más. Idempotente: si ya está cerrada, no hace nada.
pub fn confirmar_pendiente(id: &str) -> Result<(), String> {
    let raiz = RAIZ
        .get()
        .ok_or_else(|| "sin raíz de sesiones".to_string())?;
    let dir = dir_validada_en(raiz, id)?;
    if let Ok(guard) = ACTIVO.lock() {
        if guard.as_ref().is_some_and(|g| g.dir == dir) {
            return Err("esa sesión está activa; confírmala desde la sesión".to_string());
        }
    }
    confirmar_pendiente_con(raiz, id, &cifrador_real, &descifrador_real)
}

/// Núcleo de la confirmación con raíz y cifradores inyectables (testeable).
fn confirmar_pendiente_con(
    raiz: &Path,
    id: &str,
    cifrador: &dyn Fn(&str) -> Result<String, String>,
    descifrador: &dyn Fn(&str) -> Option<String>,
) -> Result<(), String> {
    let dir = dir_validada_en(raiz, id)?;
    let ruta = dir.join("journal.jsonl");
    let contenido = fs::read_to_string(&ruta).map_err(|e| e.to_string())?;
    let (eventos, _) = parsear_journal(&contenido, descifrador);
    if eventos
        .iter()
        .any(|e| matches!(e, EventoSesion::Cierre { .. }))
    {
        return Ok(()); // ya cerrada: re-confirmar no duplica nada
    }
    let linea = linea_de_evento(
        &EventoSesion::Cierre {
            motivo: "documento".to_string(),
        },
        cifrador,
    )?;
    let mut archivo = OpenOptions::new()
        .append(true)
        .open(&ruta)
        .map_err(|e| e.to_string())?;
    archivo
        .write_all(linea.as_bytes())
        .and_then(|()| archivo.write_all(b"\n"))
        .and_then(|()| archivo.sync_data())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_crypto::{cifrar_con_estricto, leer_con_llave};

    const LLAVE: [u8; 32] = [7u8; 32];

    fn cifra(texto: &str) -> Result<String, String> {
        cifrar_con_estricto(&LLAVE, texto)
    }

    fn descifra(valor: &str) -> Option<String> {
        leer_con_llave(&LLAVE, valor)
    }

    fn journal_de(eventos: &[EventoSesion]) -> String {
        eventos
            .iter()
            .map(|e| linea_de_evento(e, &|t| cifra(t)).unwrap() + "\n")
            .collect()
    }

    fn eventos_demo() -> Vec<EventoSesion> {
        vec![
            EventoSesion::Inicio {
                wall_ms: 1_756_500_000_000,
                modo: "listen".into(),
                version: VERSION,
            },
            EventoSesion::Turno {
                role: "user".into(),
                text: "hola, arranquemos la reunión".into(),
                at_ms: 2_000,
            },
            EventoSesion::Turno {
                role: "system".into(),
                text: "perfecto, te escucho".into(),
                at_ms: 9_000,
            },
        ]
    }

    #[test]
    fn round_trip_completo_y_nada_en_claro() {
        let contenido = journal_de(&eventos_demo());
        // Ni una palabra del contenido en claro en el archivo.
        assert!(!contenido.contains("reunión"));
        assert!(!contenido.contains("listen"));
        assert!(contenido.lines().all(|l| l.starts_with("esc1:")));

        let (eventos, cola_rota) = parsear_journal(&contenido, &descifra);
        assert_eq!(eventos, eventos_demo());
        assert!(!cola_rota);
    }

    #[test]
    fn cola_rota_por_kill_no_impide_recuperar_lo_anterior() {
        let mut contenido = journal_de(&eventos_demo());
        // Kill a mitad del write de la última línea: queda un trozo de base64.
        contenido.truncate(contenido.len() - 25);
        let (eventos, cola_rota) = parsear_journal(&contenido, &descifra);
        assert_eq!(eventos.len(), 2, "se recuperan todas menos la rota");
        assert!(cola_rota);
    }

    #[test]
    fn linea_corrupta_en_el_medio_corta_pero_conserva_lo_previo() {
        let lineas: Vec<String> = journal_de(&eventos_demo())
            .lines()
            .map(str::to_string)
            .collect();
        let contenido = format!("{}\nesc1:basura\n{}\n", lineas[0], lineas[2]);
        let (eventos, cola_rota) = parsear_journal(&contenido, &descifra);
        assert_eq!(eventos.len(), 1, "solo lo anterior a la corrupción");
        assert!(cola_rota);
    }

    #[test]
    fn documento_antes_del_cierre_se_recupera() {
        let mut eventos = eventos_demo();
        eventos.push(EventoSesion::Documento {
            doc: "## Acta\n- acuerdo uno".into(),
            animo: "positivo".into(),
            at_ms: 60_000,
        });
        // SIN cierre: el crash llegó después del acta, antes de guardarla.
        let contenido = journal_de(&eventos);
        let (leidos, _) = parsear_journal(&contenido, &descifra);
        let doc = leidos.iter().find_map(|e| match e {
            EventoSesion::Documento { doc, .. } => Some(doc.clone()),
            _ => None,
        });
        assert_eq!(doc.as_deref(), Some("## Acta\n- acuerdo uno"));
        let cerrado = leidos
            .iter()
            .any(|e| matches!(e, EventoSesion::Cierre { .. }));
        assert!(!cerrado, "sin cierre: la recuperación debe ofrecerla");
    }

    #[test]
    fn los_mm_ss_no_dependen_de_la_hora_de_pared() {
        // El mismo at_ms reconstruye el mismo mm:ss aunque el wall del inicio
        // sea absurdo: el reloj de pared solo etiqueta, nunca ordena.
        for wall in [0u64, 1_756_500_000_000, u64::MAX / 2] {
            let contenido = journal_de(&[
                EventoSesion::Inicio {
                    wall_ms: wall,
                    modo: "listen".into(),
                    version: VERSION,
                },
                EventoSesion::Turno {
                    role: "user".into(),
                    text: "x".into(),
                    at_ms: 83_000,
                },
            ]);
            let (eventos, _) = parsear_journal(&contenido, &descifra);
            let at = eventos.iter().find_map(|e| match e {
                EventoSesion::Turno { at_ms, .. } => Some(*at_ms),
                _ => None,
            });
            assert_eq!(at, Some(83_000)); // 01:23 siempre
        }
    }

    #[test]
    fn cifrador_que_falla_no_produce_ninguna_linea() {
        // El contrato todo-o-nada de `arrancar` se apoya en que la PRIMERA
        // línea que falle aborta el lote entero antes de tocar disco.
        let fallo = |_: &str| -> Result<String, String> { Err("sin llave".into()) };
        let r = linea_de_evento(
            &EventoSesion::Inicio {
                wall_ms: 0,
                modo: "listen".into(),
                version: VERSION,
            },
            &fallo,
        );
        assert!(r.is_err());
    }

    #[test]
    fn fallo_parcial_del_cifrado_deja_cero_archivos() {
        // El fallo llega en el SEGUNDO evento del lote, no en el primero: el
        // contrato todo-o-nada tiene que abortar igual, sin crear ni la raíz.
        let raiz = std::env::temp_dir().join(format!("escriba-fc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let llamadas = std::cell::Cell::new(0u32);
        let cifrador = |t: &str| -> Result<String, String> {
            llamadas.set(llamadas.get() + 1);
            if llamadas.get() >= 2 {
                return Err("llavero caído a mitad del lote".into());
            }
            cifra(t)
        };
        let r = arrancar_nucleo(
            &raiz,
            "aabbccddeeff00112233445566778899",
            "listen",
            0,
            &[("user".into(), "turno previo".into(), 1000)],
            &cifrador,
        );
        assert!(matches!(r, Err(FalloArranque::Cifrado(_))));
        assert!(
            !raiz.exists(),
            "un fallo parcial del cifrado no puede dejar NADA en disco"
        );
    }

    #[test]
    fn arranque_feliz_escribe_un_journal_recuperable() {
        let raiz = std::env::temp_dir().join(format!("escriba-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let id = "00112233445566778899aabbccddeeff";
        let r = arrancar_nucleo(
            &raiz,
            id,
            "listen",
            77,
            &[("user".into(), "hola desde el replay".into(), 2000)],
            &|t| cifra(t),
        );
        assert!(r.is_ok());
        let contenido = std::fs::read_to_string(raiz.join(id).join("journal.jsonl")).unwrap();
        assert!(contenido.lines().all(|l| l.starts_with("esc1:")));
        assert!(!contenido.contains("replay"), "nada en claro");
        let (eventos, cola_rota) = parsear_journal(&contenido, &descifra);
        assert!(!cola_rota);
        assert_eq!(eventos.len(), 2, "inicio + turno del replay");
        assert!(matches!(
            &eventos[0],
            EventoSesion::Inicio { wall_ms: 77, .. }
        ));
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn fallo_de_disco_no_deja_carpeta_a_medias() {
        // La "raíz" es un ARCHIVO: create_dir_all revienta y el núcleo debe
        // reportar Disco sin inventar estructura alrededor.
        let raiz = std::env::temp_dir().join(format!("escriba-file-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        std::fs::write(&raiz, b"soy un archivo, no un directorio").unwrap();
        let r = arrancar_nucleo(
            &raiz,
            "ffeeddccbbaa99887766554433221100",
            "listen",
            0,
            &[],
            &|t| cifra(t),
        );
        assert!(matches!(r, Err(FalloArranque::Disco(_))));
        let _ = std::fs::remove_file(&raiz);
    }

    #[test]
    fn ids_hostiles_se_rechazan_antes_de_mirar_el_disco() {
        let raiz = std::env::temp_dir().join(format!("escriba-val-{}", std::process::id()));
        std::fs::create_dir_all(&raiz).unwrap();
        for hostil in [
            "..",
            "../..",
            "../../etc/passwd",
            "a/../b",
            "AABBCCDDEEFF00112233445566778899",  // mayúsculas
            "0011223344556677",                  // corto
            "00112233445566778899aabbccddeeff0", // largo
            "0011223344556677 899aabbccddeeff",  // espacio
            "",
        ] {
            assert!(
                dir_validada_en(&raiz, hostil).is_err(),
                "id hostil aceptado: {hostil:?}"
            );
        }
        // Formato válido pero inexistente: también Err, sin crear nada.
        assert!(dir_validada_en(&raiz, "00112233445566778899aabbccddeeff").is_err());
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn un_symlink_con_nombre_valido_se_rechaza() {
        let base = std::env::temp_dir().join(format!("escriba-sym-{}", std::process::id()));
        let raiz = base.join("sessions");
        let fuera = base.join("fuera");
        std::fs::create_dir_all(&raiz).unwrap();
        std::fs::create_dir_all(fuera.join("x")).unwrap();
        std::fs::write(fuera.join("journal.jsonl"), b"x").unwrap();
        // Directorio-symlink apuntando FUERA de la raíz, con nombre válido.
        let id = "aaaabbbbccccddddeeeeffff00001111";
        std::os::unix::fs::symlink(&fuera, raiz.join(id)).unwrap();
        assert!(
            dir_validada_en(&raiz, id).is_err(),
            "un symlink jamás es una sesión"
        );
        // Y un journal-symlink dentro de un directorio real, igual de fuera.
        let id2 = "bbbbccccddddeeeeffff000011112222";
        std::fs::create_dir_all(raiz.join(id2)).unwrap();
        std::os::unix::fs::symlink(
            fuera.join("journal.jsonl"),
            raiz.join(id2).join("journal.jsonl"),
        )
        .unwrap();
        assert!(dir_validada_en(&raiz, id2).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn el_resumen_ignora_sesiones_cerradas_y_reporta_el_acta() {
        let mut eventos = eventos_demo();
        assert!(resumen_de_eventos(&eventos, false).is_some());

        eventos.push(EventoSesion::Documento {
            doc: "acta".into(),
            animo: "neutral".into(),
            at_ms: 30_000,
        });
        let r = resumen_de_eventos(&eventos, true).unwrap();
        assert!(r.tiene_documento);
        assert!(r.cola_rota);
        assert_eq!(r.turnos, 2);
        assert_eq!(r.duracion_ms, 30_000);

        eventos.push(EventoSesion::Cierre {
            motivo: "documento".into(),
        });
        assert!(
            resumen_de_eventos(&eventos, false).is_none(),
            "cerrada: no se ofrece"
        );
    }

    #[test]
    fn confirmar_es_idempotente_y_no_duplica_el_cierre() {
        let raiz = std::env::temp_dir().join(format!("escriba-conf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let id = "cafecafecafecafecafecafecafecafe";
        arrancar_nucleo(&raiz, id, "listen", 5, &[], &|t| cifra(t)).unwrap();

        confirmar_pendiente_con(&raiz, id, &|t| cifra(t), &descifra).unwrap();
        let contenido = std::fs::read_to_string(raiz.join(id).join("journal.jsonl")).unwrap();
        let lineas_tras_primera = contenido.lines().count();
        let (eventos, _) = parsear_journal(&contenido, &descifra);
        assert!(eventos
            .iter()
            .any(|e| matches!(e, EventoSesion::Cierre { .. })));

        // Segunda confirmación: mismo estado, ni una línea más.
        confirmar_pendiente_con(&raiz, id, &|t| cifra(t), &descifra).unwrap();
        let contenido2 = std::fs::read_to_string(raiz.join(id).join("journal.jsonl")).unwrap();
        assert_eq!(contenido2.lines().count(), lineas_tras_primera);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn el_offset_avanza_tambien_al_descartar() {
        // Canal de capacidad 1 y nadie drenando: el segundo y tercer empuje
        // se descartan, pero el reloj de la pista sigue avanzando. Ese es el
        // contrato que impide comprimir el tiempo.
        let (tx, _rx) = std::sync::mpsc::sync_channel::<Trozo>(1);
        let mut slot = Some(ProductorPista {
            tx,
            offset: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            worker: None,
        });
        for _ in 0..3 {
            empujar_en(&mut slot, &[0.5f32; 1000]);
        }
        assert_eq!(
            slot.as_ref()
                .unwrap()
                .offset
                .load(std::sync::atomic::Ordering::SeqCst),
            3000
        );
    }

    #[test]
    fn el_worker_muerto_desarma_la_pista_sin_bloquear() {
        let (tx, rx) = std::sync::mpsc::sync_channel::<Trozo>(1);
        drop(rx); // el worker murió
        let mut slot = Some(ProductorPista {
            tx,
            offset: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            worker: None,
        });
        empujar_en(&mut slot, &[0.1f32; 10]);
        assert!(slot.is_none(), "Disconnected desarma el productor");
    }

    #[test]
    fn el_worker_rellena_huecos_con_silencio_y_los_deja_escritos() {
        let dir = std::env::temp_dir().join(format!("escriba-pista-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("mic-0.escaud2");
        let writer =
            crate::recording_crypto::Escaud2Writer::create_with_key(&ruta, &LLAVE).unwrap();

        let eventos = Mutex::new(Vec::new());
        let (tx, rx) = std::sync::mpsc::sync_channel::<Trozo>(8);
        // Trozo en 0, HUECO de 2000 muestras (descartadas), trozo en 3000.
        tx.send(Trozo {
            inicio: 0,
            muestras: vec![0.25f32; 1000],
        })
        .unwrap();
        tx.send(Trozo {
            inicio: 3000,
            muestras: vec![-0.25f32; 500],
        })
        .unwrap();
        drop(tx);
        correr_pista_nucleo(
            "mic",
            rx,
            writer,
            10_000,
            std::sync::Arc::new(std::sync::atomic::AtomicU64::new(3500)),
            &|e| {
                eventos.lock().unwrap().push(e);
            },
        );

        let leidas = crate::recording_crypto::escaud2_read_samples_with_key(&ruta, &LLAVE).unwrap();
        assert_eq!(leidas.len(), 3500, "1000 + 2000 de silencio + 500");
        assert!(leidas[..1000].iter().all(|s| *s > 0.2));
        assert!(
            leidas[1000..3000].iter().all(|s| *s == 0.0),
            "el hueco es silencio, no tiempo comprimido"
        );
        assert!(leidas[3000..].iter().all(|s| *s < -0.2));

        let ev = eventos.lock().unwrap();
        let hueco = ev.iter().find_map(|e| match e {
            EventoSesion::Pista {
                evento,
                muestras_perdidas,
                at_ms,
                ..
            } if evento == "hueco" => Some((*muestras_perdidas, *at_ms)),
            _ => None,
        });
        // El hueco arranca en la muestra 1000: 10_000 ms de base + 62 ms.
        assert_eq!(hueco, Some((Some(2000), 10_062)));
        assert!(ev
            .iter()
            .any(|e| matches!(e, EventoSesion::Pista { evento, .. } if evento == "fin")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn saturacion_seguida_de_cierre_no_pierde_el_hueco_final() {
        // El caso de la revisión del 30-ago: el canal se llena, se descartan
        // trozos, y la pista se desarma ANTES de que llegue otro trozo que
        // delate el salto. El reloj compartido convierte ese tramo en
        // silencio + hueco igualmente.
        let dir = std::env::temp_dir().join(format!("escriba-satur-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("mic-0.escaud2");
        let writer =
            crate::recording_crypto::Escaud2Writer::create_with_key(&ruta, &LLAVE).unwrap();

        let offset = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
        let (tx, rx) = std::sync::mpsc::sync_channel::<Trozo>(1);
        let mut slot = Some(ProductorPista {
            tx,
            offset: std::sync::Arc::clone(&offset),
            worker: None,
        });
        // Tres empujes contra un canal de 1: entra el primero, se descartan
        // dos. Y cierre inmediato, sin ningún trozo posterior.
        for _ in 0..3 {
            empujar_en(&mut slot, &[0.5f32; 1000]);
        }
        drop(slot); // pista_desarmar: cae el productor y se cierra el canal

        let eventos = Mutex::new(Vec::new());
        correr_pista_nucleo("mic", rx, writer, 0, offset, &|e| {
            eventos.lock().unwrap().push(e);
        });

        let leidas = crate::recording_crypto::escaud2_read_samples_with_key(&ruta, &LLAVE).unwrap();
        assert_eq!(
            leidas.len(),
            3000,
            "1000 recibidas + 2000 de silencio final"
        );
        assert!(leidas[1000..].iter().all(|s| *s == 0.0));
        let ev = eventos.lock().unwrap();
        let hueco = ev.iter().find_map(|e| match e {
            EventoSesion::Pista {
                evento,
                muestras_perdidas,
                ..
            } if evento == "hueco" => Some(*muestras_perdidas),
            _ => None,
        });
        assert_eq!(hueco, Some(Some(2000)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sanar_ignora_symlinks_y_nombres_ajenos() {
        assert!(nombre_de_segmento_valido("mic-0.escaud2"));
        assert!(nombre_de_segmento_valido("sys-12.escaud2"));
        for malo in [
            "mic-.escaud2",
            "mic-0.escaudio",
            "otro-0.escaud2",
            "mic-0.escaud2.evil",
            "mic-0000000.escaud2", // índice absurdo
            "journal.jsonl",
            "..",
        ] {
            assert!(!nombre_de_segmento_valido(malo), "aceptado: {malo}");
        }

        // Y sobre disco: un symlink con nombre PERFECTO no entra a la lista.
        let base = std::env::temp_dir().join(format!("escriba-sanar-{}", std::process::id()));
        let raiz = base.join("sessions");
        let id = "feedfacefeedfacefeedfacefeedface";
        let dir = raiz.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("journal.jsonl"), b"x").unwrap();
        let fuera = base.join("victima.escaud2");
        std::fs::write(&fuera, b"contenido ajeno").unwrap();
        std::os::unix::fs::symlink(&fuera, dir.join("mic-0.escaud2")).unwrap();
        std::fs::write(dir.join("sys-0.escaud2"), b"real").unwrap();

        let rutas = segmentos_validados_en(&raiz, id);
        assert_eq!(rutas.len(), 1, "solo el archivo regular");
        assert!(rutas[0].ends_with("sys-0.escaud2"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn los_segmentos_no_pisan_al_anterior() {
        let dir = std::env::temp_dir().join(format!("escriba-seg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(siguiente_segmento(&dir, "mic").ends_with("mic-0.escaud2"));
        std::fs::write(dir.join("mic-0.escaud2"), b"x").unwrap();
        std::fs::write(dir.join("mic-1.escaud2"), b"x").unwrap();
        assert!(
            siguiente_segmento(&dir, "mic").ends_with("mic-2.escaud2"),
            "una sesión recuperada sigue en el segmento libre"
        );
        assert!(siguiente_segmento(&dir, "sys").ends_with("sys-0.escaud2"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn costo_de_fsync_por_turno_es_asumible() {
        // Medición pedida por la Fase 1: escribir 50 eventos con fsync cada
        // uno. A ritmo humano (un turno cada varios segundos) esto tiene que
        // ser ruido; si un disco lo vuelve caro, el N de fsync se revisa.
        let dir = std::env::temp_dir().join(format!("escriba-fsync-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ruta = dir.join("journal.jsonl");
        let mut archivo = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&ruta)
            .unwrap();
        let linea = journal_de(&eventos_demo()[..1]);
        let inicio = std::time::Instant::now();
        for _ in 0..50 {
            archivo.write_all(linea.as_bytes()).unwrap();
            archivo.sync_data().unwrap();
        }
        let por_evento = inicio.elapsed() / 50;
        let _ = std::fs::remove_dir_all(&dir);
        // Umbral holgado: 50 ms por evento ya sería un disco enfermo.
        assert!(
            por_evento.as_millis() < 50,
            "fsync por turno tardó {por_evento:?}"
        );
        println!("fsync por evento: {por_evento:?}");
    }
}
