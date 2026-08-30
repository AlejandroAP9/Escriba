//! Cifrado en reposo del historial (PRP-006, Fase 7).
//!
//! Cifrado POR CAMPO con prefijo de formato `esc1:` en las columnas
//! existentes: cero cambio de esquema, así una 2.2.4 abre la base y arranca
//! (el invariante de downgrade de history.rs:244 se pagó con un bucle de
//! crashes real, e59158d3, y aquí no se toca). El prefijo es a la vez el
//! marcador de migración: fila sin prefijo → se cifra al arrancar; con
//! prefijo → no se toca. Idempotente y re-ejecutable.
//!
//! La llave (256 bits) vive en el llavero del SO y JAMÁS se loguea. Sin
//! llave disponible (llavero borrado o inaccesible): la app arranca, el
//! dictado nuevo funciona, y lo ilegible se marca como no descifrable.
//! Fail-open para la función principal, fail-closed para los datos.
//!
//! `usage_daily` queda en claro a propósito: son números agregados (conteos
//! por día), no contenido.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use log::{error, warn};
use std::sync::OnceLock;

/// Prefijo de formato de un campo cifrado. Versionado por si algún día hay
/// un `esc2:`: el descifrado decide por prefijo, nunca por adivinanza.
pub const PREFIJO: &str = "esc1:";

const SERVICIO_LLAVERO: &str = "Escriba";
const CUENTA_LLAVERO: &str = "history-key-v1";
const LARGO_NONCE: usize = 12;

/// La llave se resuelve UNA vez por proceso. `None` = llavero inaccesible:
/// se degrada a texto claro (nuevo) y marcador (viejo cifrado), sin crash.
static LLAVE: OnceLock<Option<[u8; 32]>> = OnceLock::new();

