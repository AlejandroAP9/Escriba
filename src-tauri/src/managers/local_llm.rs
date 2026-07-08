use log::{debug, info, warn};
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Idle seconds before the sidecar is unloaded to free RAM (Whisper + LLM
/// residentes compiten por memoria unificada en Macs de 8GB).
const IDLE_UNLOAD_SECS: u64 = 300;
/// First model load can take a while on cold disk; health poll budget.
const STARTUP_TIMEOUT_SECS: u64 = 60;

static MANAGER: OnceLock<LocalLlmManager> = OnceLock::new();

/// Manages the llama-server sidecar: spawn on demand, health-check, idle
/// unload, and guaranteed kill on exit. The server binds strictly to
/// 127.0.0.1 on an ephemeral port and is reached through the same
/// OpenAI-compatible HTTP path as every other post-process provider.
pub struct LocalLlmManager {
    child: Mutex<Option<Child>>,
    port: AtomicU16,
    last_used: Mutex<Instant>,
    runtime_dir: PathBuf,
    models_dir: PathBuf,
    pid_file: PathBuf,
}

/// Initialize the global manager. Call once from Tauri setup with the app
/// data dir. Kills any orphan left behind by a previous crash.
pub fn init(app_data_dir: &Path) {
    let runtime_dir = app_data_dir.join("llm-runtime");
    let models_dir = app_data_dir.join("llm-models");
    let _ = fs::create_dir_all(&runtime_dir);
    let _ = fs::create_dir_all(&models_dir);

    let manager = LocalLlmManager {
        child: Mutex::new(None),
        port: AtomicU16::new(0),
        last_used: Mutex::new(Instant::now()),
        pid_file: runtime_dir.join("llama-server.pid"),
        runtime_dir,
        models_dir,
    };
    manager.kill_orphan_from_pid_file();

    if MANAGER.set(manager).is_err() {
        warn!("LocalLlmManager was already initialized");
        return;
    }

    // Idle watcher: unload the sidecar after IDLE_UNLOAD_SECS without use.
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_secs(15));
        if let Some(mgr) = MANAGER.get() {
            let idle = mgr
                .last_used
                .lock()
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);
            if idle > IDLE_UNLOAD_SECS && mgr.is_running() {
                info!("Local LLM idle for {}s, unloading sidecar", idle);
                mgr.shutdown();
            }
        }
    });
}

pub fn global() -> Option<&'static LocalLlmManager> {
    MANAGER.get()
}

impl LocalLlmManager {
    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    pub fn runtime_dir(&self) -> &Path {
        &self.runtime_dir
    }

    pub fn runtime_installed(&self) -> bool {
        self.find_runtime_binary().is_some()
    }

    pub fn model_installed(&self, model: &str) -> bool {
        self.resolve_model_path(model).is_ok()
    }

