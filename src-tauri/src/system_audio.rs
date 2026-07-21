//! Puente FFI al bridge Swift de ScreenCaptureKit (swift/system_audio.swift):
//! captura lo que suena en el computador (Zoom, Meet, un video) a 16 kHz mono
//! para que Sesiones también escuche "al otro lado" de una reunión.
//! Solo macOS 13+ (Apple Silicon); en el resto todo degrada a "no soportado".

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
mod ffi {
    extern "C" {
        pub fn escriba_sysaudio_supported() -> i32;
        pub fn escriba_sysaudio_permission_granted() -> i32;
        pub fn escriba_sysaudio_request_permission() -> i32;
        pub fn escriba_sysaudio_start() -> i32;
        pub fn escriba_sysaudio_stop();
        pub fn escriba_sysaudio_read(out: *mut f32, max: i32) -> i32;
        pub fn escriba_sysaudio_alive() -> i32;
    }
}

/// ¿La captura sigue viva? `false` si el stream se cayó solo (permiso
/// revocado, reinicio de coreaudiod) o no está corriendo (auditoría #10).
pub fn alive() -> bool {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        unsafe { ffi::escriba_sysaudio_alive() == 1 }
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        false
    }
}

/// ¿Este equipo puede capturar el audio del sistema?
pub fn supported() -> bool {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        unsafe { ffi::escriba_sysaudio_supported() == 1 }
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        false
    }
}

/// ¿Está concedido el permiso de Grabación de pantalla?
pub fn permission_granted() -> bool {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        unsafe { ffi::escriba_sysaudio_permission_granted() == 1 }
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        false
    }
}

/// Muestra el diálogo del sistema para pedir el permiso (solo la primera vez;
/// después macOS exige activarlo a mano en Ajustes del Sistema).
pub fn request_permission() -> bool {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        unsafe { ffi::escriba_sysaudio_request_permission() == 1 }
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        false
    }
}

/// Arranca la captura. Errores como claves estables para que la UI traduzca.
pub fn start() -> Result<(), String> {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        match unsafe { ffi::escriba_sysaudio_start() } {
            0 => Ok(()),
            1 => Err("unsupported".to_string()),
            2 => Err("screen_permission".to_string()),
            _ => Err("start_failed".to_string()),
        }
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        Err("unsupported".to_string())
    }
}

/// Detiene la captura y descarta lo no leído.
pub fn stop() {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    unsafe {
        ffi::escriba_sysaudio_stop()
    }
}

/// Drena las muestras capturadas (Float32, 16 kHz mono) desde el ring buffer
/// del bridge. Devuelve cuántas escribió en `buf`.
pub fn read(buf: &mut [f32]) -> usize {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        let n = unsafe { ffi::escriba_sysaudio_read(buf.as_mut_ptr(), buf.len() as i32) };
        n.max(0) as usize
    }
    #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
    {
        let _ = buf;
        0
    }
}
