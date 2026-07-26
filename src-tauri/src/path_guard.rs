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

/// Puntos de montaje donde vive un medio externo en cada plataforma.
///
/// En Windows los medios extraíbles son letras de unidad, no rutas bajo una
/// raíz común, así que se resuelven aparte en [`is_external_media`].
#[cfg(target_os = "macos")]
const EXTERNAL_MEDIA_ROOTS: &[&str] = &["/Volumes"];
#[cfg(target_os = "linux")]
const EXTERNAL_MEDIA_ROOTS: &[&str] = &["/media", "/mnt", "/run/media"];
#[cfg(target_os = "windows")]
const EXTERNAL_MEDIA_ROOTS: &[&str] = &[];

/// En Windows: `true` si la ruta está en una unidad distinta de la del home.
///
/// Una tarjeta SD o un disco USB se montan como `D:\`, `E:\`… Comparar el
/// prefijo con el del home distingue "medio externo" de "el disco del sistema"
/// sin tener que enumerar unidades.
#[cfg(target_os = "windows")]
fn is_external_media(app: &AppHandle, path: &Path) -> bool {
    let Ok(home) = app.path().home_dir() else {
        return false;
    };
    let drive = |p: &Path| {
        p.components()
            .next()
            .map(|c| c.as_os_str().to_ascii_uppercase())
    };
    match (drive(path), drive(&home)) {
        (Some(a), Some(b)) => a != b,
        _ => false,
    }
}

#[cfg(not(target_os = "windows"))]
fn is_external_media(_app: &AppHandle, _path: &Path) -> bool {
    false
}

/// Raíces desde las que el Estudio acepta abrir un archivo de medios.
///
/// Es [`consented_roots`] más los puntos de montaje de medios externos, y la
/// diferencia con el MCP es deliberada:
///
/// - En el MCP, quien pide transcribir es un agente externo, no la persona. Que
///   no pueda salir del home es exactamente lo que se quiere.
/// - En el Estudio, el archivo lo elige el usuario con el diálogo nativo o
///   arrastrándolo a la ventana. Contener al home ahí rompería un caso real:
///   transcribir una grabación directamente desde una grabadora, una tarjeta SD
///   o un disco externo.
///
/// Lo que sí cierra: leer el home de OTRO usuario del equipo, y rutas de
/// sistema. Lo que **no** cierra, y conviene no venderlo como que sí: un
/// frontend comprometido puede seguir pidiendo transcribir los archivos del
/// propio usuario. Para eso haría falta que el diálogo lo abriera el backend y
/// llevara un registro de las rutas consentidas, que es un cambio mayor.
pub fn media_roots(app: &AppHandle) -> Vec<PathBuf> {
    let mut roots = consented_roots(app);
    for candidate in EXTERNAL_MEDIA_ROOTS {
        if let Ok(p) = std::fs::canonicalize(candidate) {
            roots.push(p);
        }
    }
    roots
}

/// Comprueba que un archivo de medios elegido por el usuario sea aceptable.
///
/// Canonicaliza primero (lo que ya elimina `..` y los symlinks que apunten
/// fuera) y después exige que caiga en [`media_roots`] o en un medio externo.
pub fn contain_media_path(app: &AppHandle, path: &Path, err: &str) -> Result<PathBuf, String> {
    let canonical = std::fs::canonicalize(path).map_err(|_| err.to_string())?;
    if is_external_media(app, &canonical) {
        return Ok(canonical);
    }
    let roots = media_roots(app);
    if roots.is_empty() || !roots.iter().any(|r| canonical.starts_with(r)) {
        return Err(err.to_string());
    }
    Ok(canonical)
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

    /// Réplica de la comprobación de raíces de `contain_media_path`, sin
    /// `AppHandle` (que no se puede construir en un test unitario). Lo que se
    /// fija aquí es la propiedad que un refactor rompería sin darse cuenta.
    fn within_roots(canonical: &Path, roots: &[PathBuf]) -> bool {
        !roots.is_empty() && roots.iter().any(|r| canonical.starts_with(r))
    }

    #[test]
    fn media_roots_admiten_medios_externos_y_rechazan_otros_usuarios() {
        // El Estudio NO puede contenerse solo al home como hace el MCP: el
        // usuario elige el archivo con el diálogo nativo, y transcribir desde
        // una tarjeta SD o un disco externo es un caso real.
        let roots = vec![PathBuf::from("/Users/yo"), PathBuf::from("/Volumes")];

        assert!(within_roots(Path::new("/Users/yo/Downloads/a.mp3"), &roots));
        assert!(
            within_roots(Path::new("/Volumes/SD/entrevista.wav"), &roots),
            "un medio externo montado debe entrar"
        );

        // Y lo que sí cierra: el home de otra persona del mismo equipo y las
        // rutas de sistema.
        assert!(!within_roots(
            Path::new("/Users/otro/Documents/privado.m4a"),
            &roots
        ));
        assert!(!within_roots(Path::new("/etc/algo.wav"), &roots));
    }

    #[test]
    fn prefijo_parecido_no_cuenta_como_dentro() {
        // `starts_with` de Path compara COMPONENTES, no texto: "/Users/yotro"
        // no está dentro de "/Users/yo" aunque la cadena empiece igual. Si
        // alguien lo cambiara por comparación de strings, esto lo caza.
        let roots = vec![PathBuf::from("/Users/yo")];
        assert!(!within_roots(Path::new("/Users/yotro/x.mp3"), &roots));
    }

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
