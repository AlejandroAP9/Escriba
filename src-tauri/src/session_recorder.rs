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

/// Arranca el journal de la sesión si no hay uno activo. Todo-o-nada: las
/// líneas de `inicio` + turnos previos se cifran ANTES de crear nada en
/// disco; si el cifrado estricto no responde, no existe ni el directorio.
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

    // 1) Cifrar TODO antes de tocar el disco.
    let mut lineas = Vec::with_capacity(1 + turnos_previos.len());
    let inicio = EventoSesion::Inicio {
        wall_ms: wall_ms_inicio,
        modo: modo.to_string(),
        version: VERSION,
    };
    let todo_cifrado = (|| -> Result<(), String> {
        lineas.push(linea_de_evento(&inicio, &cifrador_real)?);
        for (role, text, at_ms) in turnos_previos {
            lineas.push(linea_de_evento(
                &EventoSesion::Turno {
                    role: role.clone(),
                    text: text.clone(),
                    at_ms: *at_ms,
                },
                &cifrador_real,
            )?);
        }
        Ok(())
    })();
    if let Err(e) = todo_cifrado {
        if AVISADO_SIN_CIFRADO.set(()).is_ok() {
            warn!("Journal de sesión desactivado (fail-closed): {e}. La sesión sigue en RAM.");
        }
        return;
    }

    // 2) Recién ahora, disco.
    let id = match id_nuevo() {
        Ok(id) => id,
        Err(e) => {
            warn!("Journal de sesión sin id: {e}");
            return;
        }
    };
    let dir = raiz.join(&id);
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
        Ok(archivo) => {
            info!("Journal de sesión activo: {id}");
            *guard = Some(Grabador { dir, archivo });
        }
        Err(e) => {
            warn!("Journal de sesión no pudo crearse: {e}");
            // Sin directorio a medias: si algo quedó, fuera.
            let _ = fs::remove_dir_all(&dir);
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

/// Cierre por documento durable: el acta ya está en el journal.
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
