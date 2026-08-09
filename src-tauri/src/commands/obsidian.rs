//! Enviar a Obsidian: escribe un documento (acta de sesión, transcripción del
//! Estudio) como nota Markdown en la carpeta del vault que elija el usuario.
//! Todo local: es una escritura de archivo, nada sale del equipo. El backend
//! escribe directo (no el webview), que deliberadamente no tiene permisos de
//! filesystem; la ruta la consiente el usuario con el selector de carpeta.

use crate::settings::{get_settings, write_settings};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Guarda (o borra) la carpeta del vault de Obsidian en los ajustes.
///
/// La carpeta se contiene al home del usuario. El comentario de cabecera decía
/// que "la ruta la consiente el usuario con el selector de carpeta", pero el
/// backend no puede comprobar que la cadena venga de verdad del selector: sin
/// esta validación, cualquiera que pudiera escribir en los ajustes convertía
/// este comando en una escritura de archivos `.md` en cualquier parte del disco.
#[tauri::command]
#[specta::specta]
pub fn set_obsidian_vault(app: AppHandle, path: String) -> Result<(), String> {
    let validated = if path.trim().is_empty() {
        // Cadena vacía es el "olvidar el vault" del frontend, no una ruta.
        String::new()
    } else {
        crate::path_guard::contain_existing_path(
            &app,
            Path::new(&path),
            "Esa carpeta no está dentro de tu carpeta personal.",
        )?
        .to_string_lossy()
        .to_string()
    };

    let mut settings = get_settings(&app);
    settings.obsidian_vault_path = validated;
    write_settings(&app, settings);
    Ok(())
}

/// Subcarpeta de notas. Se guarda ya saneada, para que lo que se muestre en
/// Ajustes sea exactamente lo que se va a usar al exportar.
#[tauri::command]
#[specta::specta]
pub fn set_obsidian_notes_folder(app: AppHandle, folder: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.obsidian_notes_folder = sanitize_folder(&folder);
    write_settings(&app, settings);
    Ok(())
}

/// Ruta del vault configurada (cadena vacía si no hay).
#[tauri::command]
#[specta::specta]
pub fn get_obsidian_vault(app: AppHandle) -> Result<String, String> {
    Ok(get_settings(&app).obsidian_vault_path)
}

/// Escribe `content` como nota Markdown titulada `title` en el vault. Devuelve
/// la ruta del archivo creado. Falla con un mensaje claro si no hay vault
/// configurado o la carpeta ya no existe (para que el frontend pida elegirla).
#[tauri::command]
#[specta::specta]
pub fn export_to_obsidian(
    app: AppHandle,
    title: String,
    content: String,
) -> Result<String, String> {
    let vault = get_settings(&app).obsidian_vault_path;
    if vault.trim().is_empty() {
        return Err("SIN_VAULT".to_string());
    }
    // Se revalida en la escritura, no solo al guardar el ajuste: un vault
    // configurado antes de que existiera la validación sigue en el store, y
    // `settings_store.json` es un archivo de texto que el usuario puede editar.
    let vault_dir =
        crate::path_guard::contain_existing_path(&app, Path::new(&vault), "VAULT_NO_EXISTE")?;
    if !vault_dir.is_dir() {
        return Err("VAULT_NO_EXISTE".to_string());
    }

    // Subcarpeta propia, creada sola: las notas exportadas no tienen por qué
    // ensuciar la raíz del vault de nadie. Se crea aquí y no al guardar el
    // ajuste, porque el usuario puede borrarla desde Obsidian en cualquier
    // momento y la exportación tiene que seguir funcionando.
    let folder = sanitize_folder(&get_settings(&app).obsidian_notes_folder);
    let target_dir = if folder.is_empty() {
        vault_dir
    } else {
        let dir = vault_dir.join(&folder);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("No se pudo crear la carpeta de notas: {}", e))?;
        // Revalidar DESPUÉS de crearla: `sanitize_folder` ya descarta `..` y
        // separadores, pero si la carpeta existía como enlace simbólico
        // apuntando fuera del vault, el nombre sería inocente y el destino no.
        // Esto es lo que convierte un ajuste editable a mano en algo seguro.
        crate::path_guard::contain_existing_path(
            &app,
            &dir,
            "La carpeta de notas quedó fuera de tu carpeta personal.",
        )?
    };

    let safe_title = sanitize_filename(&title);
    let filename = format!("{}.md", safe_title);
    // Si ya existe una nota con ese nombre, no la pisamos: sufijo incremental.
    let dest = unique_path(&target_dir, &safe_title, &filename);

    let front_matter = format!(
        "---\nsource: Escriba\ncreated: {}\n---\n\n",
        // Fecha local legible; si el reloj falla, se omite el valor.
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );
    let heading = format!("# {}\n\n", title.trim());
    let body = format!("{}{}{}", front_matter, heading, content.trim_end());

    std::fs::write(&dest, body).map_err(|e| format!("No se pudo escribir la nota: {}", e))?;

    // Nota índice (PRP-007, Fase 2): se actualiza DESPUÉS de escribir la nota
    // y con el nombre FINAL del archivo (el de unique_path, con sufijo si hubo
    // colisión). Si el índice falla, el export ya está hecho: se avisa por log
    // y no se rompe el flujo del usuario.
    if get_settings(&app).obsidian_index_note {
        if let Some(nombre_final) = dest
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .filter(|n| n != INDICE_NOTA_BASE)
        {
            let fecha = chrono::Local::now().format("%Y-%m-%d").to_string();
            if let Err(e) = actualizar_indice(&target_dir, &nombre_final, &fecha) {
                log::error!("No se pudo actualizar el índice de Obsidian: {}", e);
            }
        }
    }

    Ok(dest.to_string_lossy().to_string())
}

