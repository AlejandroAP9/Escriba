//! Comandos del Estudio de transcripción: encolar archivos, consultar
//! estado, exportar y resumir. La transcripción corre en un worker aparte
//! (nunca en el hilo del comando) y emite eventos `studio-progress`.

use crate::managers::transcription::TranscriptionManager;
use crate::studio::{decode, export, pipeline, segments::Segment};
use log::info;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize, Deserialize, Clone, Type, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Processing,
    Done,
    Error,
}

#[derive(Serialize, Clone, Type)]
pub struct StudioJob {
    pub id: u64,
    pub file_name: String,
    pub path: String,
    pub status: JobStatus,
    pub progress: f32,
    pub error: Option<String>,
    pub duration_s: f64,
    #[specta(skip)]
    #[serde(skip)]
    pub segments: Vec<Segment>,
    pub paragraphs: Vec<String>,
    pub summary: Option<String>,
    /// Modelo con el que se produjo esta transcripción (para mostrar el recibo
    /// "mismo audio, modelo X" al re-transcribir). `None` = modelo por defecto.
    pub model_id: Option<String>,
    /// Hubo tramos donde el modelo estaba adivinando (confianza media por debajo
    /// del umbral). Sirve para avisar al usuario de que conviene repasar antes
    /// de exportar, en vez de que descubra la alucinación en el subtítulo ya
    /// publicado. `false` también cuando el motor no reporta confianza.
    pub low_confidence: bool,
}

#[derive(Default)]
pub struct StudioState {
    pub jobs: Mutex<Vec<StudioJob>>,
}

#[derive(Serialize, Clone, Type)]
struct StudioProgress {
    id: u64,
    status: JobStatus,
    progress: f32,
    error: Option<String>,
}

fn emit(app: &AppHandle, id: u64, status: JobStatus, progress: f32, error: Option<String>) {
    let _ = app.emit(
        "studio-progress",
        StudioProgress {
            id,
            status,
            progress,
            error,
        },
    );
}

/// Encola archivos y arranca su transcripción en un worker. Devuelve los ids.
///
/// La ruta se valida además de la extensión. Antes el único filtro era el sufijo
/// del archivo, así que este comando transcribía CUALQUIER audio o video del
/// disco y devolvía el texto en `studio_jobs`: un oráculo de lectura para un
/// frontend comprometido. Se guarda la ruta ya canonicalizada, para que
/// `studio_retranscribe` reabra la que se validó y no la cadena original.
#[tauri::command]
#[specta::specta]
pub fn studio_enqueue(app: AppHandle, paths: Vec<String>) -> Result<Vec<u64>, String> {
    let state = app.state::<Arc<StudioState>>();
    let mut ids = Vec::new();
    let mut rejected = 0usize;

    for path_str in paths {
        let raw = PathBuf::from(&path_str);
        if !decode::supported_extension(&raw) {
            continue;
        }
        // Mensaje único para "no existe" y "fuera de límites": quien llama no
        // puede usar el error para averiguar qué archivos hay en el disco.
        let path =
            match crate::path_guard::contain_media_path(&app, &raw, "No se pudo abrir el archivo.")
            {
                Ok(p) => p,
                Err(_) => {
                    info!("Estudio: ruta rechazada, fuera de las carpetas permitidas");
                    rejected += 1;
                    continue;
                }
            };

        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        state.jobs.lock().unwrap().push(StudioJob {
            id,
            file_name,
            path: path.to_string_lossy().to_string(),
            status: JobStatus::Pending,
            progress: 0.0,
            error: None,
            duration_s: 0.0,
            segments: Vec::new(),
            paragraphs: Vec::new(),
            summary: None,
            model_id: None,
            low_confidence: false,
        });
        ids.push(id);
        spawn_job(app.clone(), id, path, None);
    }

    // Descartar archivos en silencio es desconcertante: el usuario arrastra
    // tres y aparecen dos. Se avisa sin decir cuál ni por qué, que es lo que
    // convertiría el aviso en el oráculo que acabamos de cerrar.
    if rejected > 0 {
        let _ = app.emit("studio-paths-rejected", rejected);
    }
    Ok(ids)
}

