//! Pausar/reanudar la reproducción de medios mientras dictas. En macOS usa
//! AppleScript sobre Música y Spotify (los reproductores más comunes): pausa
//! solo los que estaban sonando y los reanuda al terminar. Cubre el caso de uso
//! real (música de fondo) sin tocar nada que no estuviera reproduciendo.

use std::sync::{Mutex, OnceLock};

#[cfg(target_os = "macos")]
const PLAYERS: [&str; 2] = ["Spotify", "Music"];

/// Reproductores que ESTA app pausó, para reanudarlos exactamente esos.
static PAUSED: OnceLock<Mutex<Vec<&'static str>>> = OnceLock::new();

fn paused() -> &'static Mutex<Vec<&'static str>> {
    PAUSED.get_or_init(|| Mutex::new(Vec::new()))
}

/// Pausa los reproductores que estén sonando y recuerda cuáles.
pub fn pause_media() {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let mut mine = paused().lock().unwrap();
        mine.clear();
        for app in PLAYERS {
            let script = format!(
                "if application \"{a}\" is running then\n\
                     tell application \"{a}\"\n\
                         if player state is playing then\n\
                             pause\n\
                             return \"paused\"\n\
                         end if\n\
                     end tell\n\
                 end if\n\
                 return \"\"",
                a = app
            );
            if let Ok(out) = Command::new("osascript").args(["-e", &script]).output() {
                if String::from_utf8_lossy(&out.stdout).trim() == "paused" {
                    mine.push(app);
                }
            }
        }
    }
}

/// Reanuda solo los reproductores que esta app había pausado.
pub fn resume_media() {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let apps: Vec<&'static str> = paused().lock().unwrap().drain(..).collect();
        for app in apps {
            let script = format!("tell application \"{}\" to play", app);
            let _ = Command::new("osascript").args(["-e", &script]).output();
        }
    }
}