/// Nombre base de la nota índice (MOC) que mantiene Escriba.
const INDICE_NOTA_BASE: &str = "Escriba";
const INDICE_ABRE: &str = "<!-- escriba:indice -->";
const INDICE_CIERRA: &str = "<!-- /escriba:indice -->";

/// Reescribe SOLO el bloque gestionado del índice; lo que el usuario haya
/// escrito fuera del bloque sobrevive byte a byte. Sin marcadores (los borró),
/// el bloque se re-agrega al final sin tocar lo demás. Idempotente: una nota
/// ya listada no se duplica.
fn indice_actualizado(actual: Option<&str>, nombre_nota: &str, fecha: &str) -> String {
    let entrada = format!("- [[{}]] — {}", nombre_nota, fecha);
    let bloque_nuevo = |entradas: Vec<String>| -> String {
        format!(
            "{}\n{}\n{}",
            INDICE_ABRE,
            entradas.join("\n"),
            INDICE_CIERRA
        )
    };
    match actual {
        None => format!(
            "# Notas de Escriba\n\nNotas creadas por Escriba al exportar. Este índice mantiene solo el bloque de abajo; todo lo que escribas fuera de él es tuyo y sobrevive.\n\n{}\n",
            bloque_nuevo(vec![entrada])
        ),
        Some(texto) => {
            let (antes, dentro, despues) = match (texto.find(INDICE_ABRE), texto.find(INDICE_CIERRA))
            {
                (Some(a), Some(c)) if c > a => {
                    let dentro = &texto[a + INDICE_ABRE.len()..c];
                    (&texto[..a], dentro, &texto[c + INDICE_CIERRA.len()..])
                }
                // Marcadores ausentes o rotos: el bloque se re-agrega al final.
                _ => {
                    let mut base = texto.to_string();
                    if !base.ends_with('\n') {
                        base.push('\n');
                    }
                    return format!("{}\n{}\n", base, bloque_nuevo(vec![entrada]));
                }
            };
            let ya_listada = dentro
                .lines()
                .any(|l| l.contains(&format!("[[{}]]", nombre_nota)));
            let mut entradas: Vec<String> = Vec::new();
            if !ya_listada {
                entradas.push(entrada); // la más nueva arriba
            }
            entradas.extend(
                dentro
                    .lines()
                    .map(str::trim_end)
                    .filter(|l| !l.is_empty())
                    .map(String::from),
            );
            format!("{}{}{}", antes, bloque_nuevo(entradas), despues)
        }
    }
}

fn actualizar_indice(dir: &Path, nombre_nota: &str, fecha: &str) -> Result<(), String> {
    let ruta = dir.join(format!("{}.md", INDICE_NOTA_BASE));
    let actual = std::fs::read_to_string(&ruta).ok();
    let nuevo = indice_actualizado(actual.as_deref(), nombre_nota, fecha);
    std::fs::write(&ruta, nuevo).map_err(|e| e.to_string())
}

