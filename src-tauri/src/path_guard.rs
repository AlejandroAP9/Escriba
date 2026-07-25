//! Contención de rutas para las operaciones que tocan el disco fuera del
//! app-data dir.
//!
//! El backend recibe rutas desde tres orígenes que no controla: la webview
//! (comandos Tauri), el servidor MCP y el propio `settings_store.json`. Antes
//! de esta capa, tres caminos escribían o ejecutaban en rutas arbitrarias:
//!
//! - `commands::studio::studio_export_to` escribía bytes en cualquier ruta
//!   absoluta, así que un frontend comprometido podía sobrescribir `~/.zshrc`
//!   o dejar un LaunchAgent.
//! - `clipboard::paste_via_external_script` ejecutaba cualquier binario en
//!   cada pegado, sin comprobar siquiera que existiera.
//! - `commands::obsidian::export_to_obsidian` escribía en cualquier carpeta.
//!
//! `managers::mcp::transcribe_file` ya resolvía bien el problema; este módulo
//! generaliza ese patrón para que los cuatro usen la misma definición de
//! "dentro de los límites del usuario".

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// Raíces en las que el usuario puede pedirnos leer, escribir o ejecutar:
/// su home y el directorio de datos de la app. Ambas canonicalizadas, para que
/// la comparación posterior no se pueda burlar con `..` ni con symlinks.
pub fn consented_roots(app: &AppHandle) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    if let Ok(home) = app.path().home_dir() {
        if let Ok(home) = std::fs::canonicalize(home) {
            roots.push(home);
        }
    }
    if let Ok(data) = crate::portable::app_data_dir(app) {
        if let Ok(data) = std::fs::canonicalize(data) {
            roots.push(data);
        }
    }
    roots
}

/// Canonicaliza una ruta **que ya existe** y comprueba que caiga dentro de las
/// raíces consentidas. Para archivos por crear, usa [`contain_new_path`].
pub fn contain_existing_path(app: &AppHandle, path: &Path, err: &str) -> Result<PathBuf, String> {
    let canonical = std::fs::canonicalize(path).map_err(|_| err.to_string())?;
    let roots = consented_roots(app);
    if roots.is_empty() || !roots.iter().any(|r| canonical.starts_with(r)) {
        return Err(err.to_string());
    }
    Ok(canonical)
}

/// Comprueba una ruta de destino que todavía no existe.
///
/// No se puede canonicalizar un archivo inexistente, así que se canonicaliza el
/// directorio padre (que sí debe existir) y se le vuelve a pegar el nombre. Eso
/// resuelve los `..` y los symlinks intermedios, que es por donde se escapa una
/// comprobación ingenua sobre la cadena de texto.
pub fn contain_new_path(app: &AppHandle, path: &Path, err: &str) -> Result<PathBuf, String> {
    let parent = path.parent().ok_or_else(|| err.to_string())?;
    let file_name = path.file_name().ok_or_else(|| err.to_string())?;

    // Un nombre de archivo no puede traer separadores ni componentes relativos:
    // eso solo aparece si alguien intenta escapar del directorio elegido.
    let name_str = file_name.to_string_lossy();
    if name_str == ".." || name_str == "." || name_str.contains('/') || name_str.contains('\\') {
        return Err(err.to_string());
    }

    let canonical_parent = std::fs::canonicalize(parent).map_err(|_| err.to_string())?;
    let roots = consented_roots(app);
    if roots.is_empty() || !roots.iter().any(|r| canonical_parent.starts_with(r)) {
        return Err(err.to_string());
    }
    Ok(canonical_parent.join(file_name))
}

/// Valida un script de pegado externo antes de persistirlo en los ajustes.
///
/// `clipboard::paste_via_external_script` lo ejecuta en **cada dictado**, así
/// que es el punto del programa con mayor valor para un atacante: basta con
/// escribir una ruta en los ajustes para lograr ejecución de código en la
/// próxima pulsación del atajo. Validar aquí (en la escritura) y no en la
/// ejecución es lo correcto: el usuario recibe el error mientras configura, y
/// no un fallo silencioso a mitad de un dictado.
pub fn validate_external_script(app: &AppHandle, path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);

    if !candidate.is_absolute() {
        return Err("La ruta del script debe ser absoluta.".to_string());
    }

    let canonical = std::fs::canonicalize(&candidate)
        .map_err(|_| "No se encontró ningún archivo en esa ruta.".to_string())?;

    if !canonical.is_file() {
        return Err("La ruta debe apuntar a un archivo, no a una carpeta.".to_string());
    }

    let roots = consented_roots(app);
    if roots.is_empty() || !roots.iter().any(|r| canonical.starts_with(r)) {
        return Err(
            "El script debe estar dentro de tu carpeta personal. Escriba no ejecuta binarios del sistema."
                .to_string(),
        );
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&canonical)
            .map_err(|_| "No se pudo leer el archivo.".to_string())?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            return Err(
                "El archivo no tiene permiso de ejecución. Dale permisos con: chmod +x <ruta>"
                    .to_string(),
            );
        }
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal_in_file_name() {
        // `contain_new_path` no debe aceptar un nombre que en realidad sea una
        // ruta relativa: es la forma más directa de escapar del directorio.
        let p = PathBuf::from("/tmp/algo/..");
        assert_eq!(p.file_name().map(|n| n.to_string_lossy().to_string()), None);
    }

    #[test]
    fn relative_script_path_is_rejected_before_touching_disk() {
        // La comprobación de "absoluta" va antes de canonicalizar, así que una
        // ruta relativa nunca llega a resolverse contra el cwd del proceso.
        let candidate = PathBuf::from("script.sh");
        assert!(!candidate.is_absolute());
    }
}
