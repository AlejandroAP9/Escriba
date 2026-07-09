//! Lógica pura de segmentos del Estudio: offset por chunk, deduplicación de
//! solapes al reensamblar (algoritmo adaptado de MediaTranscribe, MIT),
//! agrupado en párrafos y heurística de calidad.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Tolerancia al deduplicar solapes entre chunks (segundos).
const DEDUP_TOLERANCE_S: f64 = 0.5;
/// Un gap de silencio mayor a esto abre párrafo nuevo.
const PARAGRAPH_GAP_S: f64 = 3.0;
/// Duración acumulada que fuerza párrafo nuevo aunque no haya gap.
const PARAGRAPH_MAX_SPAN_S: f64 = 45.0;
/// Bajo este ritmo (chars/min) la transcripción se marca sospechosa.
const MIN_CHARS_PER_MINUTE: f64 = 100.0;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
pub struct Segment {
    pub start_s: f64,
    pub end_s: f64,
    pub text: String,
}

/// Desplaza los timestamps de un chunk a la línea de tiempo global.
pub fn offset_segments(segments: &mut [Segment], offset_s: f64) {
    for seg in segments.iter_mut() {
        seg.start_s += offset_s;
        seg.end_s += offset_s;
    }
}

/// Reensambla chunks solapados descartando los segmentos ya cubiertos:
/// un segmento entra solo si empieza después del máximo `end` visto menos
/// la tolerancia (así el overlap de 5s no duplica frases en la juntura).
pub fn merge_deduplicated(chunks: Vec<Vec<Segment>>) -> Vec<Segment> {
    let mut merged: Vec<Segment> = Vec::new();
    let mut max_end_so_far: f64 = 0.0;

    for chunk in chunks {
        for seg in chunk {
            if seg.text.trim().is_empty() {
                continue;
            }
            if seg.start_s >= max_end_so_far - DEDUP_TOLERANCE_S {
                max_end_so_far = max_end_so_far.max(seg.end_s);
                merged.push(seg);
            }
        }
    }
    merged
}

/// Agrupa segmentos en párrafos legibles: corta por gap de silencio o por
/// duración acumulada.
pub fn group_paragraphs(segments: &[Segment]) -> Vec<String> {
    let mut paragraphs: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut paragraph_start = segments.first().map(|s| s.start_s).unwrap_or(0.0);
    let mut last_end = paragraph_start;

    for seg in segments {
        let gap = seg.start_s - last_end;
        let span = seg.end_s - paragraph_start;
        if !current.is_empty() && (gap > PARAGRAPH_GAP_S || span > PARAGRAPH_MAX_SPAN_S) {
            paragraphs.push(current.trim().to_string());
            current = String::new();
            paragraph_start = seg.start_s;
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(seg.text.trim());
        last_end = seg.end_s;
    }
    if !current.trim().is_empty() {
        paragraphs.push(current.trim().to_string());
    }
    paragraphs
}

/// Heurística anticalidad: una transcripción con muy pocos caracteres por
/// minuto de audio suele estar truncada o vacía (audio sin voz, VAD roto).
pub fn looks_suspicious(segments: &[Segment], audio_duration_s: f64) -> bool {
    if audio_duration_s < 30.0 {
        return false;
    }
    let chars: usize = segments.iter().map(|s| s.text.trim().len()).sum();
    let minutes = audio_duration_s / 60.0;
    (chars as f64 / minutes) < MIN_CHARS_PER_MINUTE
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: f64, end: f64, text: &str) -> Segment {
        Segment {
            start_s: start,
            end_s: end,
            text: text.to_string(),
        }
    }

    #[test]
    fn offset_moves_timeline() {
        let mut segs = vec![seg(0.0, 2.0, "a")];
        offset_segments(&mut segs, 480.0);
        assert_eq!(segs[0].start_s, 480.0);
        assert_eq!(segs[0].end_s, 482.0);
    }

    #[test]
    fn merge_drops_overlap_duplicates() {
        // Chunk 1 cubre 0-10; chunk 2 (con 5s de overlap) repite 5-10 y sigue.
        let c1 = vec![seg(0.0, 5.0, "hola"), seg(5.0, 10.0, "mundo")];
        let c2 = vec![seg(5.2, 9.8, "mundo"), seg(10.1, 15.0, "sigue")];
        let merged = merge_deduplicated(vec![c1, c2]);
        let texts: Vec<_> = merged.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, vec!["hola", "mundo", "sigue"]);
    }

    #[test]
    fn merge_keeps_borderline_within_tolerance() {
        let c1 = vec![seg(0.0, 5.0, "a")];
        let c2 = vec![seg(4.7, 8.0, "b")]; // empieza 0.3s antes del max_end: entra
        let merged = merge_deduplicated(vec![c1, c2]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn paragraphs_split_on_silence_gap() {
        let segs = vec![
            seg(0.0, 2.0, "primera idea"),
            seg(2.2, 4.0, "continúa"),
            seg(9.0, 11.0, "tema nuevo"), // gap de 5s
        ];
        let paras = group_paragraphs(&segs);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0], "primera idea continúa");
        assert_eq!(paras[1], "tema nuevo");
    }

    #[test]
    fn paragraphs_split_on_long_span() {
        let segs: Vec<Segment> = (0..12)
            .map(|i| seg(i as f64 * 5.0, i as f64 * 5.0 + 4.9, "bla"))
            .collect(); // 60s continuos sin gaps
        let paras = group_paragraphs(&segs);
        assert!(paras.len() >= 2, "60s continuos deben partirse en párrafos");
    }

    #[test]
    fn suspicious_detects_empty_transcription() {
        let segs = vec![seg(0.0, 2.0, "hola")];
        assert!(looks_suspicious(&segs, 600.0)); // 4 chars en 10 min
        assert!(!looks_suspicious(&segs, 10.0)); // audios cortos no aplican
    }
}
