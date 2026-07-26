//! Voz neural incluida (motor #1 de la lectura en voz alta de Sesiones).
//! Runtime: sherpa-onnx (binario estático arm64) ejecutando la voz Piper
//! es_MX-claude-high. Se descarga bajo demanda con SHA256 pinneado (mismo
//! patrón del motor LLM) y se reproduce con `afplay`. Cascada completa en
//! commands/conversation.rs: Piper → `say` del sistema → speechSynthesis.
//!
//! Alcance v1 (honesto): voz en español y runtime macOS arm64. Otros idiomas
//! y plataformas caen a los respaldos de la cascada.

use futures_util::StreamExt;
use log::info;
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// Release de sherpa-onnx PINNEADA (verificada 13-jul-2026). Actualizarla
/// implica recalcular ambos SHA256; nunca usar "latest".
const RUNTIME_URL: &str = "https://github.com/k2-fsa/sherpa-onnx/releases/download/v1.13.4/sherpa-onnx-v1.13.4-osx-arm64-shared.tar.bz2";
const RUNTIME_SHA256: &str = "809ab5d0c77bd8f358364a244e6ab17f2afecf9779eb9fd436fa469c3ff5375c";
const RUNTIME_DIR: &str = "sherpa-onnx-v1.13.4-osx-arm64-shared";
const RUNTIME_SIZE: u64 = 27_044_587;

/// Una voz Piper (empaquetada por sherpa-onnx con sus tokens y espeak-data),
/// descargable bajo demanda con SHA256 pinneado.
struct Voice {
    /// Carpeta que crea el tarball dentro de `tts/`.
    dir: &'static str,
    /// Nombre del modelo `.onnx` dentro de esa carpeta.
    onnx: &'static str,
    url: &'static str,
    sha256: &'static str,
    size: u64,
}

/// Catálogo de voces neurales, indexado por código ISO de idioma.
///
/// Era un `match` con los datos incrustados en cada brazo. Como tabla, añadir
/// un idioma es una fila y no tocar la lógica.
///
/// **Para añadir una voz**, en este orden:
///  1. Elegir el tarball en el repo de modelos de sherpa-onnx (mismo release
///     `tts-models` que los de abajo).
///  2. Descargarlo y calcular su SHA256 y su tamaño exacto en bytes.
///  3. Añadir la fila con esos valores reales.
///
/// El hash NO es opcional ni se puede aproximar: `download_verified` aborta si
/// no cuadra, así que una fila con un hash inventado deja la voz permanentemente
/// rota para ese idioma. Por eso aquí solo están las dos voces verificadas
/// (13-jul-2026) y el resto de idiomas cae a la voz del sistema, que siempre
/// funciona aunque suene peor.
const VOICES: &[(&str, Voice)] = &[
    (
        "es",
        Voice {
            dir: "vits-piper-es_MX-claude-high",
            onnx: "es_MX-claude-high.onnx",
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-es_MX-claude-high.tar.bz2",
            sha256: "ec33fb689c248fe64810aab564cba97babf0f506672cfd404928d46e751a4721",
            size: 67_207_890,
        },
    ),
    (
        "en",
        Voice {
            dir: "vits-piper-en_US-hfc_female-medium",
            onnx: "en_US-hfc_female-medium.onnx",
            url: "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/vits-piper-en_US-hfc_female-medium.tar.bz2",
            sha256: "3fffdceb0c65bd9415a085d09c3cb88cc82f9d74a6ca453f8ce7fc5eaee81ff8",
            size: 67_228_166,
        },
    ),
];

/// Voz neural para un idioma, o `None` si no hay (cae a la voz del sistema).
/// Acepta tanto `es` como `es-CL` o `en_US`: se compara por el código base.
fn voice_for(lang: &str) -> Option<Voice> {
    let base = lang.split(['-', '_']).next().unwrap_or(lang);
    VOICES
        .iter()
        .find(|(code, _)| *code == base)
        .map(|(_, voice)| Voice { ..*voice })
}

/// Reproducción en curso (afplay): se corta antes de hablar de nuevo.
static PLAYING: Mutex<Option<Child>> = Mutex::new(None);

fn base_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(crate::portable::app_data_dir(app)
        .map_err(|e| e.to_string())?
        .join("tts"))
}

fn tts_bin(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(base_dir(app)?
        .join(RUNTIME_DIR)
        .join("bin")
        .join("sherpa-onnx-offline-tts"))
}

fn voice_dir_named(app: &AppHandle, dir: &str) -> Result<PathBuf, String> {
    Ok(base_dir(app)?.join(dir))
}

/// ¿Runtime + voz de este idioma listos para hablar? `None` si el idioma no
/// tiene voz neural (cae a la voz del sistema).
pub fn installed_lang(app: &AppHandle, lang: &str) -> bool {
    let Some(voice) = voice_for(lang) else {
        return false;
    };
    match (tts_bin(app), voice_dir_named(app, voice.dir)) {
        (Ok(bin), Ok(dir)) => bin.is_file() && dir.join(voice.onnx).is_file(),
        _ => false,
    }
}

/// ¿Está lista la voz española incluida? (compatibilidad: Sesiones y Traductor.)
pub fn installed(app: &AppHandle) -> bool {
    installed_lang(app, "es")
}

