//! Serializadores de exportación del Estudio: SRT, VTT, TXT y JSON.
//! Lógica pura sobre segmentos con timestamps; sin I/O ni audio.

use super::segments::Segment;

fn format_timestamp(seconds: f64, decimal_sep: char) -> String {
    let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let m = (total_s / 60) % 60;
    let h = total_s / 3600;
    format!("{:02}:{:02}:{:02}{}{:03}", h, m, s, decimal_sep, ms)
}

/// SubRip: bloques numerados, timestamps con coma decimal.
pub fn to_srt(segments: &[Segment]) -> String {
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        out.push_str(&format!(
            "{}\n{} --> {}\n{}\n\n",
            i + 1,
            format_timestamp(seg.start_s, ','),
            format_timestamp(seg.end_s, ','),
            seg.text.trim()
        ));
    }
    out
}

/// WebVTT: cabecera obligatoria, timestamps con punto decimal.
pub fn to_vtt(segments: &[Segment]) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for seg in segments {
        out.push_str(&format!(
            "{} --> {}\n{}\n\n",
            format_timestamp(seg.start_s, '.'),
            format_timestamp(seg.end_s, '.'),
            seg.text.trim()
        ));
    }
    out
}

/// Texto plano agrupado en párrafos legibles.
pub fn to_txt(paragraphs: &[String]) -> String {
    paragraphs.join("\n\n")
}

/// JSON estructurado para integraciones.
pub fn to_json(segments: &[Segment]) -> Result<String, String> {
    serde_json::to_string_pretty(segments).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio::segments::Segment;

    fn seg(start: f64, end: f64, text: &str) -> Segment {
        Segment {
            start_s: start,
            end_s: end,
            text: text.to_string(),
        }
    }

    #[test]
    fn srt_formats_block_with_comma_millis() {
        let srt = to_srt(&[seg(0.0, 2.5, "Hola mundo"), seg(2.5, 5.0, "Segunda línea")]);
        assert!(srt.starts_with("1\n00:00:00,000 --> 00:00:02,500\nHola mundo\n\n"));
        assert!(srt.contains("2\n00:00:02,500 --> 00:00:05,000\nSegunda línea\n\n"));
    }

    #[test]
    fn srt_handles_hours() {
        let srt = to_srt(&[seg(3661.25, 3662.0, "x")]);
        assert!(srt.contains("01:01:01,250 --> 01:01:02,000"));
    }

    #[test]
    fn vtt_has_header_and_dot_millis() {
        let vtt = to_vtt(&[seg(0.0, 1.0, "hola")]);
        assert!(vtt.starts_with("WEBVTT\n\n"));
        assert!(vtt.contains("00:00:00.000 --> 00:00:01.000"));
    }

    #[test]
    fn json_roundtrips() {
        let json = to_json(&[seg(1.0, 2.0, "a")]).unwrap();
        assert!(json.contains("\"start_s\": 1.0"));
    }
}