fn spawn_job(app: AppHandle, id: u64, path: PathBuf, model_id: Option<String>) {
    std::thread::spawn(move || {
        let state = app.state::<Arc<StudioState>>();
        let tm = app.state::<Arc<TranscriptionManager>>();
        set_status(&state, id, JobStatus::Processing);
        emit(&app, id, JobStatus::Processing, 0.0, None);

        let result = (|| -> Result<(Vec<Segment>, f64), String> {
            // Modelo explícito (re-transcribir): cárgalo y restaura el del
            // usuario en el próximo dictado. Sin valor = modelo por defecto.
            if let Some(mid) = &model_id {
                let default_model = crate::settings::get_settings(&app).selected_model;
                if tm.get_current_model().as_deref() != Some(mid.as_str()) {
                    tm.load_model(mid).map_err(|e| e.to_string())?;
                }
                if mid != &default_model {
                    tm.reload_model_on_next_use();
                }
            }
            let samples = decode::decode_to_16k_mono(&path)?;
            let duration_s = samples.len() as f64 / decode::STUDIO_SAMPLE_RATE as f64;
            let app_for_progress = app.clone();
            let segments = pipeline::transcribe_samples(&tm, &samples, |p| {
                emit(&app_for_progress, id, JobStatus::Processing, p, None);
            })?;
            Ok((segments, duration_s))
        })();

        match result {
            Ok((segments, duration_s)) => {
                let paragraphs = crate::studio::segments::group_paragraphs(&segments);
                // Basta con que un solo tramo venga dudoso para avisar: el
                // usuario tiene que releer antes de exportar un subtítulo.
                let low_confidence = segments.iter().any(|s| s.is_low_confidence());
                if let Some(job) = state.jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
                    job.status = JobStatus::Done;
                    job.progress = 1.0;
                    job.duration_s = duration_s;
                    job.segments = segments;
                    job.paragraphs = paragraphs;
                    job.model_id = model_id.clone();
                    job.low_confidence = low_confidence;
                }
                info!("Studio job {} done", id);
                emit(&app, id, JobStatus::Done, 1.0, None);
            }
            Err(e) => {
                if let Some(job) = state.jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
                    job.status = JobStatus::Error;
                    job.error = Some(e.clone());
                }
                emit(&app, id, JobStatus::Error, 0.0, Some(e));
            }
        }
    });
}

fn set_status(state: &Arc<StudioState>, id: u64, status: JobStatus) {
    if let Some(job) = state.jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
        job.status = status;
    }
}

#[tauri::command]
#[specta::specta]
pub fn studio_jobs(app: AppHandle) -> Vec<StudioJob> {
    app.state::<Arc<StudioState>>().jobs.lock().unwrap().clone()
}

/// Re-transcribe un job ya terminado con OTRO modelo. Re-decodifica el archivo
/// original (nunca se copió a la app, así que debe seguir en su ruta) y corre la
/// transcripción de nuevo. `model_id = None` usa el modelo por defecto.
#[tauri::command]
#[specta::specta]
pub fn studio_retranscribe(
    app: AppHandle,
    id: u64,
    model_id: Option<String>,
) -> Result<(), String> {
    let state = app.state::<Arc<StudioState>>();
    let path = {
        let jobs = state.jobs.lock().unwrap();
        let job = jobs
            .iter()
            .find(|j| j.id == id)
            .ok_or("Job no encontrado")?;
        job.path.clone()
    };
    // La ruta guardada ya venía validada del encolado, pero se revalida: entre
    // una transcripción y otra el archivo pudo moverse, o su carpeta pudo pasar
    // a ser un enlace a otro sitio. La comprobación es barata y evita que la
    // validación dependa de un dato guardado hace rato.
    let path_buf = crate::path_guard::contain_media_path(
        &app,
        &PathBuf::from(&path),
        "El archivo original ya no está disponible en su ubicación",
    )?;
    // Resetea el job antes de re-encolar.
    if let Some(job) = state.jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
        job.status = JobStatus::Pending;
        job.progress = 0.0;
        job.error = None;
        job.segments = Vec::new();
        job.paragraphs = Vec::new();
        job.summary = None;
        job.low_confidence = false;
    }
    spawn_job(app.clone(), id, path_buf, model_id);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn studio_remove_job(app: AppHandle, id: u64) {
    app.state::<Arc<StudioState>>()
        .jobs
        .lock()
        .unwrap()
        .retain(|j| j.id != id);
}

