//! Supresión de ruido de fondo (RNNoise vía `nnnoiseless`). RNNoise opera a
//! 48 kHz en tramas de 480 muestras, así que el buffer de 16 kHz se sube a 48
//! kHz, se limpia, y se baja de nuevo a 16 kHz. Se aplica en lote sobre la
//! grabación completa antes de transcribir/guardar.

use nnnoiseless::DenoiseState;
use rubato::{FftFixedIn, Resampler};

const CHUNK: usize = 1024;
/// Tamaño de trama de RNNoise a 48 kHz.
const FRAME: usize = 480;

/// Resamplea un buffer mono completo con rubato (misma técnica que el pipeline
/// de captura). Si algo falla, devuelve la entrada sin tocar.
fn resample(input: &[f32], from: usize, to: usize) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let mut rs = match FftFixedIn::<f32>::new(from, to, CHUNK, 1, 1) {
        Ok(r) => r,
        Err(_) => return input.to_vec(),
    };
    let mut out: Vec<f32> = Vec::with_capacity(input.len() * to / from + CHUNK);
    let mut buf: Vec<f32> = Vec::with_capacity(CHUNK);
    let mut src = input;
    while !src.is_empty() {
        let take = (CHUNK - buf.len()).min(src.len());
        buf.extend_from_slice(&src[..take]);
        src = &src[take..];
        if buf.len() == CHUNK {
            if let Ok(o) = rs.process(&[&buf[..]], None) {
                out.extend_from_slice(&o[0]);
            }
            buf.clear();
        }
    }
    if !buf.is_empty() {
        buf.resize(CHUNK, 0.0);
        if let Ok(o) = rs.process(&[&buf[..]], None) {
            out.extend_from_slice(&o[0]);
        }
    }
    out
}

/// Limpia el ruido de un buffer mono a 16 kHz y lo devuelve a 16 kHz.
pub fn denoise_16k_mono(input: &[f32]) -> Vec<f32> {
    // Tramas incompletas no valen la pena; devuélvelas tal cual.
    if input.len() < FRAME {
        return input.to_vec();
    }

    // 16 kHz -> 48 kHz
    let up = resample(input, 16_000, 48_000);

    // RNNoise trabaja en escala i16 (-32768..32767), no en -1.0..1.0.
    let mut denoised = vec![0.0f32; up.len()];
    let mut state = DenoiseState::new();
    let mut frame_in = [0.0f32; FRAME];
    let mut frame_out = [0.0f32; FRAME];
    let mut i = 0;
    while i + FRAME <= up.len() {
        for j in 0..FRAME {
            frame_in[j] = up[i + j] * 32768.0;
        }
        state.process_frame(&mut frame_out, &frame_in);
        for j in 0..FRAME {
            denoised[i + j] = frame_out[j] / 32768.0;
        }
        i += FRAME;
    }
    // Cola menor a una trama: sin procesar.
    if i < up.len() {
        denoised[i..].copy_from_slice(&up[i..]);
    }

    // 48 kHz -> 16 kHz
    resample(&denoised, 48_000, 16_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denoise_preserves_length_and_does_not_panic() {
        // 1 segundo de ruido pseudo-aleatorio a 16 kHz.
        let n = 16_000usize;
        let mut input = vec![0.0f32; n];
        let mut seed = 12345u32;
        for x in input.iter_mut() {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            *x = (seed as f32 / u32::MAX as f32) * 0.2 - 0.1;
        }
        let out = denoise_16k_mono(&input);
        // El ida y vuelta 16k->48k->16k conserva la longitud aproximada.
        let diff = (out.len() as i64 - n as i64).abs();
        assert!(diff < 2000, "longitud muy distinta: {} vs {}", out.len(), n);
    }
}
