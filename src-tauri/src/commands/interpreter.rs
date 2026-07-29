//! Comandos del Intérprete en vivo (guía).

use crate::managers::interpreter::{global, RoomInfo};
use log::warn;
use serde::Serialize;
use specta::Type;

#[derive(Serialize, Clone, Type)]
pub struct InterpreterRoom {
    pub code: String,
    pub url: String,
    pub qr_svg: String,
    pub port: u16,
}

impl From<RoomInfo> for InterpreterRoom {
    fn from(r: RoomInfo) -> Self {
        Self {
            code: r.code,
            url: r.url,
            qr_svg: r.qr_svg,
            port: r.port,
        }
    }
}

#[derive(Serialize, Clone, Type)]
pub struct InterpreterStatus {
    pub running: bool,
    pub listeners: u32,
    pub active_languages: Vec<String>,
}

#[tauri::command]
#[specta::specta]
pub async fn interpreter_start() -> Result<InterpreterRoom, String> {
    global().start().await.map(Into::into)
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_stop() {
    global().stop();
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_status() -> InterpreterStatus {
    let s = global();
    InterpreterStatus {
        running: s.is_running(),
        listeners: s.listeners(),
        active_languages: s.active_languages(),
    }
}

/// Publica una línea del guía: traduce a cada idioma de oyente activo con el
/// motor local y la emite. En Fase 1/2 el texto viene del input de prueba o
/// del micrófono; el idioma de origen se asume el de la app.
#[tauri::command]
#[specta::specta]
pub async fn interpreter_publish(app: tauri::AppHandle, text: String, source_lang: String) {
    publish_translated(&app, text, &source_lang).await;
}

/// Publica una frase del guía: primero el original a todos, y después cada
/// traducción en cuanto está lista.
///
/// Antes esto traducía a TODOS los idiomas activos antes de emitir nada, y en
/// serie. Medido con el motor local, cada traducción cuesta de 1 a 2 segundos,
/// así que una sala con cinco idiomas dejaba la pantalla en blanco entre cinco y
/// diez segundos — y quien escuchaba en el idioma de origen, que no necesita
/// ninguna traducción, esperaba igual a las otras cuatro. Se probó siempre con
/// un solo idioma y por eso parecía instantáneo.
///
/// Ahora el original sale de inmediato y cada idioma reemplaza su línea al
/// llegar la suya. Las traducciones siguen en serie a propósito: el motor local
/// arranca sin `-np`, así que atiende una petición a la vez y lanzarlas juntas
/// solo movería la cola del cliente al servidor.
pub async fn publish_translated(app: &tauri::AppHandle, text: String, source_lang: &str) {
    use std::collections::HashMap;
    let server = global();
    let seq = server.next_seq();

    // 1) El original, ya. Quien escucha en el idioma de origen recibe su texto
    //    definitivo aquí mismo; el resto lo ve como anticipo mientras se traduce.
    let mut first: HashMap<String, String> = HashMap::new();
    first.insert(source_lang.to_string(), text.clone());
    server.publish_line(seq, text.clone(), first, Vec::new());

    // 2) Cada traducción, dirigida solo a su idioma, en cuanto está.
    for lang in server.active_languages() {
        if lang == source_lang {
            continue;
        }
        // La sala pudo cerrarse mientras se traducía la frase anterior: sin esto
        // se seguiría gastando el modelo en emisiones que ya no escucha nadie.
        if !server.is_running() {
            return;
        }
        let mut one = HashMap::new();
        match crate::actions::translate_live(app, &text, &lang).await {
            Some(t) => {
                one.insert(lang.clone(), t);
            }
            // Sin traducción (motor caído, plazo agotado) se cierra igualmente
            // la línea con el ORIGINAL. Si no, ese oyente se queda con el
            // anticipo colgado para siempre: atenuado en pantalla y, lo que es
            // peor, MUDO, porque la página solo lee en voz alta el texto
            // definitivo. Vale más oír la frase en el idioma del guía que no
            // oír nada.
            None => {
                warn!("sin traducción a '{}': se cierra con el original", lang);
                one.insert(lang.clone(), text.clone());
            }
        }
        server.publish_line(seq, text.clone(), one, vec![lang]);
    }
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_set_source_lang(lang: String) {
    global().set_source_lang(lang);
}

#[tauri::command]
#[specta::specta]
pub fn interpreter_set_listening(on: bool) {
    global().set_listening(on);
}
