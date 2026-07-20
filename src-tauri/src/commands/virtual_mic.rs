//! Micrófono virtual integrado (BlackHole 2ch, de Existential Audio, open
//! source): misma filosofía que el motor local Qwen — descarga desde la
//! fuente oficial verificada por SHA256 y activación sin salir de Escriba.
//! Es un driver de audio del sistema, así que macOS pide la contraseña de
//! admin con su diálogo nativo; eso lo exige el OS, no hay forma de omitirlo.

use tauri::AppHandle;

/// Paquete oficial pinneado (misma versión y hash que verifica Homebrew).
#[cfg(target_os = "macos")]
const PKG_URL: &str = "https://existential.audio/downloads/BlackHole2ch-0.7.1.pkg";
#[cfg(target_os = "macos")]
const PKG_SHA256: &str = "57b540f27a3e29c37e310e01bee0fdfab76733087e47f997ef9dccf851400dcf";

/// ¿Está el micrófono virtual instalado? (aparece como dispositivo de salida).
#[tauri::command]
#[specta::specta]
pub fn virtual_mic_installed() -> bool {
    crate::audio_toolkit::list_output_devices()
        .map(|list| list.iter().any(|d| d.name.contains("BlackHole")))
        .unwrap_or(false)
}

/// Descarga el paquete oficial (SHA256 verificado, descartado si no calza) y
/// lo instala con el diálogo nativo de privilegios de macOS. Devuelve false
/// si el usuario canceló el diálogo (no es un error, no hay drama).
#[tauri::command]
#[specta::specta]
pub async fn virtual_mic_install(app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let dir = crate::portable::app_data_dir(&app).map_err(|e| e.to_string())?;
        let pkg = dir.join("BlackHole2ch-0.7.1.pkg");
        download_pkg(&pkg).await?;

        // `installer` con privilegios vía osascript: la contraseña la pide el
        // diálogo nativo del sistema, encima de Escriba. La ruta entra por
        // argv y `quoted form` (sin interpolar nada en el shell). Al final se
        // reinicia coreaudiod: sin eso el driver queda en disco pero macOS no
        // lo carga y el dispositivo no aparece (QA de Alejandro, 20-jul).
        const SCRIPT: &str = "on run argv\ndo shell script \"/usr/sbin/installer -pkg \" & quoted form of item 1 of argv & \" -target / && (/usr/bin/killall coreaudiod || true)\" with administrator privileges\nend run";
        let pkg2 = pkg.clone();
        let out = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("/usr/bin/osascript")
                .args(["-e", SCRIPT])
                .arg(&pkg2)
                .output()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            // -128 = el usuario cerró el diálogo de contraseña: no es error.
            if err.contains("-128") {
                return Ok(false);
            }
            log::warn!("virtual_mic_install failed: {}", err.trim());
            return Err("La instalación no terminó. Intenta de nuevo.".to_string());
        }
        let _ = std::fs::remove_file(&pkg);
        Ok(true)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Disponible solo en macOS".to_string())
    }
}

/// Descarga a memoria (el paquete pesa menos de 1 MB), verifica el SHA256 y
/// recién ahí escribe a disco. Idempotente si ya está descargado y verifica.
#[cfg(target_os = "macos")]
async fn download_pkg(dest: &std::path::Path) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    if let Ok(bytes) = std::fs::read(dest) {
        if format!("{:x}", Sha256::digest(&bytes)) == PKG_SHA256 {
            return Ok(());
        }
    }
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(PKG_URL)
        .send()
        .await
        .map_err(|e| format!("Descarga falló: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Descarga falló con HTTP {}", resp.status()));
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Descarga interrumpida: {}", e))?;
    if format!("{:x}", Sha256::digest(&bytes)) != PKG_SHA256 {
        return Err("Verificación SHA256 falló. Descarga descartada.".to_string());
    }
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}
