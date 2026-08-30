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
    apendear(&EventoSesion::Cierre {
        motivo: "documento".to_string(),
    });
    let _ = ACTIVO.lock().map(|mut g| *g = None);
}

/// Descarte explícito del usuario (reset): el journal y el directorio se
/// eliminan ya. No es la retención de la Fase 5: es la voluntad del usuario.
pub fn cierre_descarte() {
    let dir = {
        let mut guard = match ACTIVO.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        guard.take().map(|g| g.dir)
    };
    if let Some(dir) = dir {
        if let Err(e) = fs::remove_dir_all(&dir) {
            warn!("No se pudo borrar la sesión descartada: {e}");
        }
    }
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
            resumen.id = nombre;
            pendientes.push(resumen);
        }
    }
    // Más reciente primero: si hay varias, la de ayer va arriba.
    pendientes.sort_by(|a, b| b.wall_ms.cmp(&a.wall_ms));
    pendientes
}

/// Carga los eventos de una sesión pendiente para recuperarla (solo lectura).
pub fn cargar_pendiente(id: &str) -> Result<(Vec<EventoSesion>, bool), String> {
    cargar(id)
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
