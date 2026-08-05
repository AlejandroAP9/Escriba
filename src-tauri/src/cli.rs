use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone, Default)]
#[command(
    name = "escriba",
    about = "Escriba - Tu Escriba personal: hablas, el escribe"
)]
pub struct CliArgs {
    /// Start with the main window hidden
    #[arg(long)]
    pub start_hidden: bool,

    /// Disable the system tray icon
    #[arg(long)]
    pub no_tray: bool,

    /// Toggle transcription on/off (sent to running instance)
    #[arg(long)]
    pub toggle_transcription: bool,

    /// Toggle transcription with post-processing on/off (sent to running instance)
    #[arg(long)]
    pub toggle_post_process: bool,

    /// Cancel the current operation (sent to running instance)
    #[arg(long)]
    pub cancel: bool,

    /// Enable debug mode with verbose logging
    #[arg(long)]
    pub debug: bool,

    /// Transcribe this audio/video file headlessly and exit. Accepts the same
    /// formats as the Studio (wav, mp3, m4a, opus, ogg, flac, mp4/mov…) via
    /// local decode; 16 kHz mono WAV skips the decoder. Runs the same batch
    /// transcription path as the app — no mic, no VAD, no download (the model
    /// must already be installed).
    #[arg(short = 'f', long, value_name = "FILE")]
    pub transcribe_file: Option<PathBuf>,

    /// Model id to load for --transcribe-file (default: the selected model).
    #[arg(long)]
    pub model: Option<String>,

    /// Hard-select the compute device for --transcribe-file by its registry
    /// index (see --list-devices). Omit to use the persisted accelerator
    /// setting. transcribe-cpp (whisper-family) models only.
    #[arg(long, value_name = "N")]
    pub device_index: Option<usize>,

    /// List the transcribe-cpp compute devices (with indices) and exit.
    /// Honors --json for machine-readable output.
    #[arg(long)]
    pub list_devices: bool,

    /// List the available models (with ids) and exit. Pass an id to --model.
    /// Honors --json for machine-readable output.
    #[arg(long)]
    pub list_models: bool,

    /// Repeat the transcription N times (best_ms reports the fastest run).
    #[arg(long, value_name = "N")]
    pub repeat: Option<usize>,

    /// Escriba Studio: write a .srt subtitle file next to the input
    /// (accepts mp3/m4a/mp4/flac/ogg/wav via local decode, any duration).
    #[arg(long)]
    pub export_srt: bool,

    /// Emit --transcribe-file / --list-models / --list-devices results as JSON.
    #[arg(long)]
    pub json: bool,
}
