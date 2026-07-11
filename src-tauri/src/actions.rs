#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::apple_intelligence;
use crate::audio_feedback::{play_feedback_sound, play_feedback_sound_blocking, SoundType};
use crate::audio_toolkit::{is_microphone_access_denied, is_no_input_device_error, VadPolicy};
use crate::managers::audio::AudioRecordingManager;
use crate::managers::history::HistoryManager;
use crate::managers::model::ModelManager;
use crate::managers::transcription::StreamWorkKind;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, AppSettings, OverlayStyle, APPLE_INTELLIGENCE_PROVIDER_ID};
use crate::shortcut;
use crate::tray::{change_tray_icon, TrayIconState};
use crate::utils::{
    self, show_processing_overlay, show_recording_overlay, show_transcribing_overlay,
};
use crate::TranscriptionCoordinator;
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use log::{debug, error, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Manager;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
struct RecordingErrorEvent {
    error_type: String,
    detail: Option<String>,
}

/// Drop guard that notifies the [`TranscriptionCoordinator`] when the
/// transcription pipeline finishes — whether it completes normally or panics.
struct FinishGuard(AppHandle);
impl Drop for FinishGuard {
    fn drop(&mut self) {
        if let Some(c) = self.0.try_state::<TranscriptionCoordinator>() {
            c.notify_processing_finished();
        }
    }
}

// Shortcut Action Trait
pub trait ShortcutAction: Send + Sync {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
}

// Transcribe Action
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum TranscribeMode {
    Standard,
    PostProcess,
    Translate,
    Edit,
}

/// Seleccion capturada al iniciar voice_edit (Cmd/Ctrl+C sintetico). Solo se
/// guarda la longitud en logs, jamas el contenido.
static EDIT_SELECTION: Mutex<Option<String>> = Mutex::new(None);

/// Copia la seleccion actual sin perder el clipboard del usuario: lee el
/// clipboard previo, manda Cmd/Ctrl+C, espera, lee de nuevo y SIEMPRE
/// restaura el original. Si nada cambio, no habia seleccion.
fn capture_selection(app: &AppHandle) -> Option<String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let clipboard = app.clipboard();
    let previous = clipboard.read_text().unwrap_or_default();

    let sent = {
        let enigo_state = app.try_state::<crate::input::EnigoState>()?;
        let mut enigo = enigo_state.0.lock().ok()?;
        crate::input::send_copy_ctrl_c(&mut enigo).is_ok()
    };
    if !sent {
        return None;
    }
    std::thread::sleep(std::time::Duration::from_millis(180));
    let captured = clipboard.read_text().unwrap_or_default();

    // Restaurar SIEMPRE el clipboard del usuario (premortem PRP-003).
    let _ = clipboard.write_text(previous.clone());

    if captured.is_empty() || captured == previous {
        debug!("voice_edit: no selection detected");
        return None;
    }
    debug!("voice_edit: captured selection ({} chars)", captured.len());
    Some(captured)
}

struct TranscribeAction {
    mode: TranscribeMode,
}

/// Field name for structured output JSON schema
const TRANSCRIPTION_FIELD: &str = "transcription";

/// Strip invisible Unicode characters that some LLMs may insert
fn strip_invisible_chars(s: &str) -> String {
    s.replace(['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'], "")
}

/// Build a system prompt from the user's prompt template.
/// Removes `${output}` placeholder since the transcription is sent as the user message.
fn build_system_prompt(prompt_template: &str) -> String {
    prompt_template.replace("${output}", "").trim().to_string()
}

/// Returns `true` when a transcription has no meaningful content to
/// post-process (empty or whitespace-only). Used to skip the post-processing
/// LLM call when nothing was actually transcribed, which would otherwise make
/// the model reply with an error message such as "you need to provide the
/// transcription".
fn is_blank_transcription(transcription: &str) -> bool {
    transcription.trim().is_empty()
}

