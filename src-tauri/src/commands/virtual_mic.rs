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
/// si el usuario canceló el diálogo (no es un error, no hay drama). Los
/// errores son claves estables para que el frontend traduzca (auditoría #14).
#[tauri::command]
#[specta::specta]
pub async fn virtual_mic_install(app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let dir = crate::portable::app_data_dir(&app).map_err(|e| e.to_string())?;
        let pkg = dir.join("BlackHole2ch-0.7.1.pkg");
        download_pkg(&pkg).await?;

        // TOCTOU (auditoría #5): verificar el SHA como usuario y luego instalar
        // como root deja una ventana en que otro proceso del usuario cambie el
        // .pkg por uno malicioso que se instalaría con privilegios. El paso
        // root COPIA el paquete a una carpeta propia (mktemp, root 700), lo
        // RE-VERIFICA ahí, y solo entonces instala: ya no se puede intercambiar
        // entre la verificación y la instalación. Al final reinicia coreaudiod
        // (sin eso el driver queda en disco sin cargar). El hash es constante
        // (hex), la ruta entra por `quoted form of item 1 of argv`.
        let script = format!(
            "on run argv\n\
             do shell script \"set -e; \
             tmp=$(/usr/bin/mktemp -d); \
             /bin/cp \" & quoted form of item 1 of argv & \" \\\"$tmp/bh.pkg\\\"; \
             actual=$(/usr/bin/shasum -a 256 \\\"$tmp/bh.pkg\\\" | /usr/bin/cut -d' ' -f1); \
             if [ \\\"$actual\\\" != \\\"{sha}\\\" ]; then /bin/rm -rf \\\"$tmp\\\"; exit 3; fi; \
             /usr/sbin/installer -pkg \\\"$tmp/bh.pkg\\\" -target /; \
             /bin/rm -rf \\\"$tmp\\\"; \
             (/usr/bin/killall coreaudiod || true)\" with administrator privileges\n\
             end run",
            sha = PKG_SHA256
        );
        let pkg2 = pkg.clone();
        let out = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("/usr/bin/osascript")
                .args(["-e", &script])
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
            return Err("vm.install_failed".to_string());
        }
        let _ = std::fs::remove_file(&pkg);
        Ok(true)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("vm.only_macos".to_string())
    }
}

/// Desinstala el micrófono virtual (auditoría #13: antes salir exigía `sudo rm`
/// a mano). Borra el driver de BlackHole y reinicia coreaudiod, con el diálogo
/// de privilegios de macOS. false = el usuario canceló el diálogo.
#[tauri::command]
#[specta::specta]
pub async fn virtual_mic_uninstall() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        // Ruta fija del driver (la que instala el .pkg oficial). Sin argv del
        // usuario: la ruta es literal y conocida, no hay interpolación externa.
        const SCRIPT: &str = "do shell script \"/bin/rm -rf '/Library/Audio/Plug-Ins/HAL/BlackHole2ch.driver' && (/usr/bin/killall coreaudiod || true)\" with administrator privileges";
        let out = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("/usr/bin/osascript")
                .args(["-e", SCRIPT])
                .output()
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            if err.contains("-128") {
                return Ok(false);
            }
            log::warn!("virtual_mic_uninstall failed: {}", err.trim());
            return Err("vm.uninstall_failed".to_string());
        }
        Ok(true)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("vm.only_macos".to_string())
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
        .map_err(|_| "vm.download_failed".to_string())?;
    if !resp.status().is_success() {
        return Err("vm.download_failed".to_string());
    }
    let bytes = resp
        .bytes()
        .await
        .map_err(|_| "vm.download_failed".to_string())?;
    if format!("{:x}", Sha256::digest(&bytes)) != PKG_SHA256 {
        return Err("vm.sha_failed".to_string());
    }
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}