/// Bandeja de entrada diaria (PRP-007, Fase 3): agrega un dictado al final de
/// `Inbox YYYY-MM-DD.md` como entrada con hora. Append puro: jamás reordena ni
/// reescribe lo anterior. Misma disciplina de revalidación que el export (el
/// vault se comprueba EN CADA operación, no solo al guardar el ajuste).
#[tauri::command]
#[specta::specta]
pub fn append_to_obsidian_inbox(app: AppHandle, content: String) -> Result<String, String> {
    if content.trim().is_empty() {
        return Err("VACIO".to_string());
    }
    let vault = get_settings(&app).obsidian_vault_path;
    if vault.trim().is_empty() {
        return Err("SIN_VAULT".to_string());
    }
    let vault_dir =
        crate::path_guard::contain_existing_path(&app, Path::new(&vault), "VAULT_NO_EXISTE")?;
    if !vault_dir.is_dir() {
        return Err("VAULT_NO_EXISTE".to_string());
    }
    let folder = sanitize_folder(&get_settings(&app).obsidian_notes_folder);
    let target_dir = if folder.is_empty() {
        vault_dir
    } else {
        let dir = vault_dir.join(&folder);
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("No se pudo crear la carpeta de notas: {}", e))?;
        crate::path_guard::contain_existing_path(
            &app,
            &dir,
            "La carpeta de notas quedó fuera de tu carpeta personal.",
        )?
    };

    let ahora = chrono::Local::now();
    let ruta = target_dir.join(format!("Inbox {}.md", ahora.format("%Y-%m-%d")));
    let existente = std::fs::read_to_string(&ruta).unwrap_or_default();
    let mut cuerpo = if existente.is_empty() {
        format!("# Inbox {}\n", ahora.format("%Y-%m-%d"))
    } else {
        existente
    };
    if !cuerpo.ends_with('\n') {
        cuerpo.push('\n');
    }
    cuerpo.push_str(&format!(
        "\n## {}\n\n{}\n",
        ahora.format("%H:%M"),
        content.trim()
    ));
    std::fs::write(&ruta, cuerpo).map_err(|e| format!("No se pudo escribir el inbox: {}", e))?;
    Ok(ruta.to_string_lossy().to_string())
}

/// Resultado de convertir menciones en enlaces `[[...]]`.
#[derive(serde::Serialize, specta::Type)]
pub struct LinkedResult {
    pub content: String,
    /// Cuántos enlaces se insertaron (para el hint del diálogo).
    pub links: u32,
}

/// Tope de entradas recorridas en el vault: más allá se enlaza con lo ya
/// escaneado y se sigue (nunca se congela el export por un vault gigante).
const TOPE_ESCANEO: usize = 50_000;

/// Convierte menciones del contenido en enlaces `[[Nota]]` usando SOLO los
/// nombres de archivo del vault (jamás se leen contenidos: privacidad y
/// velocidad). Corre detrás del diálogo de vista previa: el usuario ve y
/// edita el resultado ANTES de que nada toque el vault.
///
/// Reglas del matcher (blindaje matcher-includes-substring):
/// - límites de palabra Unicode: `Ana.md` no enlaza dentro de "Analía"
/// - candidatos por longitud DESC: "Plan Premium" gana sobre "Plan"
/// - mínimo 3 caracteres; insensible a mayúsculas (alias `[[Nota|mención]]`),
///   SENSIBLE a tildes ("mas" no enlaza a `Más.md`)
/// - zonas excluidas: front matter, código (bloques y spans), URLs y enlaces
///   ya existentes
///
/// Sin vault configurado (o inválido) devuelve el contenido intacto: el
/// enlazado es un extra del export, no una condición.
#[tauri::command]
#[specta::specta]
pub fn link_obsidian_mentions(app: AppHandle, content: String) -> Result<LinkedResult, String> {
    let vault = get_settings(&app).obsidian_vault_path;
    if vault.trim().is_empty() {
        return Ok(LinkedResult { content, links: 0 });
    }
    let Ok(vault_dir) =
        crate::path_guard::contain_existing_path(&app, Path::new(&vault), "VAULT_NO_EXISTE")
    else {
        return Ok(LinkedResult { content, links: 0 });
    };
    if !vault_dir.is_dir() {
        return Ok(LinkedResult { content, links: 0 });
    }
    let nombres = scan_note_names(&vault_dir);
    let (linked, count) = link_mentions(&content, &nombres);
    Ok(LinkedResult {
        content: linked,
        links: count,
    })
}