async fn post_process_transcription(
    app: &AppHandle,
    settings: &AppSettings,
    transcription: &str,
    mode: TranscribeMode,
) -> Option<String> {
    if is_blank_transcription(transcription) {
        debug!("Post-processing skipped because the transcription is empty");
        return None;
    }

    let provider = match settings.active_post_process_provider().cloned() {
        Some(provider) => provider,
        None => {
            debug!("Post-processing enabled but no provider is selected");
            return None;
        }
    };

    let model = settings
        .post_process_models
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    if model.trim().is_empty() {
        debug!(
            "Post-processing skipped because provider '{}' has no model configured",
            provider.id
        );
        return None;
    }

    // Modo traduccion: plantilla constante propia (no depende ni del prompt
    // elegido ni de que el usuario la pueda borrar). El resto del pipeline
    // (cascada local, structured output, historial) es identico.
    // Texto objetivo del pipeline: lo dictado, salvo en Edit (la seleccion).
    let mut target_text = transcription.to_string();

    // Modo edicion por voz: la instruccion es lo dictado; el objetivo es la
    // seleccion capturada. Sin seleccion -> aviso y NO tocar nada (premortem).
    let prompt = if mode == TranscribeMode::Edit {
        let selection = EDIT_SELECTION.lock().ok().and_then(|mut g| g.take());
        let Some(selection) = selection else {
            notify_fallback(app, "no_selection", "");
            return None;
        };
        if selection.chars().count() > 8000 {
            notify_fallback(app, "selection_too_long", "");
            return None;
        }
        let instruction = transcription.trim().to_string();
        if instruction.is_empty() {
            notify_fallback(app, "no_selection", "");
            return None;
        }
        target_text = selection;
        format!(
            "Aplica esta instruccion al texto. Conserva el idioma del texto salvo que la instruccion pida traducir. Responde UNICAMENTE con el texto resultante, sin explicaciones.\n\nInstruccion: {}\n\nTexto:\n${{output}}",
            instruction
        )
    } else if mode == TranscribeMode::Translate {
        let target = settings.translation_target_language.trim();
        let target = if target.is_empty() { "en" } else { target };
        format!(
            "Traduce el siguiente dictado al idioma con codigo ISO '{}'. Conserva el significado, tono y formato exactos; elimina muletillas y repeticiones accidentales; usa redaccion natural de hablante nativo. Responde UNICAMENTE con la traduccion.\n\nDictado:\n${{output}}",
            target
        )
    } else {
        let selected_prompt_id = match &settings.post_process_selected_prompt_id {
            Some(id) => id.clone(),
            None => {
                debug!("Post-processing skipped because no prompt is selected");
                return None;
            }
        };

        match settings
            .post_process_prompts
            .iter()
            .find(|prompt| prompt.id == selected_prompt_id)
        {
            Some(prompt) => prompt.prompt.clone(),
            None => {
                debug!(
                    "Post-processing skipped because prompt '{}' was not found",
                    selected_prompt_id
                );
                return None;
            }
        }
    };

    if prompt.trim().is_empty() {
        debug!("Post-processing skipped because the selected prompt is empty");
        return None;
    }

    // Escriba: diccionario personal conectado al phraser. Los terminos del
    // usuario (nombres propios, jerga, marcas) se respetan en la correccion.
    let prompt = if settings.custom_words.is_empty() {
        prompt
    } else {
        format!(
            "{}

Vocabulario personal del usuario (escribir SIEMPRE exactamente asi, sin corregir): {}",
            prompt,
            settings.custom_words.join(", ")
        )
    };

    debug!(
        "Starting LLM post-processing with provider '{}' (model: {})",
        provider.id, model
    );

    let api_key = settings
        .post_process_api_keys
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    // Disable reasoning for providers where post-processing rarely benefits from it.
    // - custom: top-level reasoning_effort (works for local OpenAI-compat servers)
    // - openrouter: nested reasoning object; exclude:true also keeps reasoning text
    //   out of the response so it can't pollute structured-output JSON parsing
    // Escriba: cascada del motor local. Orden: sidecar llama-server ->
    // Ollama detectado -> Apple Intelligence -> texto crudo con aviso.
    // BYOK (OpenAI/Groq/etc.) JAMAS entra automaticamente: cero nube sorpresa.
    let mut provider = provider;
    let mut model = model;
    if provider.id == crate::settings::LOCAL_LLM_PROVIDER_ID {
        match resolve_local_route(&model).await {
            LocalRoute::Sidecar(base_url) => {
                debug!("Local LLM sidecar ready at {}", base_url);
                provider.base_url = base_url;
            }
            LocalRoute::Ollama {
                base_url,
                model: ollama_model,
            } => {
                warn!(
                    "Local sidecar unavailable; falling back to Ollama (model {})",
                    ollama_model
                );
                notify_fallback(app, "ollama", &ollama_model);
                provider.base_url = base_url;
                // Modo legacy con Ollama: modelos desconocidos pueden ignorar
                // json_schema; el prompt ${output} funciona con cualquiera.
                provider.supports_structured_output = false;
                model = ollama_model;
            }
            LocalRoute::AppleIntelligence => {
                warn!("Local sidecar and Ollama unavailable; falling back to Apple Intelligence");
                notify_fallback(app, "apple_intelligence", "");
                provider.id = APPLE_INTELLIGENCE_PROVIDER_ID.to_string();
                provider.supports_structured_output = true;
            }
            LocalRoute::Unavailable(reason) => {
                error!("No local post-process route available: {}", reason);
                notify_fallback(app, "raw", &reason);
                return None;
            }
        }
    }
    let provider = provider;
    let model = model;

    // Post-procesado determinista para el motor local: sin temperatura,
    // llama-server usa ~0.8 y los modelos chicos alucinan (pierden palabras,
    // cambian signos). 0.2 replica el comportamiento validado en el spike.
    let temperature = if provider.id == crate::settings::LOCAL_LLM_PROVIDER_ID {
        Some(0.2_f32)
    } else {
        None
    };

    let (reasoning_effort, reasoning) = match provider.id.as_str() {
        "custom" | crate::settings::LOCAL_LLM_PROVIDER_ID => (Some("none".to_string()), None),
        "openrouter" => (
            None,
            Some(crate::llm_client::ReasoningConfig {
                effort: Some("none".to_string()),
                exclude: Some(true),
            }),
        ),
        _ => (None, None),
    };

    if provider.supports_structured_output {
        debug!("Using structured outputs for provider '{}'", provider.id);

        let system_prompt = build_system_prompt(&prompt);
        let user_content = target_text.clone();

        // Handle Apple Intelligence separately since it uses native Swift APIs
        if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                if !apple_intelligence::check_apple_intelligence_availability() {
                    debug!(
                        "Apple Intelligence selected but not currently available on this device"
                    );
                    return None;
                }

                let token_limit = model.trim().parse::<i32>().unwrap_or(0);
                return match apple_intelligence::process_text_with_system_prompt(
                    &system_prompt,
                    &user_content,
                    token_limit,
                ) {
                    Ok(result) => {
                        if result.trim().is_empty() {
                            debug!("Apple Intelligence returned an empty response");
                            None
                        } else {
                            let result = strip_invisible_chars(&result);
                            debug!(
                                "Apple Intelligence post-processing succeeded. Output length: {} chars",
                                result.len()
                            );
                            Some(result)
                        }
                    }
                    Err(err) => {
                        error!("Apple Intelligence post-processing failed: {}", err);
                        None
                    }
                };
            }

            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                debug!("Apple Intelligence provider selected on unsupported platform");
                return None;
            }
        }

        // Define JSON schema for transcription output
        let json_schema = serde_json::json!({
            "type": "object",
            "properties": {
                (TRANSCRIPTION_FIELD): {
                    "type": "string",
                    "description": "The cleaned and processed transcription text"
                }
            },
            "required": [TRANSCRIPTION_FIELD],
            "additionalProperties": false
        });

        match crate::llm_client::send_chat_completion_with_schema(
            &provider,
            api_key.clone(),
            &model,
            user_content,
            Some(system_prompt),
            Some(json_schema),
            reasoning_effort.clone(),
            reasoning.clone(),
            temperature,
        )
        .await
        {
            Ok(Some(content)) => {
                // Parse the JSON response to extract the transcription field
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        if let Some(transcription_value) =
                            json.get(TRANSCRIPTION_FIELD).and_then(|t| t.as_str())
                        {
                            let result = strip_invisible_chars(transcription_value);
                            debug!(
                                "Structured output post-processing succeeded for provider '{}'. Output length: {} chars",
                                provider.id,
                                result.len()
                            );
                            return Some(result);
                        } else {
                            error!("Structured output response missing 'transcription' field");
                            return Some(strip_invisible_chars(&content));
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse structured output JSON: {}. Returning raw content.",
                            e
                        );
                        return Some(strip_invisible_chars(&content));
                    }
                }
            }
            Ok(None) => {
                error!("LLM API response has no content");
                return None;
            }
            Err(e) => {
                warn!(
                    "Structured output failed for provider '{}': {}. Falling back to legacy mode.",
                    provider.id, e
                );
                // Fall through to legacy mode below
            }
        }
    }

    // Legacy mode: Replace ${output} variable in the prompt with the actual text
    let processed_prompt = prompt.replace("${output}", &target_text);
    debug!("Processed prompt length: {} chars", processed_prompt.len());

    match crate::llm_client::send_chat_completion(
        &provider,
        api_key,
        &model,
        processed_prompt,
        reasoning_effort,
        reasoning,
        temperature,
    )
    .await
    {
        Ok(Some(content)) => {
            let content = strip_invisible_chars(&content);
            debug!(
                "LLM post-processing succeeded for provider '{}'. Output length: {} chars",
                provider.id,
                content.len()
            );
            Some(content)
        }
        Ok(None) => {
            error!("LLM API response has no content");
            None
        }
        Err(e) => {
            error!(
                "LLM post-processing failed for provider '{}': {}. Falling back to original transcription.",
                provider.id,
                e
            );
            None
        }
    }
}

