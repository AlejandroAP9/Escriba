use crate::managers::history::{HistoryEntry, HistoryManager};
use crate::managers::model::ModelManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings;
use crate::tray_i18n::get_tray_translations;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Manager, Theme};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Clone, Debug, PartialEq)]
pub enum TrayIconState {
    Idle,
    Recording,
    Transcribing,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AppTheme {
    Dark,
    Light,
    Colored, // Pink/colored theme for Linux
}

/// Gets the current app theme, with Linux defaulting to Colored theme
pub fn get_current_theme(app: &AppHandle) -> AppTheme {
    if cfg!(target_os = "linux") {
        // On Linux, always use the colored theme
        return AppTheme::Colored;
    }

    // On Windows, the taskbar's own light/dark mode is independent from the
    // app's theme: Settings > Personalization > Colors lets users pick
    // separate "app mode" / "system mode" values (and/or an accent color for
    // the taskbar). `window.theme()` only reflects app mode, so a dark
    // taskbar paired with a light app mode (or vice versa) picks the
    // wrong-colored tray icon, making it blend into the taskbar. Read the
    // taskbar's own setting instead, falling back to window.theme() below if
    // the registry read fails.
    #[cfg(target_os = "windows")]
    if let Some(is_light) = windows_taskbar_uses_light_theme() {
        return if is_light {
            AppTheme::Light
        } else {
            AppTheme::Dark
        };
    }

    // macOS (and Windows registry-read fallback): map window theme to our app theme
    if let Some(main_window) = app.get_webview_window("main") {
        match main_window.theme().unwrap_or(Theme::Dark) {
            Theme::Light => AppTheme::Light,
            Theme::Dark => AppTheme::Dark,
            _ => AppTheme::Dark, // Default fallback
        }
    } else {
        AppTheme::Dark
    }
}

/// Reads whether Windows' taskbar/Start menu ("system") is set to light mode,
/// via the same registry value Explorer itself uses. Returns `None` if the
/// key is missing or unreadable (e.g. unexpected Windows version), so the
/// caller can fall back to `window.theme()`.
#[cfg(target_os = "windows")]
fn windows_taskbar_uses_light_theme() -> Option<bool> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let personalize = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .ok()?;
    let uses_light = personalize
        .get_value::<u32, _>("SystemUsesLightTheme")
        .ok()?;
    Some(uses_light != 0)
}

/// Watches the taskbar's own light/dark setting in the background and
/// refreshes the tray icon whenever it changes on its own (i.e. the user
/// only touched "system mode" / the taskbar color, without changing the
/// app's theme, so `WindowEvent::ThemeChanged` never fires). Runs for the
/// lifetime of the process, same as the idle-unload watcher in
/// `managers/local_llm.rs` — no explicit shutdown, it just dies with the app.
///
/// A background poll (rather than hooking the raw `WM_SETTINGCHANGE`/
/// "ImmersiveColorSet" window message) is a deliberate simplicity trade-off:
/// it avoids subclassing the window's WndProc with unsafe FFI, at the cost of
/// the icon taking up to a couple seconds to catch up instead of updating
/// instantly.
#[cfg(target_os = "windows")]
pub fn start_taskbar_theme_watcher(app: &AppHandle) {
    use std::time::Duration;

    let app_handle = app.clone();
    let mut last_is_light = windows_taskbar_uses_light_theme();

    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(2));

        let current_is_light = windows_taskbar_uses_light_theme();
        if current_is_light != last_is_light {
            last_is_light = current_is_light;
            change_tray_icon(&app_handle, TrayIconState::Idle);
        }
    });
}

/// Gets the appropriate icon path for the given theme and state
pub fn get_icon_path(theme: AppTheme, state: TrayIconState) -> &'static str {
    match (theme, state) {
        // Dark theme uses light icons
        (AppTheme::Dark, TrayIconState::Idle) => "resources/tray_idle.png",
        (AppTheme::Dark, TrayIconState::Recording) => "resources/tray_recording.png",
        (AppTheme::Dark, TrayIconState::Transcribing) => "resources/tray_transcribing.png",
        // Light theme uses dark icons
        (AppTheme::Light, TrayIconState::Idle) => "resources/tray_idle_dark.png",
        (AppTheme::Light, TrayIconState::Recording) => "resources/tray_recording_dark.png",
        (AppTheme::Light, TrayIconState::Transcribing) => "resources/tray_transcribing_dark.png",
        // Colored theme uses pink icons (for Linux)
        (AppTheme::Colored, TrayIconState::Idle) => "resources/handy.png",
        (AppTheme::Colored, TrayIconState::Recording) => "resources/recording.png",
        (AppTheme::Colored, TrayIconState::Transcribing) => "resources/transcribing.png",
    }
}