/// Recorre el vault y junta los nombres de nota (`basename` sin `.md`).
/// Sin seguir symlinks (un enlace a /etc o a otro home no aporta candidatos ni
/// se recorre), saltando directorios ocultos (`.obsidian`, `.trash`) y con
/// tope de entradas.
fn scan_note_names(vault: &Path) -> Vec<String> {
    let mut nombres = Vec::new();
    let mut pendientes = vec![vault.to_path_buf()];
    let mut vistos = 0usize;
    while let Some(dir) = pendientes.pop() {
        let Ok(entradas) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entrada in entradas.flatten() {
            vistos += 1;
            if vistos > TOPE_ESCANEO {
                log::warn!(
                    "Vault con más de {} entradas: se enlaza con lo escaneado",
                    TOPE_ESCANEO
                );
                return nombres;
            }
            let path = entrada.path();
            let nombre = entrada.file_name();
            let nombre = nombre.to_string_lossy();
            if nombre.starts_with('.') {
                continue; // .obsidian, .trash, ocultos en general
            }
            // symlink_metadata NO resuelve el enlace: un symlink (a carpeta o
            // archivo) se ignora entero.
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                pendientes.push(path);
            } else if let Some(base) = nombre.strip_suffix(".md") {
                if base.chars().count() >= 3 {
                    nombres.push(base.to_string());
                }
            }
        }
    }
    nombres
}

/// ¿El caracter es parte de una palabra? (límites de palabra Unicode)
fn es_de_palabra(c: char) -> bool {
    c.is_alphanumeric()
}

/// Rangos de bytes del contenido que NO se tocan: front matter, código,
/// URLs y enlaces ya existentes.
fn zonas_protegidas(texto: &str) -> Vec<(usize, usize)> {
    let mut zonas = Vec::new();
    // Front matter YAML al inicio.
    if let Some(resto) = texto.strip_prefix("---\n") {
        if let Some(fin) = resto.find("\n---") {
            zonas.push((0, 4 + fin + 4));
        }
    }
    // Bloques de código cercados.
    let mut en_bloque: Option<usize> = None;
    let mut offset = 0usize;
    for linea in texto.split_inclusive('\n') {
        if linea.trim_start().starts_with("```") {
            match en_bloque.take() {
                Some(inicio) => zonas.push((inicio, offset + linea.len())),
                None => en_bloque = Some(offset),
            }
        }
        offset += linea.len();
    }
    if let Some(inicio) = en_bloque {
        zonas.push((inicio, texto.len()));
    }
    // Spans de `código`, enlaces [[...]], enlaces [texto](url) y URLs.
    let bytes = texto.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let resto = &texto[i..];
        if resto.starts_with('`') {
            if let Some(fin) = resto[1..].find('`') {
                zonas.push((i, i + fin + 2));
                i += fin + 2;
                continue;
            }
        } else if resto.starts_with("[[") {
            if let Some(fin) = resto.find("]]") {
                zonas.push((i, i + fin + 2));
                i += fin + 2;
                continue;
            }
        } else if resto.starts_with('[') {
            // [texto](url) completo, para no enlazar dentro de un enlace.
            if let Some(cierre) = resto.find("](") {
                if let Some(fin) = resto[cierre..].find(')') {
                    zonas.push((i, i + cierre + fin + 1));
                    i += cierre + fin + 1;
                    continue;
                }
            }
        } else if resto.starts_with("http://") || resto.starts_with("https://") {
            let fin = resto.find(char::is_whitespace).unwrap_or(resto.len());
            zonas.push((i, i + fin));
            i += fin;
            continue;
        }
        // Avanzar un caracter completo (UTF-8).
        i += resto.chars().next().map(char::len_utf8).unwrap_or(1);
    }
    zonas
}

fn solapa(zonas: &[(usize, usize)], inicio: usize, fin: usize) -> bool {
    zonas.iter().any(|(a, b)| inicio < *b && *a < fin)
}