async fn maybe_convert_chinese_variant(
    effective_language: &str,
    transcription: &str,
) -> Option<String> {
    // Gate on the language the model actually transcribed in (the effective
    // language), not the persisted intent. A leftover zh-Hans/zh-Hant intent
    // from a previously selected model must not run OpenCC S2T/T2S over output a
    // non-Chinese model produced — that would silently rewrite any shared CJK
    // characters (e.g. Japanese kanji) in the result.
    let is_simplified = effective_language == "zh-Hans";
    let is_traditional = effective_language == "zh-Hant";

    if !is_simplified && !is_traditional {
        debug!("effective language is not Simplified or Traditional Chinese; skipping conversion");
        return None;
    }

    debug!(
        "Starting Chinese variant conversion using OpenCC for language: {}",
        effective_language
    );

    // Use OpenCC to convert based on selected language
    let config = if is_simplified {
        // Convert Traditional Chinese to Simplified Chinese
        BuiltinConfig::Tw2sp
    } else {
        // Convert Simplified Chinese to Traditional Chinese
        BuiltinConfig::S2tw
    };

    match OpenCC::from_config(config) {
        Ok(converter) => {
            let converted = converter.convert(transcription);
            debug!(
                "OpenCC translation completed. Input length: {}, Output length: {}",
                transcription.len(),
                converted.len()
            );
            Some(converted)
        }
        Err(e) => {
            error!("Failed to initialize OpenCC converter: {}. Falling back to original transcription.", e);
            None
        }
    }
}

pub(crate) struct ProcessedTranscription {
    pub final_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
    #[allow(dead_code)]
    pub interpreter_published: bool,
}