pub fn change_tray_icon(app: &AppHandle, icon: TrayIconState) {
    let tray = app.state::<TrayIcon>();
    let theme = get_current_theme(app);

    let icon_path = get_icon_path(theme, icon.clone());

    let icon_started = std::time::Instant::now();
    let _ = tray.set_icon(Some(
        Image::from_path(
            app.path()
                .resolve(icon_path, tauri::path::BaseDirectory::Resource)
                .expect("failed to resolve"),
        )
        .expect("failed to set icon"),
    ));
    let icon_elapsed = icon_started.elapsed();

    // Update menu based on state
    let menu_started = std::time::Instant::now();
    update_tray_menu(app, &icon, None);
    debug!(
        "tray icon change ({:?}): set_icon={:?} menu={:?}",
        icon,
        icon_elapsed,
        menu_started.elapsed()
    );
}

pub fn tray_tooltip() -> String {
    version_label()
}

fn version_label() -> String {
    if cfg!(debug_assertions) {
        format!("Escriba v{} (Dev)", env!("CARGO_PKG_VERSION"))
    } else {
        format!("Escriba v{}", env!("CARGO_PKG_VERSION"))
    }
}

pub fn update_tray_menu(app: &AppHandle, state: &TrayIconState, locale: Option<&str>) {
    let settings = settings::get_settings(app);

    let locale = locale.unwrap_or(&settings.app_language);
    let strings = get_tray_translations(Some(locale.to_string()));

    // Platform-specific accelerators
    #[cfg(target_os = "macos")]
    let (settings_accelerator, quit_accelerator) = (Some("Cmd+,"), Some("Cmd+Q"));
    #[cfg(not(target_os = "macos"))]
    let (settings_accelerator, quit_accelerator) = (Some("Ctrl+,"), Some("Ctrl+Q"));

    // Create common menu items
    let version_label = version_label();
    let version_i = MenuItem::with_id(app, "version", &version_label, false, None::<&str>)
        .expect("failed to create version item");
    let settings_i = MenuItem::with_id(
        app,
        "settings",
        &strings.settings,
        true,
        settings_accelerator,
    )
    .expect("failed to create settings item");
    let check_updates_i = MenuItem::with_id(
        app,
        "check_updates",
        &strings.check_updates,
        settings.update_checks_enabled,
        None::<&str>,
    )
    .expect("failed to create check updates item");
    let copy_last_transcript_i = MenuItem::with_id(
        app,
        "copy_last_transcript",
        &strings.copy_last_transcript,
        true,
        None::<&str>,
    )
    .expect("failed to create copy last transcript item");
    let model_loaded = app.state::<Arc<TranscriptionManager>>().is_model_loaded();
    let quit_i = MenuItem::with_id(app, "quit", &strings.quit, true, quit_accelerator)
        .expect("failed to create quit item");
    let open_app_i = MenuItem::with_id(app, "open_app", &strings.open_app, true, None::<&str>)
        .expect("failed to create open app item");
    let dictate_now_i =
        MenuItem::with_id(app, "dictate_now", &strings.dictate_now, true, None::<&str>)
            .expect("failed to create dictate now item");
    // El label del Dictado libre refleja su estado real (activar/desactivar).
    let free_dictation_label = if crate::commands::free_dictation::is_active() {
        &strings.free_dictation_off
    } else {
        &strings.free_dictation_on
    };
    let free_dictation_i = MenuItem::with_id(
        app,
        "free_dictation",
        free_dictation_label,
        true,
        None::<&str>,
    )
    .expect("failed to create free dictation item");
    let quick_listen_i = MenuItem::with_id(
        app,
        "quick_listen",
        &strings.quick_listen,
        true,
        None::<&str>,
    )
    .expect("failed to create quick listen item");
    let open_history_i = MenuItem::with_id(
        app,
        "open_history",
        &strings.open_history,
        true,
        None::<&str>,
    )
    .expect("failed to create open history item");
    let separator = || PredefinedMenuItem::separator(app).expect("failed to create separator");

    // Build model submenu — label is the active model name
    let model_manager = app.state::<Arc<ModelManager>>();
    let models = model_manager.get_available_models();
    let current_model_id = &settings.selected_model;

    let mut downloaded: Vec<_> = models.into_iter().filter(|m| m.is_downloaded).collect();
    downloaded.sort_by(|a, b| a.name.cmp(&b.name));

    let submenu_label = downloaded
        .iter()
        .find(|m| m.id == *current_model_id)
        .map(|m| m.name.clone())
        .unwrap_or_else(|| strings.model.clone());

    let model_submenu = {
        let submenu = Submenu::with_id(app, "model_submenu", &submenu_label, true)
            .expect("failed to create model submenu");

        for model in &downloaded {
            let is_active = model.id == *current_model_id;
            let item_id = format!("model_select:{}", model.id);
            let item =
                CheckMenuItem::with_id(app, &item_id, &model.name, true, is_active, None::<&str>)
                    .expect("failed to create model item");
            let _ = submenu.append(&item);
        }

        submenu
    };

    let unload_model_i = MenuItem::with_id(
        app,
        "unload_model",
        &strings.unload_model,
        model_loaded,
        None::<&str>,
    )
    .expect("failed to create unload model item");

    let menu = match state {
        TrayIconState::Recording | TrayIconState::Transcribing => {
            let cancel_i = MenuItem::with_id(app, "cancel", &strings.cancel, true, None::<&str>)
                .expect("failed to create cancel item");
            Menu::with_items(
                app,
                &[
                    &version_i,
                    &separator(),
                    &open_app_i,
                    &free_dictation_i,
                    &cancel_i,
                    &separator(),
                    &copy_last_transcript_i,
                    &separator(),
                    &settings_i,
                    &check_updates_i,
                    &separator(),
                    &quit_i,
                ],
            )
            .expect("failed to create menu")
        }
        TrayIconState::Idle => Menu::with_items(
            app,
            &[
                &version_i,
                &separator(),
                &open_app_i,
                &dictate_now_i,
                &free_dictation_i,
                &quick_listen_i,
                &open_history_i,
                &separator(),
                &copy_last_transcript_i,
                &separator(),
                &model_submenu,
                &unload_model_i,
                &separator(),
                &settings_i,
                &check_updates_i,
                &separator(),
                &quit_i,
            ],
        )
        .expect("failed to create menu"),
    };

    let tray = app.state::<TrayIcon>();
    let _ = tray.set_menu(Some(menu));
    let _ = tray.set_icon_as_template(true);
    let _ = tray.set_tooltip(Some(version_label));
}