/// El matcher puro (testeable sin filesystem): convierte menciones en
/// `[[enlaces]]` y devuelve (texto, cuántos).
fn link_mentions(texto: &str, nombres: &[String]) -> (String, u32) {
    if texto.is_empty() || nombres.is_empty() {
        return (texto.to_string(), 0);
    }
    // Filtro barato: un candidato solo puede matchear si su PRIMERA palabra
    // aparece en el texto. Con 20k notas deja pasar un puñado.
    let palabras_texto: std::collections::HashSet<String> = texto
        .split(|c: char| !es_de_palabra(c))
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect();

    let mut candidatos: Vec<&String> = nombres
        .iter()
        .filter(|n| {
            n.split(|c: char| !es_de_palabra(c))
                .next()
                .map(|w| palabras_texto.contains(&w.to_lowercase()))
                .unwrap_or(false)
        })
        .collect();
    // Longitud DESC: el nombre más largo gana ("Plan Premium" antes que "Plan").
    candidatos.sort_by_key(|n| std::cmp::Reverse(n.chars().count()));
    candidatos.dedup();

    let mut protegidas = zonas_protegidas(texto);
    let mut reemplazos: Vec<(usize, usize, String)> = Vec::new();

    for nombre in candidatos {
        let objetivo: Vec<char> = nombre.chars().collect();
        let chars: Vec<(usize, char)> = texto.char_indices().collect();
        let mut k = 0usize;
        while k + objetivo.len() <= chars.len() {
            // Comparación char a char: insensible a caja, sensible a tildes.
            let calza = objetivo.iter().enumerate().all(|(d, oc)| {
                let tc = chars[k + d].1;
                tc == *oc || tc.to_lowercase().eq(oc.to_lowercase())
            });
            if calza {
                let inicio = chars[k].0;
                let fin = chars
                    .get(k + objetivo.len())
                    .map(|(i, _)| *i)
                    .unwrap_or(texto.len());
                // Límites de palabra a ambos lados.
                let borde_izq = k == 0 || !es_de_palabra(chars[k - 1].1);
                let borde_der = chars
                    .get(k + objetivo.len())
                    .map(|(_, c)| !es_de_palabra(*c))
                    .unwrap_or(true);
                if borde_izq && borde_der && !solapa(&protegidas, inicio, fin) {
                    let mencion = &texto[inicio..fin];
                    let enlace = if mencion == nombre.as_str() {
                        format!("[[{}]]", nombre)
                    } else {
                        // La caja difiere: se conserva cómo lo dijo el usuario.
                        format!("[[{}|{}]]", nombre, mencion)
                    };
                    reemplazos.push((inicio, fin, enlace));
                    // El rango queda protegido para candidatos más cortos.
                    protegidas.push((inicio, fin));
                    k += objetivo.len();
                    continue;
                }
            }
            k += 1;
        }
    }

    if reemplazos.is_empty() {
        return (texto.to_string(), 0);
    }
    reemplazos.sort_by_key(|(inicio, _, _)| *inicio);
    let mut salida = String::with_capacity(texto.len() + reemplazos.len() * 8);
    let mut cursor = 0usize;
    let total = reemplazos.len() as u32;
    for (inicio, fin, enlace) in reemplazos {
        salida.push_str(&texto[cursor..inicio]);
        salida.push_str(&enlace);
        cursor = fin;
    }
    salida.push_str(&texto[cursor..]);
    (salida, total)
}

