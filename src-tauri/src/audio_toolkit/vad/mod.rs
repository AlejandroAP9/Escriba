use anyhow::Result;

/// Tramas de 30 ms que se guardan antes del inicio de voz detectado, para no
/// cortar la primera sílaba. 15 × 30 ms = 450 ms.
pub const VAD_PREFILL_FRAMES: usize = 15;

/// Cola tras el final de la voz en dictado por atajo: 15 × 30 ms = 450 ms.
/// Aquí el usuario marca el final soltando la tecla, así que la cola solo
/// existe para no cortar la última sílaba.
pub const VAD_OFFLINE_HANGOVER_FRAMES: usize = 15;

/// Cola larga para ASR de streaming: 55 × 30 ms = 1650 ms.
///
/// **No usar en modos conversacionales.** Ahí el fin de turno lo decide un
/// temporizador de silencio propio (ver `VAD_CONVERSATIONAL_HANGOVER_FRAMES`),
/// y encadenar las dos esperas medía lo mismo dos veces: el turno tardaba
/// 1650 ms de cola + 900 ms de silencio + el sondeo en cerrarse.
pub const VAD_STREAMING_HANGOVER_FRAMES: usize = 55;

/// Cola en manos libres y dictado libre: 15 × 30 ms = 450 ms.
///
/// Basta para no cortar la última sílaba, porque quien decide el fin de turno
/// es el temporizador de silencio que viene después. Una pausa a mitad de frase
/// de hasta ~1,3 s sigue sin partir el turno, porque el buffer no se vacía
/// hasta que pasan 900 ms sin ninguna trama.
pub const VAD_CONVERSATIONAL_HANGOVER_FRAMES: usize = 15;

/// Tramas consecutivas de voz que hacen falta para declarar inicio de habla.
pub const VAD_ONSET_FRAMES: usize = 2;

pub enum VadFrame<'a> {
    /// Speech – may aggregate several frames (prefill + current + hangover)
    Speech(&'a [f32]),
    /// Non-speech (silence, noise). Down-stream code can ignore it.
    Noise,
}

impl<'a> VadFrame<'a> {
    #[inline]
    pub fn is_speech(&self) -> bool {
        matches!(self, VadFrame::Speech(_))
    }
}

pub trait VoiceActivityDetector: Send + Sync {
    /// Primary streaming API: feed one 30-ms frame, get keep/drop decision.
    fn push_frame<'a>(&'a mut self, frame: &'a [f32]) -> Result<VadFrame<'a>>;

    fn is_voice(&mut self, frame: &[f32]) -> Result<bool> {
        Ok(self.push_frame(frame)?.is_speech())
    }

    /// Set the post-speech hangover tail (in 30 ms frames) applied to
    /// subsequent frames. Detectors without a smoothing tail can ignore this.
    fn set_hangover_frames(&mut self, _frames: usize) {}

    fn reset(&mut self) {}
}

mod silero;
mod smoothed;

pub use silero::SileroVad;
pub use smoothed::SmoothedVad;
