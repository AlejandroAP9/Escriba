use crate::managers::local_llm;
use crate::settings::LOCAL_LLM_DEFAULT_MODEL_ID;
use futures_util::StreamExt;
use log::{info, warn};
use serde::Serialize;
use sha2::{Digest, Sha256};
use specta::Type;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter};

/// Release de llama.cpp PINNEADA y verificada (spike 7-jul-2026). Actualizarla
/// implica recalcular los tres SHA256. Nunca usar "latest": un runtime no
/// verificado es superficie de ataque (ver docs/plan/07-security-plan.md).
const RUNTIME_TAG: &str = "b9902";

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RUNTIME_ASSET: &str = "llama-b9902-bin-macos-arm64.tar.gz";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RUNTIME_SHA256: &str = "229db5607ffd2251cf836d76c36e8ff58de23dfba76e578e3a466235671bab0d";

#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const RUNTIME_ASSET: &str = "llama-b9902-bin-macos-x64.tar.gz";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const RUNTIME_SHA256: &str = "df5d74c1d7046829f109a2f7e80d79c1398a86dcff2cfacfc9b44633c8b96abe";

// Windows: variante CPU universal (b9902 no publica Vulkan; el plan ya
// contemplaba CPU como fallback seguro en Windows).
#[cfg(target_os = "windows")]
const RUNTIME_ASSET: &str = "llama-b9902-bin-win-cpu-x64.zip";
#[cfg(target_os = "windows")]
const RUNTIME_SHA256: &str = "3a095d4c6f997d0d500ae5a973cb16e5c91f1f7405406e0ab588bf3fb3d8d872";

// Linux: variante CPU x64 (misma filosofía que Windows: el fallback seguro).
// SHA256 del digest oficial del release de GitHub. (QA pionero de Linux,
// 18-jul-2026: "Plataforma no soportada para el motor local todavía".)
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const RUNTIME_ASSET: &str = "llama-b9902-bin-ubuntu-x64.tar.gz";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const RUNTIME_SHA256: &str = "a331765c7ab782412f9ff718bfec21637b9a4ad8a59c29315f2d1f108c30f645";

#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    all(target_os = "linux", target_arch = "x86_64")
)))]
const RUNTIME_ASSET: &str = "";
#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    all(target_os = "linux", target_arch = "x86_64")
)))]
const RUNTIME_SHA256: &str = "";

/// Modelo por defecto: Qwen3-4B-Instruct-2507 Q4_K_M (Apache-2.0). SHA256
/// tomado del LFS oid oficial de HuggingFace (API /tree), no calculado a mano.
const MODEL_URL: &str = "https://huggingface.co/unsloth/Qwen3-4B-Instruct-2507-GGUF/resolve/main/Qwen3-4B-Instruct-2507-Q4_K_M.gguf";
const MODEL_SHA256: &str = "3605803b982cb64aead44f6c1b2ae36e3acdb41d8e46c8a94c6533bc4c67e597";
const MODEL_SIZE: u64 = 2_497_281_120;

static SETUP_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Clone, Type)]
pub struct LocalLlmStatus {
    pub runtime_installed: bool,
    pub model_installed: bool,
    pub engine_running: bool,
    pub setup_in_progress: bool,
    pub model_file: String,
}

#[derive(Serialize, Clone, Type)]
struct SetupProgress {
    stage: String, // "runtime" | "model" | "done" | "error"
    downloaded: f64,
    total: f64,
    message: String,
}

fn emit_progress(app: &AppHandle, stage: &str, downloaded: u64, total: u64, message: &str) {
    let _ = app.emit(
        "local-llm-setup-progress",
        SetupProgress {
            stage: stage.to_string(),
            downloaded: downloaded as f64,
            total: total as f64,
            message: message.to_string(),
        },
    );
}

#[tauri::command]
#[specta::specta]
pub fn get_local_llm_status() -> LocalLlmStatus {
    let (runtime_installed, model_installed, engine_running) = match local_llm::global() {
        Some(mgr) => (
            mgr.runtime_installed(),
            mgr.model_installed(LOCAL_LLM_DEFAULT_MODEL_ID),
            mgr.is_running(),
        ),
        None => (false, false, false),
    };
    LocalLlmStatus {
        runtime_installed,
        model_installed,
        engine_running,
        setup_in_progress: SETUP_IN_PROGRESS.load(Ordering::Relaxed),
        model_file: LOCAL_LLM_DEFAULT_MODEL_ID.to_string(),
    }
}

/// Descarga runtime + modelo con progreso, verifica SHA256 y deja el motor
/// precalentado. Idempotente: lo ya instalado y verificado se salta.
#[tauri::command]
#[specta::specta]
pub async fn setup_local_llm(app: AppHandle) -> Result<(), String> {
    if RUNTIME_ASSET.is_empty() {
        return Err("Plataforma no soportada para el motor local todavía".to_string());
    }
    if SETUP_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        return Err("La instalación ya está en curso".to_string());
    }
    let result = do_setup(&app).await;
    SETUP_IN_PROGRESS.store(false, Ordering::SeqCst);
    if let Err(ref e) = result {
        emit_progress(&app, "error", 0, 0, e);
    }
    result
}

