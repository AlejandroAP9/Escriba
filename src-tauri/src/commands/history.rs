use crate::actions::{process_transcription_output, TranscribeMode};
use crate::managers::{
    history::{HistoryManager, PaginatedHistory},
    transcription::TranscriptionManager,
};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_history_entries(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    cursor: Option<i64>,
    limit: Option<usize>,
) -> Result<PaginatedHistory, String> {
    history_manager
        .get_history_entries(cursor, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_history_entry_saved(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .toggle_saved_status(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_audio_file_path(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    file_name: String,
) -> Result<String, String> {
    // Nombre conservado por compatibilidad con bindings 2.2.x. Ya no entrega
    // una ruta de disco: devuelve el protocolo privado que descifra por rangos.
    if !history_manager
        .contains_audio_file(&file_name)
        .map_err(|e| e.to_string())?
    {
        return Err("Recording not found in history".to_string());
    }
    let path = history_manager
        .get_audio_file_path(&file_name)
        .map_err(|e| e.to_string())?;
    crate::recording_crypto::ensure_playable(&path).map_err(|e| e.to_string())?;
    crate::recording_crypto::playback_url(&file_name).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .delete_entry(id)
        .await
        .map_err(|e| e.to_string())
}

/// Re-transcribe una entrada del historial. Si `model_id` viene con valor, usa
/// ESE modelo (sin tocar el modelo por defecto del usuario: se restaura en el
/// próximo dictado). Si es `None`, usa el modelo seleccionado actual. Sirve para
/// "misma grabación, más precisión con otro modelo".
#[tauri::command]
#[specta::specta]
pub async fn retry_history_entry_transcription(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    id: i64,
    model_id: Option<String>,
) -> Result<(), String> {
    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;

    let audio_path = history_manager
        .get_audio_file_path(&entry.file_name)
        .map_err(|e| e.to_string())?;
    let samples = crate::recording_crypto::read_wav_samples(&audio_path)
        .map_err(|e| format!("Failed to load audio: {}", e))?;

    if samples.is_empty() {
        return Err("Recording has no audio samples".to_string());
    }

    let default_model = crate::settings::get_settings(&app).selected_model;
    let tm = Arc::clone(&transcription_manager);
    let transcription = tauri::async_runtime::spawn_blocking(move || -> Result<String, String> {
        // Elige el modelo destino: el pedido, o el por defecto.
        let target = model_id.clone().unwrap_or_else(|| default_model.clone());
        if tm.get_current_model().as_deref() != Some(target.as_str()) {
            tm.load_model(&target).map_err(|e| e.to_string())?;
        }
        // Si fue un modelo puntual distinto al del usuario, restáuralo al
        // próximo dictado (no persistimos selected_model).
        if let Some(mid) = &model_id {
            if mid != &default_model {
                tm.reload_model_on_next_use();
            }
        }
        tm.transcribe(samples).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("Transcription task panicked: {}", e))??;

    if transcription.is_empty() {
        return Err("Recording contains no speech".to_string());
    }

    let processed = process_transcription_output(
        &app,
        &transcription,
        if entry.post_process_requested {
            TranscribeMode::PostProcess
        } else {
            TranscribeMode::Standard
        },
    )
    .await;
    history_manager
        .update_transcription(
            id,
            transcription,
            processed.post_processed_text,
            processed.post_process_prompt,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_history_limit(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    limit: usize,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.history_limit = limit;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_recording_retention_period(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    period: String,
) -> Result<(), String> {
    use crate::settings::RecordingRetentionPeriod;

    let retention_period = match period.as_str() {
        "never" => RecordingRetentionPeriod::Never,
        "preserve_limit" => RecordingRetentionPeriod::PreserveLimit,
        "days3" => RecordingRetentionPeriod::Days3,
        "weeks2" => RecordingRetentionPeriod::Weeks2,
        "months3" => RecordingRetentionPeriod::Months3,
        _ => return Err(format!("Invalid retention period: {}", period)),
    };

    let mut settings = crate::settings::get_settings(&app);
    settings.recording_retention_period = retention_period;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn get_usage_stats(
    app: tauri::AppHandle,
) -> Result<crate::managers::history::UsageStats, String> {
    use tauri::Manager;
    let hm = app.state::<std::sync::Arc<crate::managers::history::HistoryManager>>();
    hm.get_usage_stats().map_err(|e| e.to_string())
}

/// Sugerencias de diccionario a partir del historial (idea de Benjamín
/// Carreño, comunidad 16-jul-2026): detecta palabras "inusuales" que el
/// usuario repite en sus dictados — nombres propios, marcas, jerga técnica —
/// y las propone para Palabras personalizadas, donde el post-proceso las
/// respeta siempre. Heurística local, cero red: mayúscula fuera de inicio de
/// frase, mezcla de mayúsculas o dígitos, repetida al menos 3 veces.
#[tauri::command]
#[specta::specta]
pub async fn suggest_custom_words(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<Vec<String>, String> {
    use std::collections::HashMap;

    let existing: Vec<String> = crate::settings::get_settings(&app)
        .custom_words
        .iter()
        .map(|w| w.to_lowercase())
        .collect();

    let page = history_manager
        .get_history_entries(None, Some(200))
        .await
        .map_err(|e| e.to_string())?;

    // (apariciones totales, apariciones "fuertes": capitalizada fuera de
    // inicio de frase, o con dígitos/mayúsculas internas)
    let mut counts: HashMap<String, (u32, u32, String)> = HashMap::new();

    for entry in &page.entries {
        let text = &entry.transcription_text;
        let mut sentence_start = true;
        let mut token = String::new();
        let mut flush = |tok: &mut String, at_sentence_start: bool| {
            if tok.chars().count() >= 4 {
                let first_upper = tok.chars().next().map(char::is_uppercase).unwrap_or(false);
                let has_digit = tok.chars().any(|c| c.is_ascii_digit());
                let inner_upper = tok.chars().skip(1).any(char::is_uppercase);
                let strong = has_digit || inner_upper || (first_upper && !at_sentence_start);
                let key = tok.to_lowercase();
                let e = counts.entry(key).or_insert((0, 0, tok.clone()));
                e.0 += 1;
                if strong {
                    e.1 += 1;
                    // Conservar la grafía "fuerte" (la forma que el usuario quiere)
                    e.2 = tok.clone();
                }
            }
            tok.clear();
        };
        for c in text.chars() {
            if c.is_alphanumeric() {
                token.push(c);
            } else {
                if !token.is_empty() {
                    flush(&mut token, sentence_start);
                    sentence_start = false;
                }
                if matches!(c, '.' | '!' | '?' | '¡' | '¿' | '\n') {
                    sentence_start = true;
                }
            }
        }
        if !token.is_empty() {
            flush(&mut token, sentence_start);
        }
    }

    let mut candidates: Vec<(String, u32)> = counts
        .into_iter()
        .filter(|(key, (total, strong, _))| *total >= 3 && *strong >= 2 && !existing.contains(key))
        .map(|(_, (total, _, display))| (display, total))
        .collect();
    candidates.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(candidates.into_iter().take(8).map(|(w, _)| w).collect())
}