fn emit_progress(app: &AppHandle, stage: &str, downloaded: u64, total: u64, message: &str) {
    let _ = app.emit(
        "tts-setup-progress",
        serde_json::json!({
            "stage": stage,
            "downloaded": downloaded,
            "total": total,
            "message": message,
        }),
    );
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Descarga con progreso y verificación SHA256 (patrón del motor LLM: un byte
/// alterado = archivo rechazado y borrado).
async fn download_verified(
    app: &AppHandle,
    stage: &str,
    url: &str,
    dest: &Path,
    expected_sha: &str,
    known_total: u64,
) -> Result<(), String> {
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
        return Err("La descarga no pasó la verificación de integridad".to_string());
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(())
}

/// Extrae un .tar.bz2 con el tar del sistema (bsdtar de macOS trae bzip2).
fn extract_tar_bz2(archive: &Path, dest_dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest_dir).map_err(|e| e.to_string())?;
    let status = Command::new("/usr/bin/tar")
        .arg("xjf")
        .arg(archive)
        .arg("-C")
        .arg(dest_dir)
        .status()
        .map_err(|e| format!("No se pudo extraer: {}", e))?;
    if !status.success() {
        return Err("La extracción falló".to_string());
    }
    Ok(())
}

/// Re-firma ad-hoc el runtime extraído: macOS (arm64) mata con SIGKILL los
/// binarios cuya firma no calza tras la descarga (verificado en el spike).
fn resign_runtime(runtime_dir: &Path) -> Result<(), String> {
    let _ = Command::new("/usr/bin/xattr")
        .arg("-cr")
        .arg(runtime_dir)
        .status();
    let mut targets: Vec<PathBuf> = vec![runtime_dir.join("bin").join("sherpa-onnx-offline-tts")];
    if let Ok(entries) = std::fs::read_dir(runtime_dir.join("lib")) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "dylib").unwrap_or(false) {
                targets.push(p);
            }
        }
    }
    for target in targets {
        if target.exists() {
            let _ = Command::new("/usr/bin/codesign")
                .args(["-s", "-", "-f"])
                .arg(&target)
                .status();
        }
    }
    Ok(())
}

/// Descarga runtime + voz española, verifica, extrae y re-firma. Idempotente.
/// (Compatibilidad: es el disparador de la voz incluida de Sesiones.)
pub async fn setup(app: &AppHandle) -> Result<(), String> {
    setup_lang(app, "es").await
}

/// Igual que `setup`, pero para el idioma dado (voz del Intérprete). El
/// runtime es compartido; solo cambia la voz que se baja.
pub async fn setup_lang(app: &AppHandle, lang: &str) -> Result<(), String> {
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = (app, lang);
        return Err("La voz neural v1 es solo para macOS Apple Silicon".to_string());
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let voice = voice_for(lang).ok_or_else(|| "Idioma sin voz neural".to_string())?;
        let dir = base_dir(app)?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        if !tts_bin(app)?.is_file() {
            let archive = dir.join("runtime.tar.bz2");
            download_verified(
                app,
                "runtime",
                RUNTIME_URL,
                &archive,
                RUNTIME_SHA256,
                RUNTIME_SIZE,
            )
            .await?;
            emit_progress(app, "extract", 0, 0, "Extrayendo runtime");
            extract_tar_bz2(&archive, &dir)?;
            resign_runtime(&dir.join(RUNTIME_DIR))?;
            let _ = std::fs::remove_file(&archive);
        }

        if !voice_dir_named(app, voice.dir)?.join(voice.onnx).is_file() {
            let archive = dir.join("voice.tar.bz2");
            download_verified(app, "voice", voice.url, &archive, voice.sha256, voice.size).await?;
            emit_progress(app, "extract", 0, 0, "Extrayendo voz");
            extract_tar_bz2(&archive, &dir)?;
            let _ = std::fs::remove_file(&archive);
        }

        info!("Voz neural lista (sherpa-onnx + {})", voice.dir);
        emit_progress(app, "done", 0, 0, "Voz neural lista");
        Ok(())
    }
}

/// Sintetiza `text` en el idioma dado a un WAV en `out_path` con la voz
/// neural (bloqueante). Sin reproducir: el llamador la reproduce donde quiera
/// (p. ej. el Intérprete la enruta al micrófono virtual). `Err` si el idioma
/// no tiene voz neural o no está instalada.
pub fn synth_to_wav(
    app: &AppHandle,
    text: &str,
    lang: &str,
    out_path: &Path,
) -> Result<(), String> {
    let voice = voice_for(lang).ok_or_else(|| "Idioma sin voz neural".to_string())?;
    let bin = tts_bin(app)?;
    let dir = voice_dir_named(app, voice.dir)?;
    let status = Command::new(&bin)
        .arg(format!("--vits-model={}", dir.join(voice.onnx).display()))
        .arg(format!(
            "--vits-tokens={}",
            dir.join("tokens.txt").display()
        ))
        .arg(format!(
            "--vits-data-dir={}",
            dir.join("espeak-ng-data").display()
        ))
        .arg(format!("--output-filename={}", out_path.display()))
        .arg(text)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("La síntesis falló: {}", e))?;
    if !status.success() || !out_path.is_file() {
        return Err("La síntesis falló".to_string());
    }
    Ok(())
}

/// Sintetiza y reproduce (bloqueante en la síntesis, ~1-2.5 s). Llamar desde
/// spawn_blocking. Corta cualquier reproducción anterior.
pub fn speak_blocking(app: &AppHandle, text: &str) -> Result<(), String> {
    let wav = base_dir(app)?.join("speak.wav");

    stop();
    synth_to_wav(app, text, "es", &wav)?;

    let child = Command::new("/usr/bin/afplay")
        .arg(&wav)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("No se pudo reproducir: {}", e))?;
    *PLAYING.lock().unwrap() = Some(child);
    Ok(())
}

/// ¿Hay una reproducción de la voz incluida en curso?
pub fn is_playing() -> bool {
    if let Ok(mut guard) = PLAYING.lock() {
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => return true,
                _ => {
                    *guard = None;
                }
            }
        }
    }
    false
}

/// Detiene la reproducción en curso (si la hay).
pub fn stop() {
    if let Ok(mut guard) = PLAYING.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
