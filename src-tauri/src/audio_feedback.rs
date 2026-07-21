use crate::settings::SoundTheme;
use crate::settings::{self, AppSettings};
use cpal::traits::{DeviceTrait, HostTrait};
use log::{debug, error, warn};
use rodio::OutputStreamBuilder;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Manager};

pub enum SoundType {
    Start,
    Stop,
}

fn resolve_sound_path(
    app: &AppHandle,
    settings: &AppSettings,
    sound_type: SoundType,
) -> Option<PathBuf> {
    let sound_file = get_sound_path(settings, sound_type);
    let base_dir = get_sound_base_dir(settings);
    match base_dir {
        tauri::path::BaseDirectory::AppData => {
            crate::portable::resolve_app_data(app, &sound_file).ok()
        }
        _ => app.path().resolve(&sound_file, base_dir).ok(),
    }
}

fn get_sound_path(settings: &AppSettings, sound_type: SoundType) -> String {
    match (settings.sound_theme, sound_type) {
        (SoundTheme::Custom, SoundType::Start) => "custom_start.wav".to_string(),
        (SoundTheme::Custom, SoundType::Stop) => "custom_stop.wav".to_string(),
        (_, SoundType::Start) => settings.sound_theme.to_start_path(),
        (_, SoundType::Stop) => settings.sound_theme.to_stop_path(),
    }
}

fn get_sound_base_dir(settings: &AppSettings) -> tauri::path::BaseDirectory {
    match settings.sound_theme {
        SoundTheme::Custom => tauri::path::BaseDirectory::AppData,
        _ => tauri::path::BaseDirectory::Resource,
    }
}

pub fn play_feedback_sound(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);
    if !settings.audio_feedback {
        return;
    }
    if let Some(path) = resolve_sound_path(app, &settings, sound_type) {
        play_sound_async(app, path);
    }
}

pub fn play_feedback_sound_blocking(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);
    if !settings.audio_feedback {
        return;
    }
    if let Some(path) = resolve_sound_path(app, &settings, sound_type) {
        play_sound_blocking(app, &path);
    }
}

pub fn play_test_sound(app: &AppHandle, sound_type: SoundType) {
    let settings = settings::get_settings(app);
    if let Some(path) = resolve_sound_path(app, &settings, sound_type) {
        play_sound_blocking(app, &path);
    }
}

fn play_sound_async(app: &AppHandle, path: PathBuf) {
    let app_handle = app.clone();
    thread::spawn(move || {
        if let Err(e) = play_sound_at_path(&app_handle, path.as_path()) {
            error!("Failed to play sound '{}': {}", path.display(), e);
        }
    });
}

fn play_sound_blocking(app: &AppHandle, path: &Path) {
    if let Err(e) = play_sound_at_path(app, path) {
        error!("Failed to play sound '{}': {}", path.display(), e);
    }
}

fn play_sound_at_path(app: &AppHandle, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let settings = settings::get_settings(app);
    let volume = settings.audio_feedback_volume;
    let selected_device = settings.selected_output_device.clone();
    play_audio_file(path, selected_device, volume)
}

/// Sink de la voz del Intérprete en curso, para poder cortarla (auditoría
/// #18: Pausar/Descartar deben frenar lo que ya está sonando hacia la llamada).
static INTERPRETER_SINK: Mutex<Option<Arc<rodio::Sink>>> = Mutex::new(None);

/// Corta la reproducción de la voz del Intérprete si hay una en curso.
pub fn stop_interpreter_playback() {
    if let Ok(mut guard) = INTERPRETER_SINK.lock() {
        if let Some(sink) = guard.take() {
            sink.stop();
        }
    }
}

/// Reproduce un WAV en el dispositivo elegido registrando el sink, para que
/// `stop_interpreter_playback()` pueda cortarlo a mitad. Bloqueante hasta que
/// termina o lo cortan. Lo usa la voz del Intérprete de reuniones.
pub fn play_interpreter_voice(
    path: &std::path::Path,
    selected_device: Option<String>,
    volume: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream_builder = open_stream_builder(selected_device)?;
    let stream_handle = stream_builder.open_stream()?;
    let mixer = stream_handle.mixer();
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);
    let sink = Arc::new(rodio::play(mixer, buf_reader)?);
    sink.set_volume(volume);
    // Corta cualquier locución anterior y registra esta (una a la vez).
    stop_interpreter_playback();
    if let Ok(mut guard) = INTERPRETER_SINK.lock() {
        *guard = Some(Arc::clone(&sink));
    }
    sink.sleep_until_end();
    if let Ok(mut guard) = INTERPRETER_SINK.lock() {
        // Solo lo limpiamos si sigue siendo el nuestro (no pisar una locución
        // más nueva que ya lo reemplazó).
        if guard
            .as_ref()
            .map(|s| Arc::ptr_eq(s, &sink))
            .unwrap_or(false)
        {
            *guard = None;
        }
    }
    Ok(())
}

/// Construye el `OutputStreamBuilder` para el dispositivo pedido (por nombre;
/// "Default"/None = salida por defecto). Compartido por los reproductores.
fn open_stream_builder(
    selected_device: Option<String>,
) -> Result<rodio::OutputStreamBuilder, Box<dyn std::error::Error>> {
    match selected_device {
        Some(device_name) if device_name != "Default" => {
            let host = crate::audio_toolkit::get_cpal_host();
            let devices = host.output_devices()?;
            for device in devices {
                if device.name()? == device_name {
                    return Ok(OutputStreamBuilder::from_device(device)?);
                }
            }
            warn!("Device '{}' not found, using default device", device_name);
            Ok(OutputStreamBuilder::from_default_device()?)
        }
        _ => Ok(OutputStreamBuilder::from_default_device()?),
    }
}

/// Público: también lo usa la voz del Intérprete de reuniones para sacar la
/// traducción por un dispositivo concreto (p. ej. un micrófono virtual).
pub fn play_audio_file(
    path: &std::path::Path,
    selected_device: Option<String>,
    volume: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream_builder = if let Some(device_name) = selected_device {
        if device_name == "Default" {
            debug!("Using default device");
            OutputStreamBuilder::from_default_device()?
        } else {
            let host = crate::audio_toolkit::get_cpal_host();
            let devices = host.output_devices()?;

            let mut found_device = None;
            for device in devices {
                if device.name()? == device_name {
                    found_device = Some(device);
                    break;
                }
            }

            match found_device {
                Some(device) => OutputStreamBuilder::from_device(device)?,
                None => {
                    warn!("Device '{}' not found, using default device", device_name);
                    OutputStreamBuilder::from_default_device()?
                }
            }
        }
    } else {
        debug!("Using default device");
        OutputStreamBuilder::from_default_device()?
    };

    let stream_handle = stream_builder.open_stream()?;
    let mixer = stream_handle.mixer();

    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);

    let sink = rodio::play(mixer, buf_reader)?;
    sink.set_volume(volume);
    sink.sleep_until_end();

    Ok(())
}