    pub fn is_running(&self) -> bool {
        let mut guard = match self.child.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        match guard.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(None) => true,
                _ => {
                    *guard = None;
                    false
                }
            },
            None => false,
        }
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.port.load(Ordering::Relaxed))
    }

    fn touch(&self) {
        if let Ok(mut t) = self.last_used.lock() {
            *t = Instant::now();
        }
    }

    /// Locate the llama-server binary inside runtime_dir (the release
    /// tarball extracts into a versioned subfolder, so search two levels).
    fn find_runtime_binary(&self) -> Option<PathBuf> {
        let name = if cfg!(windows) {
            "llama-server.exe"
        } else {
            "llama-server"
        };
        let direct = self.runtime_dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
        let entries = fs::read_dir(&self.runtime_dir).ok()?;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let candidate = path.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
                // one more level (e.g. build/bin/)
                if let Ok(sub) = fs::read_dir(&path) {
                    for sub_entry in sub.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.is_dir() {
                            let candidate = sub_path.join(name);
                            if candidate.is_file() {
                                return Some(candidate);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn resolve_model_path(&self, model: &str) -> Result<PathBuf, String> {
        let file_name = if model.ends_with(".gguf") {
            model.to_string()
        } else {
            format!("{}.gguf", model)
        };
        let path = self.models_dir.join(&file_name);
        if path.is_file() {
            Ok(path)
        } else {
            Err(format!(
                "Modelo local no encontrado: {} (esperado en {})",
                file_name,
                self.models_dir.display()
            ))
        }
    }

    /// Ensure the sidecar is up and healthy; returns the OpenAI-compatible
    /// base URL (http://127.0.0.1:{port}/v1).
    pub async fn ensure_running(&self, model: &str) -> Result<String, String> {
        self.touch();

        if self.is_running() && health_ok(&self.base_url(), 1500).await {
            return Ok(self.base_url());
        }

        // Not healthy: clean slate before respawn.
        self.shutdown();

        let binary = self
            .find_runtime_binary()
            .ok_or_else(|| "Runtime llama-server no encontrado".to_string())?;
        let model_path = self.resolve_model_path(model)?;

        let port = pick_free_port()?;
        self.port.store(port, Ordering::Relaxed);

        info!(
            "Spawning llama-server on 127.0.0.1:{} with model {}",
            port,
            model_path.display()
        );

        let mut cmd = Command::new(&binary);
        cmd.arg("-m")
            .arg(&model_path)
            .args(["--host", "127.0.0.1"])
            .args(["--port", &port.to_string()])
            .args(["-c", "4096"])
            .arg("--no-webui")
            .arg("--jinja")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("No se pudo iniciar llama-server: {}", e))?;
        let pid = child.id();
        let _ = fs::write(&self.pid_file, pid.to_string());
        if let Ok(mut guard) = self.child.lock() {
            *guard = Some(child);
        }

        // Poll /health until the model is loaded.
        let url = self.base_url();
        let deadline = Instant::now() + Duration::from_secs(STARTUP_TIMEOUT_SECS);
        loop {
            if health_ok(&url, 500).await {
                info!("llama-server healthy at {}", url);
                self.touch();
                return Ok(url);
            }
            if !self.is_running() {
                return Err("llama-server terminó antes de estar listo (¿modelo corrupto o RAM insuficiente?)".to_string());
            }
            if Instant::now() > deadline {
                self.shutdown();
                return Err(format!(
                    "llama-server no respondió /health en {}s",
                    STARTUP_TIMEOUT_SECS
                ));
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Kill the sidecar (idempotent). Called on idle timeout, respawn and app exit.
    pub fn shutdown(&self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut child) = guard.take() {
                debug!("Killing llama-server (pid {})", child.id());
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        let _ = fs::remove_file(&self.pid_file);
    }

    /// If a previous run crashed, its PID file survives; kill that process
    /// so we never leak sidecars.
    fn kill_orphan_from_pid_file(&self) {
        let Ok(contents) = fs::read_to_string(&self.pid_file) else {
            return;
        };
        let Ok(pid) = contents.trim().parse::<u32>() else {
            let _ = fs::remove_file(&self.pid_file);
            return;
        };
        warn!("Found orphan llama-server pid file ({}), killing it", pid);
        #[cfg(unix)]
        {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
        }
        #[cfg(windows)]
        {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .status();
        }
        let _ = fs::remove_file(&self.pid_file);
    }
}

fn pick_free_port() -> Result<u16, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("No se pudo reservar un puerto local: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("No se pudo leer el puerto local: {}", e))?
        .port();
    drop(listener);
    Ok(port)
}

async fn health_ok(base_url: &str, timeout_ms: u64) -> bool {
    let health_url = base_url.trim_end_matches("/v1").to_string() + "/health";
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client.get(&health_url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Pre-warm the sidecar: spawn it and run a 1-token completion so the first
/// real dictation doesn't pay the cold-load + first-inference cost (~60s on
/// a cold Metal pipeline). Fire-and-forget from app startup.
pub async fn warmup(model: &str) {
    let Some(mgr) = MANAGER.get() else { return };
    let base_url = match mgr.ensure_running(model).await {
        Ok(url) => url,
        Err(err) => {
            warn!("Local LLM warmup skipped: {}", err);
            return;
        }
    };
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "hola"}],
        "max_tokens": 1
    });
    let url = format!("{}/chat/completions", base_url);
    match client.post(&url).json(&body).send().await {
        Ok(_) => info!("Local LLM warmed up and ready"),
        Err(err) => warn!("Local LLM warmup request failed: {}", err),
    }
    mgr.touch();
}

/// Kill the sidecar on app exit. Wired into RunEvent::Exit in lib.rs.
pub fn shutdown_on_exit() {
    if let Some(mgr) = MANAGER.get() {
        mgr.shutdown();
    }
}