/// Llave maestra del historial. Solo los módulos de cifrado pueden verla;
/// nunca cruza a comandos, frontend ni logs.
pub(crate) fn llave() -> Option<&'static [u8; 32]> {
    LLAVE
        .get_or_init(|| {
            let entrada = match keyring::Entry::new(SERVICIO_LLAVERO, CUENTA_LLAVERO) {
                Ok(e) => e,
                Err(e) => {
                    error!("Llavero inaccesible; historial sin cifrado nuevo: {}", e);
                    return None;
                }
            };
            match entrada.get_password() {
                Ok(hex) => match decodificar_hex(&hex) {
                    Some(k) => Some(k),
                    None => {
                        // Llave corrupta en el llavero: no se pisa (podría ser
                        // de otra instalación); se opera sin cifrado nuevo.
                        error!("Llave del llavero con formato inválido; se ignora");
                        None
                    }
                },
                Err(keyring::Error::NoEntry) => {
                    let mut k = [0u8; 32];
                    if getrandom::getrandom(&mut k).is_err() {
                        error!("Sin CSPRNG para crear la llave del historial");
                        return None;
                    }
                    let hex: String = k.iter().map(|b| format!("{:02x}", b)).collect();
                    match entrada.set_password(&hex) {
                        Ok(()) => Some(k),
                        Err(e) => {
                            // Sin persistencia no hay cifrado: cifrar con una
                            // llave efímera sería perder el historial al
                            // reiniciar.
                            error!("No se pudo guardar la llave en el llavero: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    warn!("Llavero no responde; historial sin cifrado nuevo: {}", e);
                    None
                }
            }
        })
        .as_ref()
}

fn decodificar_hex(hex: &str) -> Option<[u8; 32]> {
    let limpio = hex.trim();
    if limpio.len() != 64 {
        return None;
    }
    let mut k = [0u8; 32];
    for i in 0..32 {
        k[i] = u8::from_str_radix(limpio.get(i * 2..i * 2 + 2)?, 16).ok()?;
    }
    Some(k)
}

/// ¿Hay llave disponible en este proceso?
pub fn cifrado_disponible() -> bool {
    llave().is_some()
}

/// Cifra un campo de texto. Sin llave, devuelve el texto tal cual (fail-open
/// para que el dictado nunca se pierda). Un campo ya cifrado no se re-cifra.
pub fn cifrar_campo(texto: &str) -> String {
    let Some(k) = llave() else {
        return texto.to_string();
    };
    cifrar_con(k, texto)
}

/// Núcleo de cifrado con llave explícita (testeable sin tocar el llavero).
fn cifrar_con(k: &[u8; 32], texto: &str) -> String {
    if texto.is_empty() || texto.starts_with(PREFIJO) {
        return texto.to_string();
    }
    let cifra = ChaCha20Poly1305::new(Key::from_slice(k));
    let mut nonce = [0u8; LARGO_NONCE];
    if getrandom::getrandom(&mut nonce).is_err() {
        // Sin nonce aleatorio no se cifra: repetir nonce con la misma llave
        // rompe la confidencialidad. Mejor claro que mal cifrado.
        error!("Sin CSPRNG para el nonce; el campo queda en claro");
        return texto.to_string();
    }
    match cifra.encrypt(Nonce::from_slice(&nonce), texto.as_bytes()) {
        Ok(ct) => {
            let mut cuerpo = Vec::with_capacity(LARGO_NONCE + ct.len());
            cuerpo.extend_from_slice(&nonce);
            cuerpo.extend_from_slice(&ct);
            format!("{}{}", PREFIJO, B64.encode(cuerpo))
        }
        Err(_) => {
            // El AEAD solo falla por límites absurdos de tamaño; no se pierde
            // el dictado por eso.
            error!("Cifrado del campo falló; queda en claro");
            texto.to_string()
        }
    }
}

/// Cifra SIN degradar jamás a claro. Para el journal de sesiones (PRP-009):
/// su contrato es fail-closed todo-o-nada, el opuesto exacto del historial
/// de dictado (donde `cifrar_campo` degrada a propósito para no perder
/// texto). No unificar: dos contratos, dos funciones.
pub fn cifrar_campo_estricto(texto: &str) -> Result<String, String> {
    let Some(k) = llave() else {
        return Err("cifrado no disponible: sin llave del llavero".to_string());
    };
    cifrar_con_estricto(k, texto)
}

/// Núcleo estricto con llave explícita (testeable sin tocar el llavero).
pub(crate) fn cifrar_con_estricto(k: &[u8; 32], texto: &str) -> Result<String, String> {
    let cifra = ChaCha20Poly1305::new(Key::from_slice(k));
    let mut nonce = [0u8; LARGO_NONCE];
    getrandom::getrandom(&mut nonce)
        .map_err(|_| "sin CSPRNG para el nonce: no se cifra".to_string())?;
    let ct = cifra
        .encrypt(Nonce::from_slice(&nonce), texto.as_bytes())
        .map_err(|_| "el AEAD rechazó el cifrado".to_string())?;
    let mut cuerpo = Vec::with_capacity(LARGO_NONCE + ct.len());
    cuerpo.extend_from_slice(&nonce);
    cuerpo.extend_from_slice(&ct);
    Ok(format!("{}{}", PREFIJO, B64.encode(cuerpo)))
}

/// Descifrado con llave explícita para los tests del journal (la
/// recuperación real pasa por `leer_campo`): Some solo si descifra limpio.
#[cfg(test)]
pub(crate) fn leer_con_llave(k: &[u8; 32], valor: &str) -> Option<String> {
    match leer_con(k, valor) {
        CampoLeido::Descifrado(s) => Some(s),
        _ => None,
    }
}

/// Resultado de descifrar un campo leído de la base.
pub enum CampoLeido {
    /// Texto plano heredado (aún sin migrar) o campo vacío.
    Claro(String),
    /// Cifrado y descifrado correctamente.
    Descifrado(String),
    /// Cifrado pero ilegible (sin llave, o corrupto): NO se pierde la fila,
    /// se muestra el marcador y el usuario decide si purgar.
    NoDescifrable,
}

/// Descifra un campo si trae el prefijo; el texto claro pasa intacto.
pub fn leer_campo(valor: &str) -> CampoLeido {
    if !valor.starts_with(PREFIJO) {
        return CampoLeido::Claro(valor.to_string());
    }
    let Some(k) = llave() else {
        return CampoLeido::NoDescifrable;
    };
    leer_con(k, valor)
}

/// Núcleo de descifrado con llave explícita (testeable sin tocar el llavero).
fn leer_con(k: &[u8; 32], valor: &str) -> CampoLeido {
    let Some(cuerpo_b64) = valor.strip_prefix(PREFIJO) else {
        return CampoLeido::Claro(valor.to_string());
    };
    let Ok(cuerpo) = B64.decode(cuerpo_b64) else {
        return CampoLeido::NoDescifrable;
    };
    if cuerpo.len() <= LARGO_NONCE {
        return CampoLeido::NoDescifrable;
    }
    let (nonce, ct) = cuerpo.split_at(LARGO_NONCE);
    let cifra = ChaCha20Poly1305::new(Key::from_slice(k));
    match cifra.decrypt(Nonce::from_slice(nonce), ct) {
        Ok(plano) => match String::from_utf8(plano) {
            Ok(s) => CampoLeido::Descifrado(s),
            Err(_) => CampoLeido::NoDescifrable,
        },
        Err(_) => CampoLeido::NoDescifrable,
    }
}

/// Marcador visible para una entrada que no se pudo descifrar. El frontend lo
/// muestra tal cual; borrar la entrada (acción que ya existe) es la purga.
pub const MARCADOR_NO_DESCIFRABLE: &str =
    "⟨no descifrable: la llave del historial no está disponible⟩";

/// Texto listo para la UI: claro y descifrado pasan; lo ilegible se marca.
pub fn campo_para_ui(valor: &str) -> String {
    match leer_campo(valor) {
        CampoLeido::Claro(s) | CampoLeido::Descifrado(s) => s,
        CampoLeido::NoDescifrable => MARCADOR_NO_DESCIFRABLE.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // IMPORTANTE: los tests JAMÁS tocan el llavero real (crearían la llave de
    // producción desde el binario de test y sembrarían prompts de Keychain,
    // el riesgo del premortem "llavero atado a la firma"). Todo corre con una
    // llave inyectada por los núcleos `cifrar_con`/`leer_con`; `leer_campo`
    // solo se prueba por el camino sin prefijo, que no resuelve llave.

    const K: [u8; 32] = [7u8; 32];

    #[test]
    fn texto_claro_pasa_intacto_sin_tocar_llavero() {
        match leer_campo("hola sin prefijo") {
            CampoLeido::Claro(s) => assert_eq!(s, "hola sin prefijo"),
            _ => panic!("el texto claro debe pasar como claro"),
        }
    }

    #[test]
    fn roundtrip_con_llave_inyectada() {
        let cifrado = cifrar_con(&K, "el médico llegó rápido");
        assert!(cifrado.starts_with(PREFIJO), "debe llevar prefijo");
        assert!(!cifrado.contains("médico"), "nada de texto claro visible");
        match leer_con(&K, &cifrado) {
            CampoLeido::Descifrado(s) => assert_eq!(s, "el médico llegó rápido"),
            _ => panic!("debía descifrar"),
        }
        // Idempotencia: lo ya cifrado no se re-cifra (marcador de migración).
        assert_eq!(cifrar_con(&K, &cifrado), cifrado);
    }

    #[test]
    fn llave_equivocada_es_no_descifrable() {
        let cifrado = cifrar_con(&K, "secreto");
        let otra = [9u8; 32];
        match leer_con(&otra, &cifrado) {
            CampoLeido::NoDescifrable => {}
            _ => panic!("con otra llave debe ser no descifrable, jamás basura"),
        }
    }

    #[test]
    fn cuerpo_corrupto_jamas_panic() {
        for corrupto in ["esc1:@@@no-es-base64@@@", "esc1:", "esc1:AAAA"] {
            match leer_con(&K, corrupto) {
                CampoLeido::NoDescifrable => {}
                CampoLeido::Claro(_) => panic!("un prefijo esc1: nunca es claro"),
                CampoLeido::Descifrado(_) => panic!("basura no descifra"),
            }
        }
    }

    #[test]
    fn campo_vacio_no_gana_prefijo() {
        assert_eq!(cifrar_con(&K, ""), "");
    }

    #[test]
    fn hex_de_llave_valida_e_invalida() {
        let hex: String = K.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(decodificar_hex(&hex), Some(K));
        assert_eq!(decodificar_hex("corto"), None);
        assert_eq!(decodificar_hex(&"zz".repeat(32)), None);
    }
}