/// Limpia el nombre de la subcarpeta de notas.
///
/// Este valor sale de los ajustes, y `settings_store.json` es un archivo de
/// texto que se puede editar a mano: sin sanear, un `../../..` ahí convertía la
/// exportación en escritura de `.md` en cualquier parte del disco. Es
/// exactamente el agujero que la competencia reportó haber encontrado en su
/// propia integración con Obsidian.
///
/// Se acepta UN SOLO nivel dentro del vault. Los separadores se vuelven
/// espacios (así `../../etc` queda en algo inofensivo) y los puntos de los
/// extremos se recortan, lo que además impide alcanzar cualquier carpeta oculta
/// del vault: `.obsidian`, que es su configuración, queda como `obsidian`.
///
/// Cadena vacía significa "escribir en la raíz", que es el comportamiento
/// anterior y sigue siendo elegible a propósito.
fn sanitize_folder(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect();
    // Recortar los puntos es lo que descarta `..` y las carpetas ocultas de una
    // vez. Después de esto no puede quedar un punto al principio, así que no
    // hace falta comprobarlo aparte.
    let trimmed = cleaned.trim().trim_matches('.').trim();
    trimmed
        .chars()
        .take(60)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Limpia el título para usarlo como nombre de archivo: quita los caracteres
/// que rompen rutas en cualquier SO y recorta a un largo razonable.
fn sanitize_filename(title: &str) -> String {
    let cleaned: String = title
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\n' | '\r' | '\t' => ' ',
            other => other,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    let base = if trimmed.is_empty() {
        "Nota de Escriba"
    } else {
        trimmed
    };
    base.chars().take(80).collect::<String>().trim().to_string()
}

/// Ruta que no pisa una nota existente: `Título.md`, luego `Título 2.md`, etc.
fn unique_path(dir: &Path, base: &str, filename: &str) -> PathBuf {
    let first = dir.join(filename);
    if !first.exists() {
        return first;
    }
    for n in 2..1000 {
        let candidate = dir.join(format!("{} {}.md", base, n));
        if !candidate.exists() {
            return candidate;
        }
    }
    first
}

#[cfg(test)]
mod tests {
    use super::sanitize_folder;
    use super::{link_mentions, scan_note_names};

    fn nombres(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_enlaza_por_substring() {
        // Nota `Ana.md`: "Analía" queda intacta (blindaje matcher-includes).
        let (salida, n) = link_mentions("Hablé con Analía en la tarde", &nombres(&["Ana"]));
        assert_eq!(salida, "Hablé con Analía en la tarde");
        assert_eq!(n, 0);
        let (salida, n) = link_mentions("Hablé con Ana en la tarde", &nombres(&["Ana"]));
        assert_eq!(salida, "Hablé con [[Ana]] en la tarde");
        assert_eq!(n, 1);
    }

    #[test]
    fn el_nombre_mas_largo_gana() {
        let (salida, _) = link_mentions(
            "hablamos del Plan Premium con el curso",
            &nombres(&["Plan", "Plan Premium"]),
        );
        assert_eq!(salida, "hablamos del [[Plan Premium]] con el curso");
    }

    #[test]
    fn insensible_a_caja_sensible_a_tildes() {
        // Caja distinta → alias que conserva cómo lo dijo el usuario.
        let (salida, _) = link_mentions("vimos el plan premium ayer", &nombres(&["Plan Premium"]));
        assert_eq!(salida, "vimos el [[Plan Premium|plan premium]] ayer");
        // "mas" sin tilde NO enlaza a la nota `Más.md`.
        let (salida, n) = link_mentions("quiero mas tiempo", &nombres(&["Más"]));
        assert_eq!(salida, "quiero mas tiempo");
        assert_eq!(n, 0);
        // "más" con tilde sí.
        let (salida, _) = link_mentions("quiero más tiempo", &nombres(&["Más"]));
        assert_eq!(salida, "quiero [[Más|más]] tiempo");
    }

    #[test]
    fn zonas_excluidas_no_se_tocan() {
        let texto = "---\ntitle: Flor\n---\n\nHabló Flor.\n\n```\nFlor en código\n```\n\nY `Flor` inline, y [[Flor]] ya enlazada, y https://flor.cl/Flor queda.";
        let (salida, n) = link_mentions(texto, &nombres(&["Flor"]));
        // Solo la mención libre ("Habló Flor.") se enlaza.
        assert_eq!(n, 1);
        assert!(salida.contains("Habló [[Flor]]."));
        assert!(salida.contains("title: Flor"), "front matter intacto");
        assert!(
            salida.contains("```\nFlor en código\n```"),
            "bloque intacto"
        );
        assert!(salida.contains("`Flor` inline"), "span intacto");
        assert!(!salida.contains("[[[["), "no re-enlaza lo enlazado");
    }

    #[test]
    fn nombres_cortos_no_participan_y_vacio_es_identidad() {
        let (salida, n) = link_mentions("se lo di a él", &nombres(&[]));
        assert_eq!(salida, "se lo di a él");
        assert_eq!(n, 0);
    }

    #[test]
    fn indice_respeta_lo_del_usuario_y_es_idempotente() {
        use super::indice_actualizado;
        // Creación desde cero.
        let v1 = indice_actualizado(None, "Acta Lunes", "2026-08-08");
        assert!(v1.contains("- [[Acta Lunes]] — 2026-08-08"));
        // El usuario escribe fuera del bloque; una segunda nota entra ARRIBA
        // dentro del bloque y lo del usuario sobrevive byte a byte.
        let con_notas_usuario = format!("MIS NOTAS PROPIAS arriba\n\n{}\ny abajo también", v1);
        let v2 = indice_actualizado(Some(&con_notas_usuario), "Acta Martes", "2026-08-09");
        assert!(v2.starts_with("MIS NOTAS PROPIAS arriba"));
        assert!(v2.ends_with("y abajo también"));
        let pos_martes = v2.find("[[Acta Martes]]").unwrap();
        let pos_lunes = v2.find("[[Acta Lunes]]").unwrap();
        assert!(pos_martes < pos_lunes, "la más nueva va arriba");
        // Idempotencia: re-exportar la misma nota no la duplica.
        let v3 = indice_actualizado(Some(&v2), "Acta Martes", "2026-08-09");
        assert_eq!(v3.matches("[[Acta Martes]]").count(), 1);
        // Marcadores borrados por el usuario: el bloque se re-agrega al final.
        let sin_marcadores = "solo texto del usuario";
        let v4 = indice_actualizado(Some(sin_marcadores), "Acta X", "2026-08-10");
        assert!(v4.starts_with("solo texto del usuario"));
        assert!(v4.contains("[[Acta X]]"));
    }

    #[test]
    fn symlink_fuera_del_vault_no_aporta_candidatos() {
        use std::fs;
        let base = std::env::temp_dir().join(format!("escriba-vault-test-{}", std::process::id()));
        let vault = base.join("vault");
        let fuera = base.join("fuera");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(vault.join("normal")).unwrap();
        fs::create_dir_all(&fuera).unwrap();
        fs::write(vault.join("normal").join("Nota Real.md"), "").unwrap();
        fs::write(fuera.join("Secreta.md"), "").unwrap();
        fs::write(vault.join(".obsidian.md"), "").unwrap(); // oculto: fuera
        #[cfg(unix)]
        std::os::unix::fs::symlink(&fuera, vault.join("enlace")).unwrap();
        let encontrados = scan_note_names(&vault);
        assert!(encontrados.contains(&"Nota Real".to_string()));
        assert!(
            !encontrados.iter().any(|n| n == "Secreta"),
            "el symlink no debe recorrerse: {:?}",
            encontrados
        );
        let _ = fs::remove_dir_all(&base);
    }

    /// Lo que hay que garantizar no es una cadena concreta, sino que el
    /// resultado sea SIEMPRE un único nombre de carpeta normal: así, unirlo al
    /// vault no puede salir de él por mucho que alguien edite los ajustes a
    /// mano. Afirmar la salida literal probaba la implementación de hoy; esto
    /// prueba la propiedad.
    #[test]
    fn folder_is_always_a_single_normal_component() {
        use std::path::{Component, PathBuf};

        for raw in [
            "../../etc",
            "..",
            "/etc/passwd",
            "a/b",
            "..\\..\\windows",
            "./../x",
            "C:\\Windows",
            "~/otro",
        ] {
            let out = sanitize_folder(raw);
            if out.is_empty() {
                continue; // vacío = raíz del vault, que es contenido por definición
            }
            let as_path = PathBuf::from(&out);
            let comps: Vec<_> = as_path.components().collect();
            assert_eq!(
                comps.len(),
                1,
                "{raw:?} salió como {out:?}, con más de un componente"
            );
            assert!(
                matches!(comps[0], Component::Normal(_)),
                "{raw:?} salió como {out:?}, que no es un nombre normal"
            );
        }
    }

    #[test]
    fn folder_cannot_reach_a_hidden_directory() {
        // La propiedad que importa no es qué devuelve, sino que NUNCA pueda
        // resolver a una carpeta oculta: `.obsidian` es la configuración del
        // vault y escribir ahí lo rompe.
        for raw in [".obsidian", ".git", "...trampa"] {
            let out = sanitize_folder(raw);
            assert!(
                !out.starts_with('.'),
                "{raw:?} resolvió a una carpeta oculta: {out:?}"
            );
        }
        assert_eq!(sanitize_folder(".obsidian"), "obsidian");
    }

    #[test]
    fn folder_keeps_a_normal_name() {
        assert_eq!(sanitize_folder("Escriba"), "Escriba");
        assert_eq!(sanitize_folder("  Mis dictados  "), "Mis dictados");
    }

    #[test]
    fn folder_empty_means_vault_root() {
        assert_eq!(sanitize_folder(""), "");
        assert_eq!(sanitize_folder("   "), "");
    }
}