fn last_transcript_text(entry: &HistoryEntry) -> &str {
    entry
        .post_processed_text
        .as_deref()
        .unwrap_or(&entry.transcription_text)
}

pub fn set_tray_visibility(app: &AppHandle, visible: bool) {
    let tray = app.state::<TrayIcon>();
    if let Err(e) = tray.set_visible(visible) {
        error!("Failed to set tray visibility: {}", e);
    } else {
        info!("Tray visibility set to: {}", visible);
    }
}

pub fn copy_last_transcript(app: &AppHandle) {
    let history_manager = app.state::<Arc<HistoryManager>>();
    let entry = match history_manager.get_latest_completed_entry() {
        Ok(Some(entry)) => entry,
        Ok(None) => {
            warn!("No completed transcription history entries available for tray copy.");
            return;
        }
        Err(err) => {
            error!(
                "Failed to fetch last completed transcription entry: {}",
                err
            );
            return;
        }
    };

    let text = last_transcript_text(&entry);
    if text.trim().is_empty() {
        warn!("Last completed transcription is empty; skipping tray copy.");
        return;
    }

    if let Err(err) = app.clipboard().write_text(text) {
        error!("Failed to copy last transcript to clipboard: {}", err);
        return;
    }

    info!("Copied last transcript to clipboard via tray.");
}

#[cfg(test)]
mod tests {
    use super::last_transcript_text;
    use crate::managers::history::HistoryEntry;

    fn build_entry(transcription: &str, post_processed: Option<&str>) -> HistoryEntry {
        HistoryEntry {
            id: 1,
            file_name: "handy-1.wav".to_string(),
            timestamp: 0,
            saved: false,
            title: "Recording".to_string(),
            transcription_text: transcription.to_string(),
            post_processed_text: post_processed.map(|text| text.to_string()),
            post_process_prompt: None,
            post_process_requested: false,
        }
    }

    #[test]
    fn uses_post_processed_text_when_available() {
        let entry = build_entry("raw", Some("processed"));
        assert_eq!(last_transcript_text(&entry), "processed");
    }

    #[test]
    fn falls_back_to_raw_transcription() {
        let entry = build_entry("raw", None);
        assert_eq!(last_transcript_text(&entry), "raw");
    }
}
