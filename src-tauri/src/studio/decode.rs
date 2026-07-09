//! Decode de archivos de audio/video a f32 mono 16kHz para el Estudio.
//! symphonia (pure Rust): mp3, m4a/aac/alac, mp4, flac, ogg/vorbis, wav,
//! aiff, caf. Sin ffmpeg. El resampleo reusa FrameResampler (rubato).

use crate::audio_toolkit::audio::FrameResampler;
use log::debug;
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub const STUDIO_SAMPLE_RATE: usize = 16_000;

pub fn supported_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .as_deref(),
        Some(
            "mp3" | "m4a" | "mp4" | "mov" | "aac" | "flac" | "ogg" | "wav" | "aiff" | "aif" | "caf"
        )
    )
}

/// Decodifica cualquier formato soportado a f32 mono 16kHz.
/// El archivo se procesa por paquetes (no se carga entero en RAM).
pub fn decode_to_16k_mono(path: &Path) -> Result<Vec<f32>, String> {
    let file = File::open(path).map_err(|e| format!("No se pudo abrir el archivo: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Formato no reconocido: {}", e))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("El archivo no tiene pista de audio decodificable")?;
    let track_id = track.id;
    let src_rate = track
        .codec_params
        .sample_rate
        .ok_or("Pista sin sample rate")? as usize;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Codec no soportado: {}", e))?;

    let mut resampler =
        FrameResampler::new(src_rate, STUDIO_SAMPLE_RATE, Duration::from_millis(100));
    let mut out: Vec<f32> = Vec::new();
    let mut mono_buf: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(symphonia::core::errors::Error::ResetRequired) => break,
            Err(e) => return Err(format!("Error leyendo el archivo: {}", e)),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            // Paquete corrupto aislado: se salta, no se aborta todo el archivo.
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Error decodificando: {}", e)),
        };

        downmix_to_mono(&decoded, &mut mono_buf);
        resampler.push(&mono_buf, |frame| out.extend_from_slice(frame));
    }

    // Vaciar la cola interna del resampler con ~100ms de silencio.
    let flush = vec![0.0f32; src_rate / 10];
    resampler.push(&flush, |frame| out.extend_from_slice(frame));

    debug!(
        "Studio decode: {:.1}s de audio a {}Hz",
        out.len() as f64 / STUDIO_SAMPLE_RATE as f64,
        STUDIO_SAMPLE_RATE
    );
    if out.is_empty() {
        return Err("El archivo no produjo audio (¿pista vacía?)".to_string());
    }
    Ok(out)
}

/// Promedia todos los canales a mono f32, cubriendo los layouts de sample
/// que entregan los codecs de symphonia.
fn downmix_to_mono(decoded: &AudioBufferRef, mono: &mut Vec<f32>) {
    macro_rules! mix {
        ($buf:expr, $to_f32:expr) => {{
            let channels = $buf.spec().channels.count();
            let frames = $buf.frames();
            mono.clear();
            mono.resize(frames, 0.0);
            for ch in 0..channels {
                let plane = $buf.chan(ch);
                for (i, s) in plane.iter().enumerate() {
                    mono[i] += $to_f32(*s);
                }
            }
            let inv = 1.0 / channels.max(1) as f32;
            for s in mono.iter_mut() {
                *s *= inv;
            }
        }};
    }

    match decoded {
        AudioBufferRef::F32(buf) => mix!(buf, |s: f32| s),
        AudioBufferRef::F64(buf) => mix!(buf, |s: f64| s as f32),
        AudioBufferRef::S32(buf) => mix!(buf, |s: i32| s as f32 / i32::MAX as f32),
        AudioBufferRef::S16(buf) => mix!(buf, |s: i16| s as f32 / i16::MAX as f32),
        AudioBufferRef::U8(buf) => mix!(buf, |s: u8| (s as f32 - 128.0) / 128.0),
        AudioBufferRef::S24(buf) => mix!(buf, |s: symphonia::core::sample::i24| {
            s.inner() as f32 / 8_388_607.0
        }),
        AudioBufferRef::U16(buf) => mix!(buf, |s: u16| (s as f32 - 32768.0) / 32768.0),
        AudioBufferRef::U24(buf) => mix!(buf, |s: symphonia::core::sample::u24| {
            (s.inner() as f32 - 8_388_608.0) / 8_388_608.0
        }),
        AudioBufferRef::U32(buf) => mix!(buf, |s: u32| (s as f64 / u32::MAX as f64 * 2.0 - 1.0)
            as f32),
        AudioBufferRef::S8(buf) => mix!(buf, |s: i8| s as f32 / i8::MAX as f32),
    }
}
