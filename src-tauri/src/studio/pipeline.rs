//! Orquestación del Estudio: audio largo → chunks con overlap →
//! transcripción con segmentos → reensamblado deduplicado.

use super::segments::{merge_deduplicated, offset_segments, Segment};
use crate::managers::transcription::TranscriptionManager;

const SAMPLE_RATE: usize = 16_000;
/// Ventana por chunk (~8 min): por progreso y RAM, no por el modelo.
const CHUNK_SECONDS: usize = 480;
const OVERLAP_SECONDS: usize = 5;

/// Transcribe samples 16kHz mono de cualquier duración, en chunks si hace
/// falta. `on_progress` recibe 0.0..1.0 al cierre de cada chunk.
pub fn transcribe_samples(
    tm: &TranscriptionManager,
    samples: &[f32],
    mut on_progress: impl FnMut(f32),
) -> Result<Vec<Segment>, String> {
    // La ventana depende del motor: Whisper digiere chunks largos; los ONNX
    // (Canary, Parakeet...) necesitan frases cortas o devuelven vacío.
    let chunk_len = tm.max_window_seconds().min(CHUNK_SECONDS) * SAMPLE_RATE;
    let overlap_len = OVERLAP_SECONDS * SAMPLE_RATE;

    if samples.len() <= chunk_len + overlap_len {
        let segments = tm
            .transcribe_segments(samples.to_vec())
            .map_err(|e| e.to_string())?;
        on_progress(1.0);
        return Ok(segments);
    }

    let total_chunks = samples.len().div_ceil(chunk_len);
    let mut chunks: Vec<Vec<Segment>> = Vec::with_capacity(total_chunks);
    for index in 0..total_chunks {
        let start = index * chunk_len;
        let end = (start + chunk_len + overlap_len).min(samples.len());
        let mut segments = tm
            .transcribe_segments(samples[start..end].to_vec())
            .map_err(|e| e.to_string())?;
        offset_segments(&mut segments, start as f64 / SAMPLE_RATE as f64);
        chunks.push(segments);
        on_progress((index + 1) as f32 / total_chunks as f32);
    }
    Ok(merge_deduplicated(chunks))
}
