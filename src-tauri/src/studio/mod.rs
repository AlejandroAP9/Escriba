//! Estudio de transcripción (Capacidad D): archivo → segmentos con
//! timestamps → SRT/VTT/TXT/JSON + resumen local. Ver PRP-004.

pub mod decode;
pub mod export;
pub mod pipeline;
pub mod playback;
pub mod segments;