/// Resolve the persisted language *intent* into the language the currently-loaded
/// model will actually use — the same capability-aware coercion the transcription
/// paths apply (see [`crate::managers::model::effective_language`]). Post-processing
/// resolves it independently so it agrees with the language the transcription ran
/// in, without threading a value through the pipeline.
fn resolve_effective_language(app: &AppHandle, settings: &AppSettings) -> String {
    let tm = app.state::<Arc<TranscriptionManager>>();
    let model_manager = app.state::<Arc<ModelManager>>();
    let active_model = tm
        .get_current_model()
        .unwrap_or_else(|| settings.selected_model.clone());
    match model_manager.get_model_info(&active_model) {
        Some(info) => crate::managers::model::effective_language(
            &settings.selected_language,
            &info.supported_languages,
            info.supports_language_detection,
        ),
        None => settings.selected_language.clone(),
    }
}

pub(crate) async fn process_transcription_output(
    app: &AppHandle,
    transcription: &str,
    mode: TranscribeMode,
) -> ProcessedTranscription {
    let settings = get_settings(app);
    let mut final_text = transcription.to_string();
    let mut post_processed_text: Option<String> = None;
    let mut post_process_prompt: Option<String> = None;

    // Resolve the language the transcription actually ran in (the persisted
    // intent coerced against the loaded model's capabilities) so OpenCC keys off
    // the effective language rather than a possibly-stale intent.
    let effective_language = resolve_effective_language(app, &settings);
    if let Some(converted_text) =
        maybe_convert_chinese_variant(&effective_language, transcription).await
    {
        final_text = converted_text;
    }

    // Modo Traductor (1-a-1): si esta escuchando, detecta idioma y traduce al
    // otro, lo emite al frontend (pantalla grande + voz) y no pega.
    if crate::commands::translator::is_listening() && !final_text.trim().is_empty() {
        let (a, b) = crate::commands::translator::langs();
        if let Some((target, translation)) = converse_translate(app, &final_text, &a, &b).await {
            #[derive(serde::Serialize, Clone)]
            struct TranslatorResult {
                source: String,
                target_lang: String,
                translation: String,
            }
            let _ = app.emit(
                "translator-result",
                TranslatorResult {
                    source: final_text.clone(),
                    target_lang: target,
                    translation,
                },
            );
        }
        return ProcessedTranscription {
            final_text,
            post_processed_text: None,
            post_process_prompt: None,
            interpreter_published: true,
        };
    }

    // Interprete en vivo: si la sala esta escuchando, este dictado va a la sala
    // (traducido por idioma) en vez de pegarse. El guia habla, los oyentes leen.
    if crate::managers::interpreter::global().is_listening() && !final_text.trim().is_empty() {
        let source_lang = crate::managers::interpreter::global().source_lang();
        crate::commands::interpreter::publish_translated(app, final_text.clone(), &source_lang)
            .await;
        return ProcessedTranscription {
            final_text,
            post_processed_text: None,
            post_process_prompt: None,
            interpreter_published: true,
        };
    }

    if mode != TranscribeMode::Standard {
        if let Some(processed_text) =
            post_process_transcription(app, &settings, &final_text, mode).await
        {
            post_processed_text = Some(processed_text.clone());
            final_text = processed_text;

            if let Some(prompt_id) = &settings.post_process_selected_prompt_id {
                if let Some(prompt) = settings
                    .post_process_prompts
                    .iter()
                    .find(|prompt| &prompt.id == prompt_id)
                {
                    post_process_prompt = Some(prompt.prompt.clone());
                }
            }
        }
    } else if final_text != transcription {
        post_processed_text = Some(final_text.clone());
    }

    ProcessedTranscription {
        final_text,
        post_processed_text,
        post_process_prompt,
        interpreter_published: false,
    }
}

