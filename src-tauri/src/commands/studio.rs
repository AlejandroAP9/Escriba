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
#[tauri::command]
#[specta::specta]
pub fn studio_enqueue(app: AppHandle, paths: Vec<String>) -> Result<Vec<u64>, String> {
    let state = app.state::<Arc<StudioState>>();
    let mut ids = Vec::new();

    for path_str in paths {
        let path = PathBuf::from(&path_str);
        if !decode::supported_extension(&path) {
            continue;
        }
        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        state.jobs.lock().unwrap().push(StudioJob {
            id,
            file_name,
            path: path_str.clone(),
            status: JobStatus::Pending,
            progress: 0.0,
            error: None,
            duration_s: 0.0,
            segments: Vec::new(),
            paragraphs: Vec::new(),
            summary: None,
            model_id: None,
        });
        ids.push(id);
        spawn_job(app.clone(), id, path, None);
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
                if let Some(job) = state.jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
                    job.status = JobStatus::Done;
                    job.progress = 1.0;
                    job.duration_s = duration_s;
                    job.segments = segments;
                    job.paragraphs = paragraphs;
                    job.model_id = model_id.clone();
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
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("El archivo original ya no está disponible en su ubicación".to_string());
    }
    // Resetea el job antes de re-encolar.
    if let Some(job) = state.jobs.lock().unwrap().iter_mut().find(|j| j.id == id) {
        job.status = JobStatus::Pending;
        job.progress = 0.0;
        job.error = None;
        job.segments = Vec::new();
        job.paragraphs = Vec::new();
        job.summary = None;
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
#[tauri::command]
#[specta::specta]
pub fn studio_export_to(
    app: AppHandle,
    id: u64,
    format: String,
    dest: String,
) -> Result<(), String> {
    let content = studio_export(app, id, format)?;
    std::fs::write(&dest, content).map_err(|e| format!("No se pudo guardar el archivo: {}", e))
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
