//! Enviar a Obsidian: escribe un documento (acta de sesión, transcripción del
//! Estudio) como nota Markdown en la carpeta del vault que elija el usuario.
//! Todo local: es una escritura de archivo, nada sale del equipo. El backend
//! escribe directo (no el webview), así que el `fs:scope` acotado a $APPDATA
//! no aplica; la ruta la consiente el usuario con el selector de carpeta.

use crate::settings::{get_settings, write_settings};
use std::path::{Path, PathBuf};
use tauri::AppHandle;

/// Guarda (o borra) la carpeta del vault de Obsidian en los ajustes.
#[tauri::command]
#[specta::specta]
pub fn set_obsidian_vault(app: AppHandle, path: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.obsidian_vault_path = path;
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
    let vault_dir = PathBuf::from(&vault);
    if !vault_dir.is_dir() {
        return Err("VAULT_NO_EXISTE".to_string());
    }

    let safe_title = sanitize_filename(&title);
    let filename = format!("{}.md", safe_title);
    // Si ya existe una nota con ese nombre, no la pisamos: sufijo incremental.
    let dest = unique_path(&vault_dir, &safe_title, &filename);

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