impl ShortcutAction for TranscribeAction {
    fn start(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        let start_time = Instant::now();
        debug!("TranscribeAction::start called for binding: {}", binding_id);

        // Load model in the background
        let tm = app.state::<Arc<TranscriptionManager>>();
        let rm = app.state::<Arc<AudioRecordingManager>>();

        // Load ASR model and VAD model in parallel
        let kickoff_started = Instant::now();
        tm.initiate_model_load();
        let rm_clone = Arc::clone(&rm);
        std::thread::spawn(move || {
            if let Err(e) = rm_clone.preload_vad() {
                debug!("VAD pre-load failed: {}", e);
            }
        });
        let kickoff_elapsed = kickoff_started.elapsed();

        let binding_id = binding_id.to_string();
        let tray_started = Instant::now();
        change_tray_icon(app, TrayIconState::Recording);
        let tray_elapsed = tray_started.elapsed();

        // Get the microphone mode to determine audio feedback timing
        let plan_started = Instant::now();
        let settings = get_settings(app);
        let is_always_on = settings.always_on_microphone;

        let selected_model_info = app
            .state::<Arc<ModelManager>>()
            .get_model_info(&settings.selected_model);

        // Use the app-facing model capability as the single pre-recording source
        // for live streaming decisions. Unknown support is represented as false
        // until the model registry is updated by discovery or runtime load.
        let model_supports_streaming = selected_model_info
            .as_ref()
            .map(|m| m.supports_streaming)
            .unwrap_or(false);
        let vad_policy = if !settings.vad_enabled {
            VadPolicy::Disabled
        } else if model_supports_streaming {
            VadPolicy::Streaming
        } else {
            VadPolicy::Offline
        };
        if model_supports_streaming {
            tm.start_stream();
        }
        let plan_elapsed = plan_started.elapsed();

        // Sizing the overlay follows the same advertised capability. A model that
        // doesn't stream (or whose capability is not known yet) gets the compact
        // pill instead of an oversized transparent live window.
        let overlay_started = Instant::now();
        match settings.overlay_style {
            OverlayStyle::Live if model_supports_streaming => utils::show_streaming_overlay(app),
            OverlayStyle::Live | OverlayStyle::Minimal => show_recording_overlay(app),
            OverlayStyle::None => {} // show_overlay_state no-ops on None anyway
        }
        // Everything above runs before capture can begin, so each span here is
        // added keypress->capture latency.
        debug!(
            "start-path pre-recording steps: model_kickoff={:?} tray={:?} settings+stream_plan={:?} overlay={:?}",
            kickoff_elapsed,
            tray_elapsed,
            plan_elapsed,
            overlay_started.elapsed()
        );
        debug!("Microphone mode - always_on: {}", is_always_on);

        let mut recording_error: Option<String> = None;
        if is_always_on {
            // Always-on mode: Play audio feedback immediately, then apply mute after sound finishes
            debug!("Always-on mode: Playing audio feedback immediately");
            let rm_clone = Arc::clone(&rm);
            let app_clone = app.clone();
            // The blocking helper exits immediately if audio feedback is disabled,
            // so we can always reuse this thread to ensure mute happens right after playback.
            std::thread::spawn(move || {
                play_feedback_sound_blocking(&app_clone, SoundType::Start);
                rm_clone.apply_mute();
            });

            if let Err(e) = rm.try_start_recording(&binding_id, vad_policy) {
                debug!("Recording failed: {}", e);
                recording_error = Some(e);
            }
        } else {
            // On-demand mode: Start recording first, then play audio feedback, then apply mute
            // This allows the microphone to be activated before playing the sound
            debug!("On-demand mode: Starting recording first, then audio feedback");
            let recording_start_time = Instant::now();
            match rm.try_start_recording(&binding_id, vad_policy) {
                Ok(()) => {
                    debug!("Recording started in {:?}", recording_start_time.elapsed());
                    // Small delay to ensure microphone stream is active
                    let app_clone = app.clone();
                    let rm_clone = Arc::clone(&rm);
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        debug!("Handling delayed audio feedback/mute sequence");
                        // Helper handles disabled audio feedback by returning early, so we reuse it
                        // to keep mute sequencing consistent in every mode.
                        play_feedback_sound_blocking(&app_clone, SoundType::Start);
                        rm_clone.apply_mute();
                    });
                }
                Err(e) => {
                    debug!("Failed to start recording: {}", e);
                    recording_error = Some(e);
                }
            }
        }

        if recording_error.is_none() {
            // Dynamically register the cancel shortcut in a separate task to avoid deadlock
            shortcut::register_cancel_shortcut(app);
        } else {
            // Starting failed (for example due to blocked microphone permissions).
            // Revert UI state so we don't stay stuck in the recording overlay.
            tm.cancel_stream();
            utils::hide_recording_overlay(app);
            change_tray_icon(app, TrayIconState::Idle);
            if let Some(err) = recording_error {
                let error_type = if is_microphone_access_denied(&err) {
                    "microphone_permission_denied"
                } else if is_no_input_device_error(&err) {
                    "no_input_device"
                } else {
                    "unknown"
                };
                let _ = app.emit(
                    "recording-error",
                    RecordingErrorEvent {
                        error_type: error_type.to_string(),
                        detail: Some(err),
                    },
                );
            }
        }

        debug!(
            "TranscribeAction::start completed in {:?}",
            start_time.elapsed()
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        // Unregister the cancel shortcut when transcription stops
        shortcut::unregister_cancel_shortcut(app);

        let stop_time = Instant::now();
        debug!("TranscribeAction::stop called for binding: {}", binding_id);

        let ah = app.clone();
        let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
        let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());
        let hm = Arc::clone(&app.state::<Arc<HistoryManager>>());

        change_tray_icon(app, TrayIconState::Transcribing);
        // Stop should give immediate visual feedback. Live streaming can keep
        // the larger panel, but it still switches from listening to a working
        // spinner while the stream finalizes. Non-streaming paths use the
        // compact transcribing pill (None no-ops in show_*).
        let style = get_settings(app).overlay_style;
        match (style, tm.is_streaming()) {
            (OverlayStyle::Live, true) => {
                tm.emit_stream_working(StreamWorkKind::Transcribing);
            }
            _ => show_transcribing_overlay(app),
        }

        // Unmute before playing audio feedback so the stop sound is audible
        rm.remove_mute();

        // Play audio feedback for recording stop
        play_feedback_sound(app, SoundType::Stop);

        let binding_id = binding_id.to_string(); // Clone binding_id for the async task
        let mode = self.mode;
        let post_process = mode != TranscribeMode::Standard;
        let cancel_generation = rm.cancel_generation();

        tauri::async_runtime::spawn(async move {
            let _guard = FinishGuard(ah.clone());
            debug!(
                "Starting async transcription task for binding: {}",
                binding_id
            );

            // voice_edit: capturar la seleccion AHORA (usuario ya solto el
            // atajo; el Cmd+C sintetico no interfiere con el listener). La
            // seleccion sigue viva porque el usuario no ha tocado nada.
            if mode == TranscribeMode::Edit {
                let ah_capture = ah.clone();
                let selection =
                    tauri::async_runtime::spawn_blocking(move || capture_selection(&ah_capture))
                        .await
                        .ok()
                        .flatten();
                if let Ok(mut guard) = EDIT_SELECTION.lock() {
                    *guard = selection;
                }
            }

            let stop_recording_time = Instant::now();
            if let Some(samples) = rm.stop_recording(&binding_id, cancel_generation) {
                debug!(
                    "Recording stopped and samples retrieved in {:?}, sample count: {}",
                    stop_recording_time.elapsed(),
                    samples.len()
                );

                if rm.was_cancelled_since(cancel_generation) {
                    debug!("Transcription operation cancelled after recording stop");
                    tm.cancel_stream();
                    utils::hide_recording_overlay(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                    return;
                }

                if samples.is_empty() {
                    debug!("Recording produced no audio samples; skipping persistence");
                    // Tear down any streaming worker so its channel doesn't leak
                    // and block the next start_stream.
                    tm.cancel_stream();
                    utils::hide_recording_overlay(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                } else {
                    // Save WAV concurrently with transcription
                    let sample_count = samples.len();
                    let file_name = format!("handy-{}.wav", chrono::Utc::now().timestamp());
                    let wav_path = hm.recordings_dir().join(&file_name);
                    let wav_path_for_verify = wav_path.clone();
                    let samples_for_wav = samples.clone();
                    let wav_handle = tauri::async_runtime::spawn_blocking(move || {
                        crate::audio_toolkit::save_wav_file(&wav_path, &samples_for_wav)
                    });

                    // Transcribe concurrently with WAV save. If a live stream was
                    // running, finalize it and use its text (all audio was already
                    // fed to the stream); otherwise batch-transcribe the samples.
                    let transcription_time = Instant::now();
                    let transcription_result = match tm.finalize_stream() {
                        // A finalized stream with usable text wins. An empty result
                        // (no active stream, produced nothing, or a finalize error
                        // after the engine was returned) falls back to a full batch
                        // transcription of the same audio. A finalize timeout is
                        // surfaced instead — the worker may still hold the engine,
                        // so a batch fallback would contend with it.
                        Ok(Some(text)) if !text.trim().is_empty() => Ok(text),
                        Ok(_) => tm.transcribe(samples),
                        Err(err) => Err(err),
                    };

                    // Await WAV save and verify
                    let wav_saved = match wav_handle.await {
                        Ok(Ok(())) => {
                            match crate::audio_toolkit::verify_wav_file(
                                &wav_path_for_verify,
                                sample_count,
                            ) {
                                Ok(()) => true,
                                Err(e) => {
                                    error!("WAV verification failed: {}", e);
                                    false
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            error!("Failed to save WAV file: {}", e);
                            false
                        }
                        Err(e) => {
                            error!("WAV save task panicked: {}", e);
                            false
                        }
                    };

                    if rm.was_cancelled_since(cancel_generation) {
                        debug!("Transcription operation cancelled before output handling");
                        utils::hide_recording_overlay(&ah);
                        change_tray_icon(&ah, TrayIconState::Idle);
                        return;
                    }

                    match transcription_result {
                        Ok(transcription) => {
                            debug!(
                                "Transcription completed in {:?}: '{}'",
                                transcription_time.elapsed(),
                                transcription
                            );

                            if post_process {
                                if style == OverlayStyle::Live {
                                    tm.emit_stream_working(StreamWorkKind::Polishing);
                                } else {
                                    show_processing_overlay(&ah);
                                }
                            }
                            let processed =
                                process_transcription_output(&ah, &transcription, mode).await;

                            if processed.interpreter_published {
                                debug!("dictado publicado al interprete; no se pega");
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                                return;
                            }

                            if mode == TranscribeMode::Edit
                                && processed.post_processed_text.is_none()
                            {
                                debug!("voice_edit sin resultado: no se pega nada");
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                                return;
                            }

                            if rm.was_cancelled_since(cancel_generation) {
                                debug!("Transcription operation cancelled before paste");
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                                return;
                            }

                            // Save to history if WAV was saved
                            if wav_saved {
                                if let Err(err) = hm.save_entry(
                                    file_name,
                                    transcription,
                                    post_process,
                                    processed.post_processed_text.clone(),
                                    processed.post_process_prompt.clone(),
                                ) {
                                    error!("Failed to save history entry: {}", err);
                                }
                            }

                            if processed.final_text.is_empty() {
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                            } else {
                                let ah_clone = ah.clone();
                                let paste_time = Instant::now();
                                let final_text = processed.final_text;
                                let rm_for_paste = Arc::clone(&rm);
                                ah.run_on_main_thread(move || {
                                    if rm_for_paste.was_cancelled_since(cancel_generation) {
                                        debug!("Transcription operation cancelled before paste");
                                        utils::hide_recording_overlay(&ah_clone);
                                        change_tray_icon(&ah_clone, TrayIconState::Idle);
                                        return;
                                    }

                                    match utils::paste(final_text, ah_clone.clone()) {
                                        Ok(()) => debug!(
                                            "Text pasted successfully in {:?}",
                                            paste_time.elapsed()
                                        ),
                                        Err(e) => {
                                            error!("Failed to paste transcription: {}", e);
                                            let _ = ah_clone.emit("paste-error", ());
                                        }
                                    }
                                    utils::hide_recording_overlay(&ah_clone);
                                    change_tray_icon(&ah_clone, TrayIconState::Idle);
                                })
                                .unwrap_or_else(|e| {
                                    error!("Failed to run paste on main thread: {:?}", e);
                                    utils::hide_recording_overlay(&ah);
                                    change_tray_icon(&ah, TrayIconState::Idle);
                                });
                            }
                        }
                        Err(err) => {
                            if rm.was_cancelled_since(cancel_generation) {
                                debug!(
                                    "Transcription operation cancelled after transcription error"
                                );
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                                return;
                            }

                            error!("Transcription failed: {}", err);
                            // Surface the failure to the UI (toast). The full
                            // message is also in handy.log via the line above.
                            let _ = ah.emit("transcription-error", err.to_string());
                            // Save entry with empty text so user can retry
                            if wav_saved {
                                if let Err(save_err) = hm.save_entry(
                                    file_name,
                                    String::new(),
                                    post_process,
                                    None,
                                    None,
                                ) {
                                    error!("Failed to save failed history entry: {}", save_err);
                                }
                            }
                            utils::hide_recording_overlay(&ah);
                            change_tray_icon(&ah, TrayIconState::Idle);
                        }
                    }
                }
            } else {
                debug!("No samples retrieved from recording stop");
                // Tear down any streaming worker so its channel doesn't leak.
                tm.cancel_stream();
                utils::hide_recording_overlay(&ah);
                change_tray_icon(&ah, TrayIconState::Idle);
            }
        });

        debug!(
            "TranscribeAction::stop completed in {:?}",
            stop_time.elapsed()
        );
    }
}

// Cancel Action
struct CancelAction;

impl ShortcutAction for CancelAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        utils::cancel_current_operation(app);
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on stop for cancel
    }
}