/// Exporta un job terminado al formato pedido y devuelve el contenido para
/// que el frontend lo guarde con el diálogo nativo.
#[tauri::command]
#[specta::specta]
pub fn studio_export(app: AppHandle, id: u64, format: String) -> Result<String, String> {
    let jobs = app.state::<Arc<StudioState>>();
    let jobs = jobs.jobs.lock().unwrap();
    let job = jobs
        .iter()
        .find(|j| j.id == id)
        .ok_or("Job no encontrado")?;
    if job.status != JobStatus::Done {
        return Err("La transcripción aún no está lista".to_string());
    }
    match format.as_str() {
        "srt" => Ok(export::to_srt(&job.segments)),
        "vtt" => Ok(export::to_vtt(&job.segments)),
        "txt" => Ok(export::to_txt(&job.paragraphs)),
        "json" => export::to_json(&job.segments),
        _ => Err(format!("Formato no soportado: {}", format)),
    }
}

/// Exporta un job y lo ESCRIBE en la ruta que el usuario eligió con el diálogo
/// nativo. La escritura ocurre en el backend (std::fs), no en el webview, para
/// que la app no necesite permisos de escritura al home en la capa de la UI.
///
/// El backend no puede comprobar que `dest` venga de verdad del diálogo nativo,
/// así que además contiene la ruta al home del usuario: sin eso, este comando
/// escribía bytes arbitrarios en cualquier ruta absoluta (`~/.zshrc`, un
/// LaunchAgent, `~/.ssh/authorized_keys`).
#[tauri::command]
#[specta::specta]
pub fn studio_export_to(
    app: AppHandle,
    id: u64,
    format: String,
    dest: String,
) -> Result<(), String> {
    let safe_dest = crate::path_guard::contain_new_path(
        &app,
        std::path::Path::new(&dest),
        "No se pudo guardar ahí. Elige una carpeta dentro de tu carpeta personal.",
    )?;
    let content = studio_export(app, id, format)?;
    std::fs::write(&safe_dest, content).map_err(|e| format!("No se pudo guardar el archivo: {}", e))
}

/// Resume la transcripción con el motor de IA local (misma cascada del phraser).
#[tauri::command]
#[specta::specta]
pub async fn studio_summarize(app: AppHandle, id: u64) -> Result<String, String> {
    let full_text = {
        let jobs = app.state::<Arc<StudioState>>();
        let jobs = jobs.jobs.lock().unwrap();
        let job = jobs
            .iter()
            .find(|j| j.id == id)
            .ok_or("Job no encontrado")?;
        if job.status != JobStatus::Done {
            return Err("La transcripción aún no está lista".to_string());
        }
        job.paragraphs.join("\n\n")
    };

    let summary = crate::actions::summarize_text(&app, &full_text)
        .await
        .ok_or("No se pudo generar el resumen (¿motor local disponible?)")?;

    if let Some(job) = app
        .state::<Arc<StudioState>>()
        .jobs
        .lock()
        .unwrap()
        .iter_mut()
        .find(|j| j.id == id)
    {
        job.summary = Some(summary.clone());
    }
    Ok(summary)
}
