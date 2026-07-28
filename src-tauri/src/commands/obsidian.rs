//! Enviar a Obsidian: escribe un documento (acta de sesión, transcripción del
//! Estudio) como nota Markdown en la carpeta del vault que elija el usuario.
//! Todo local: es una escritura de archivo, nada sale del equipo. El backend
//! escribe directo (no el webview), así que el `fs:scope` acotado a $APPDATA
//! no aplica; la ruta la consiente el usuario con el selector de carpeta.

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
    Ok(dest.to_string_lossy().to_string())
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