async fn do_setup(app: &AppHandle) -> Result<(), String> {
    let mgr = local_llm::global().ok_or("Motor local no inicializado")?;

    // 1. Runtime
    if !mgr.runtime_installed() {
        let url = format!(
            "https://github.com/ggml-org/llama.cpp/releases/download/{}/{}",
            RUNTIME_TAG, RUNTIME_ASSET
        );
        let archive = mgr.runtime_dir().join(RUNTIME_ASSET);
        download_verified(app, "runtime", &url, &archive, RUNTIME_SHA256, 0).await?;
        extract_archive(&archive, mgr.runtime_dir())?;
        let _ = std::fs::remove_file(&archive);
        if !mgr.runtime_installed() {
            return Err(
                "El runtime se descargó pero no se encontró llama-server tras extraer".to_string(),
            );
        }
        info!("Local LLM runtime installed");
    }

    // 2. Modelo
    if !mgr.model_installed(LOCAL_LLM_DEFAULT_MODEL_ID) {
        let dest = mgr.models_dir().join(LOCAL_LLM_DEFAULT_MODEL_ID);
        download_verified(app, "model", MODEL_URL, &dest, MODEL_SHA256, MODEL_SIZE).await?;
        info!("Local LLM model installed");
    }

    // 3. Precalentar para que el primer dictado sea instantáneo.
    emit_progress(
        app,
        "model",
        MODEL_SIZE,
        MODEL_SIZE,
        "Preparando el motor...",
    );
    // El precalentado ahora SÍ decide el resultado de la instalación: si el
    // motor no arranca, decir "listo" es mentir, y el usuario se queda con
    // todas las funciones de IA muertas sin saber por qué.
    if let Err(err) = local_llm::warmup_checked(LOCAL_LLM_DEFAULT_MODEL_ID).await {
        log::error!("El motor local no pudo arrancar tras instalarse: {}", err);
        // Clave estable: el frontend la traduce (auditoría #14). El detalle
        // técnico queda en el log, no en la cara del usuario.
        //
        // Solo se culpa a Windows cuando el proceso NO PUDO EJECUTARSE, que es
        // la firma del bloqueo del sistema. Un arranque lento en un equipo
        // modesto no es lo mismo y no merece un mensaje que acuse al sistema
        // operativo de algo que no hizo.
        let bloqueado_por_windows = cfg!(windows) && local_llm::is_launch_failure(&err);
        return Err(if bloqueado_por_windows {
            "ENGINE_BLOCKED_WINDOWS".to_string()
        } else {
            "ENGINE_WONT_START".to_string()
        });
    }

    emit_progress(app, "done", 0, 0, "Motor local listo");
    Ok(())
}

/// Descarga en streaming a un .part, hasheando en el camino; renombra solo si
/// el SHA256 coincide. Un byte alterado = archivo rechazado y borrado.
async fn download_verified(
    app: &AppHandle,
    stage: &str,
    url: &str,
    dest: &Path,
    expected_sha: &str,
    known_total: u64,
) -> Result<(), String> {
    // Ya existe y verifica: no re-descargar (idempotencia).
    if dest.is_file() && file_sha256(dest)? == expected_sha {
        return Ok(());
    }

    let tmp = dest.with_extension("part");
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Descarga falló: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("Descarga falló con HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(known_total);

    let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut last_emit = std::time::Instant::now();

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Descarga interrumpida: {}", e))?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if last_emit.elapsed().as_millis() > 200 {
            emit_progress(app, stage, downloaded, total, "");
            last_emit = std::time::Instant::now();
        }
    }
    drop(file);

    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_sha {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!(
            "Verificación SHA256 falló para {} (esperado {}, obtenido {}). Descarga descartada.",
            stage, expected_sha, actual
        ));
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    emit_progress(app, stage, total.max(downloaded), total.max(downloaded), "");
    Ok(())
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| e.to_string())?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// Extracción del runtime: `tar` del sistema (arg-vector, sin shell) en
/// macOS/Linux; en Windows el `.zip` se descomprime **en proceso** con el crate
/// `zip` (nada de PowerShell: evita inyección por rutas con comillas/apóstrofos,
/// p.ej. usuarios como `O'Brien`).
fn extract_archive(archive: &Path, dest_dir: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let status = Command::new("tar")
            .arg("xzf")
            .arg(archive)
            .arg("-C")
            .arg(dest_dir)
            .status()
            .map_err(|e| format!("No se pudo ejecutar tar: {}", e))?;
        if !status.success() {
            return Err("La extracción del runtime falló".to_string());
        }
    }
    #[cfg(windows)]
    {
        extract_zip_in_process(archive, dest_dir)?;
    }
    let _ = warn!("Runtime extracted to {}", dest_dir.display());
    Ok(())
}

/// Descomprime un `.zip` sin shell. `enclosed_name()` sanea rutas `..`/absolutas
/// (previene zip-slip); el archivo ya viene verificado por SHA256 antes de aquí.
#[cfg(windows)]
fn extract_zip_in_process(archive: &Path, dest_dir: &Path) -> Result<(), String> {
    let file = std::fs::File::open(archive)
        .map_err(|e| format!("No se pudo abrir el archivo del runtime: {}", e))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|e| format!("El archivo del runtime no es un zip válido: {}", e))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("Error leyendo el zip: {}", e))?;
        let Some(rel) = entry.enclosed_name() else {
            // Entrada con ruta insegura (`..`/absoluta): la ignoramos.
            continue;
        };
        let out_path = dest_dir.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&out_path)
            .map_err(|e| format!("No se pudo escribir {}: {}", out_path.display(), e))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}