// Test Action
struct TestAction;

impl ShortcutAction for TestAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Started - {} (App: {})", // Changed "Pressed" to "Started" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Stopped - {} (App: {})", // Changed "Released" to "Stopped" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }
}

// Static Action Map
pub static ACTION_MAP: Lazy<HashMap<String, Arc<dyn ShortcutAction>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "transcribe".to_string(),
        Arc::new(TranscribeAction {
            mode: TranscribeMode::Standard,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_with_post_process".to_string(),
        Arc::new(TranscribeAction {
            mode: TranscribeMode::PostProcess,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_translate".to_string(),
        Arc::new(TranscribeAction {
            mode: TranscribeMode::Translate,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "voice_edit".to_string(),
        Arc::new(TranscribeAction {
            mode: TranscribeMode::Edit,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "cancel".to_string(),
        Arc::new(CancelAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "test".to_string(),
        Arc::new(TestAction) as Arc<dyn ShortcutAction>,
    );
    map
});

#[cfg(test)]
mod tests {
    use super::is_blank_transcription;

    #[test]
    fn blank_transcription_is_detected() {
        assert!(is_blank_transcription(""));
        assert!(is_blank_transcription("   "));
        assert!(is_blank_transcription("\t\n  \r\n"));
    }

    #[test]
    fn non_blank_transcription_is_kept() {
        assert!(!is_blank_transcription("hello"));
        assert!(!is_blank_transcription("  hello  "));
    }
}

/// Rutas posibles del motor local, en orden de preferencia.
enum LocalRoute {
    Sidecar(String),
    Ollama { base_url: String, model: String },
    AppleIntelligence,
    Unavailable(String),
}

async fn resolve_local_route(model: &str) -> LocalRoute {
    let mut reason = String::new();
    if let Some(manager) = crate::managers::local_llm::global() {
        match manager.ensure_running(model).await {
            Ok(base_url) => return LocalRoute::Sidecar(base_url),
            Err(err) => reason = err,
        }
    } else {
        reason = "motor local no inicializado".to_string();
    }

    if let Some(ollama_model) = detect_ollama_model().await {
        return LocalRoute::Ollama {
            base_url: "http://127.0.0.1:11434/v1".to_string(),
            model: ollama_model,
        };
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if apple_intelligence::check_apple_intelligence_availability() {
        return LocalRoute::AppleIntelligence;
    }

    LocalRoute::Unavailable(reason)
}

/// Ollama instalado y con al menos un modelo: GET /v1/models (timeout corto,
/// es localhost). Devuelve el primer modelo disponible.
async fn detect_ollama_model() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(700))
        .build()
        .ok()?;
    let resp = client
        .get("http://127.0.0.1:11434/v1/models")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("data")?
        .as_array()?
        .first()?
        .get("id")?
        .as_str()
        .map(|s| s.to_string())
}

/// Aviso al frontend de que el post-proceso degrado de ruta (toast).
fn notify_fallback(app: &AppHandle, route: &str, detail: &str) {
    #[derive(serde::Serialize, Clone)]
    struct FallbackPayload {
        route: String,
        detail: String,
    }
    let _ = app.emit(
        "local-llm-fallback",
        FallbackPayload {
            route: route.to_string(),
            detail: detail.to_string(),
        },
    );
}

/// Resume un texto largo con el motor LLM local (misma cascada del phraser,
/// temperatura baja). Usado por el Estudio. Devuelve None si no hay ruta local.
pub async fn summarize_text(app: &AppHandle, text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }
    let settings = get_settings(app);
    let mut provider = settings
        .post_process_providers
        .iter()
        .find(|p| p.id == crate::settings::LOCAL_LLM_PROVIDER_ID)
        .cloned()?;
    let model = settings
        .post_process_models
        .get(crate::settings::LOCAL_LLM_PROVIDER_ID)
        .cloned()
        .unwrap_or_default();

    match resolve_local_route(&model).await {
        LocalRoute::Sidecar(base_url) => provider.base_url = base_url,
        LocalRoute::Ollama { base_url, .. } => {
            provider.base_url = base_url;
            provider.supports_structured_output = false;
        }
        _ => return None,
    }

    let system_prompt = "Eres un asistente que resume transcripciones. Entrega un resumen claro y fiel en el mismo idioma del texto: primero 2-3 frases con la idea central, luego los puntos clave en viñetas. No inventes nada que no esté en el texto. Responde solo con el resumen.".to_string();

    match crate::llm_client::send_chat_completion(
        &provider,
        String::new(),
        &model,
        format!("{}\n\nTranscripción:\n{}", system_prompt, text),
        Some("none".to_string()),
        None,
        Some(0.3),
    )
    .await
    {
        Ok(Some(content)) => {
            let cleaned = strip_invisible_chars(&content);
            if cleaned.trim().is_empty() {
                None
            } else {
                Some(cleaned)
            }
        }
        _ => None,
    }
}

/// Traduce un texto al idioma destino (código ISO) con el motor local. Usado
/// por el Intérprete en vivo. Devuelve None si no hay ruta local o falla.
pub async fn translate_text(app: &AppHandle, text: &str, target_lang: &str) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }
    let settings = get_settings(app);
    let mut provider = settings
        .post_process_providers
        .iter()
        .find(|p| p.id == crate::settings::LOCAL_LLM_PROVIDER_ID)
        .cloned()?;
    let model = settings
        .post_process_models
        .get(crate::settings::LOCAL_LLM_PROVIDER_ID)
        .cloned()
        .unwrap_or_default();

    match resolve_local_route(&model).await {
        LocalRoute::Sidecar(base_url) => provider.base_url = base_url,
        LocalRoute::Ollama { base_url, .. } => {
            provider.base_url = base_url;
            provider.supports_structured_output = false;
        }
        _ => return None,
    }

    let prompt = format!(
        "Traduce el siguiente texto al idioma con código ISO '{}'. Responde ÚNICAMENTE con la traducción, sin comillas ni explicaciones. Conserva el tono y la naturalidad de un hablante nativo.\n\nTexto:\n{}",
        target_lang, text
    );

    match crate::llm_client::send_chat_completion(
        &provider,
        String::new(),
        &model,
        prompt,
        Some("none".to_string()),
        None,
        Some(0.2),
    )
    .await
    {
        Ok(Some(content)) => {
            let cleaned = strip_invisible_chars(&content);
            if cleaned.trim().is_empty() {
                None
            } else {
                Some(cleaned.trim().to_string())
            }
        }
        _ => None,
    }
}

/// Modo Traductor: el texto esta en `lang_a` o en `lang_b`; detecta cual y lo
/// traduce al OTRO. Devuelve (idioma_destino_iso, traduccion) usando el motor
/// local. El LLM detecta y traduce en una sola pasada.
pub async fn converse_translate(
    app: &AppHandle,
    text: &str,
    lang_a: &str,
    lang_b: &str,
) -> Option<(String, String)> {
    if text.trim().is_empty() {
        return None;
    }
    let settings = get_settings(app);
    let mut provider = settings
        .post_process_providers
        .iter()
        .find(|p| p.id == crate::settings::LOCAL_LLM_PROVIDER_ID)
        .cloned()?;
    let model = settings
        .post_process_models
        .get(crate::settings::LOCAL_LLM_PROVIDER_ID)
        .cloned()
        .unwrap_or_default();
    match resolve_local_route(&model).await {
        LocalRoute::Sidecar(base_url) => provider.base_url = base_url,
        LocalRoute::Ollama { base_url, .. } => {
            provider.base_url = base_url;
            provider.supports_structured_output = false;
        }
        _ => return None,
    }

    let prompt = format!(
        "El siguiente texto esta en el idioma '{a}' o en el idioma '{b}' (codigos ISO). Detecta en cual de los dos esta y TRADUCELO al OTRO. Responde EXACTAMENTE en este formato, sin nada mas:
<codigo_iso_destino>|<traduccion>

Ejemplo si el destino es ingles: en|Hello there

Texto:
{text}",
        a = lang_a,
        b = lang_b,
        text = text
    );

    let content = match crate::llm_client::send_chat_completion(
        &provider,
        String::new(),
        &model,
        prompt,
        Some("none".to_string()),
        None,
        Some(0.2),
    )
    .await
    {
        Ok(Some(c)) => strip_invisible_chars(&c),
        _ => return None,
    };

    // Parsear "codigo|traduccion". Si el modelo no puso el codigo, inferimos
    // el destino como el idioma distinto al detectado por heuristica simple.
    let trimmed = content.trim();
    if let Some((code, translation)) = trimmed.split_once('|') {
        let code = code.trim().to_lowercase();
        let translation = translation.trim().to_string();
        if !translation.is_empty() && (code == lang_a || code == lang_b) {
            return Some((code, translation));
        }
        if !translation.is_empty() {
            // Codigo raro: asumir el destino como lang_b si no coincide.
            return Some((lang_b.to_string(), translation));
        }
    }
    // Sin formato: devolver el texto tal cual hacia lang_b como ultimo recurso.
    if !trimmed.is_empty() {
        return Some((lang_b.to_string(), trimmed.to_string()));
    }
    None
}
