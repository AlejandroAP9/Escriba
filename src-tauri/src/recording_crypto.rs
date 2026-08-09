//! Contenedor cifrado y acceso por rangos para las grabaciones del historial.
//!
//! Formato `ESCAUD1`:
//! - cabecera fija con largo claro y prefijo aleatorio de nonce;
//! - frames XChaCha20-Poly1305 de 64 KiB;
//! - un tag AEAD por frame, de modo que un seek solo descifra los frames que
//!   toca y una grabación larga no se carga completa en RAM.
//!
//! La webview nunca recibe una ruta de disco ni permiso de filesystem. Un
//! protocolo Tauri privado sirve WAV claro por rangos desde este contenedor.

use anyhow::{anyhow, bail, Context, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::Arc;
use tauri::http::{header, Method, Request, Response, StatusCode};
use tauri::{AppHandle, Manager};

const MAGIC: &[u8; 8] = b"ESCAUD1\0";
const HEADER_LEN: u64 = 8 + 4 + 8 + 16;
const FRAME_SIZE: usize = 64 * 1024;
const TAG_LEN: u64 = 16;
const NONCE_PREFIX_LEN: usize = 16;
/// Incluso si un motor pide `bytes=0-`, nunca se materializa el archivo entero.
const MAX_RESPONSE_BYTES: u64 = 512 * 1024;
const AUDIO_KEY_DOMAIN: &[u8] = b"Escriba/audio-at-rest/v1";

#[derive(Clone, Copy, Debug)]
struct ContainerHeader {
    plaintext_len: u64,
    nonce_prefix: [u8; NONCE_PREFIX_LEN],
}

impl ContainerHeader {
    fn encode(self) -> [u8; HEADER_LEN as usize] {
        let mut bytes = [0u8; HEADER_LEN as usize];
        bytes[0..8].copy_from_slice(MAGIC);
        bytes[8..12].copy_from_slice(&(FRAME_SIZE as u32).to_le_bytes());
        bytes[12..20].copy_from_slice(&self.plaintext_len.to_le_bytes());
        bytes[20..36].copy_from_slice(&self.nonce_prefix);
        bytes
    }

    fn read_from(mut reader: impl Read) -> Result<Self> {
        let mut bytes = [0u8; HEADER_LEN as usize];
        reader
            .read_exact(&mut bytes)
            .context("cabecera de grabación cifrada truncada")?;
        if &bytes[0..8] != MAGIC {
            bail!("la grabación no usa el formato ESCAUD1");
        }
        let frame_size = u32::from_le_bytes(bytes[8..12].try_into()?);
        if frame_size as usize != FRAME_SIZE {
            bail!("tamaño de frame ESCAUD1 no soportado: {frame_size}");
        }
        let plaintext_len = u64::from_le_bytes(bytes[12..20].try_into()?);
        if plaintext_len == 0 {
            bail!("grabación cifrada vacía");
        }
        let mut nonce_prefix = [0u8; NONCE_PREFIX_LEN];
        nonce_prefix.copy_from_slice(&bytes[20..36]);
        Ok(Self {
            plaintext_len,
            nonce_prefix,
        })
    }

    fn frame_count(self) -> u64 {
        self.plaintext_len.div_ceil(FRAME_SIZE as u64)
    }

    fn frame_plain_len(self, frame_index: u64) -> Result<u64> {
        if frame_index >= self.frame_count() {
            bail!("índice de frame fuera de rango");
        }
        let start = frame_index
            .checked_mul(FRAME_SIZE as u64)
            .ok_or_else(|| anyhow!("overflow al resolver frame"))?;
        Ok((self.plaintext_len - start).min(FRAME_SIZE as u64))
    }

    fn expected_file_len(self) -> Result<u64> {
        HEADER_LEN
            .checked_add(self.plaintext_len)
            .and_then(|n| n.checked_add(self.frame_count().checked_mul(TAG_LEN)?))
            .ok_or_else(|| anyhow!("largo de contenedor fuera de rango"))
    }
}

fn derive_audio_key(master: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(AUDIO_KEY_DOMAIN);
    hasher.update(master);
    hasher.finalize().into()
}

fn audio_key() -> Option<[u8; 32]> {
    crate::history_crypto::llave().map(derive_audio_key)
}

fn nonce_for(prefix: &[u8; NONCE_PREFIX_LEN], frame_index: u64) -> [u8; 24] {
    let mut nonce = [0u8; 24];
    nonce[..NONCE_PREFIX_LEN].copy_from_slice(prefix);
    nonce[NONCE_PREFIX_LEN..].copy_from_slice(&frame_index.to_le_bytes());
    nonce
}

fn aad_for(header: ContainerHeader, frame_index: u64, plain_len: u64) -> [u8; 28] {
    let mut aad = [0u8; 28];
    aad[..8].copy_from_slice(MAGIC);
    aad[8..16].copy_from_slice(&header.plaintext_len.to_le_bytes());
    aad[16..24].copy_from_slice(&frame_index.to_le_bytes());
    aad[24..28].copy_from_slice(&(plain_len as u32).to_le_bytes());
    aad
}

struct EncryptingWriter<W: Write> {
    output: W,
    cipher: XChaCha20Poly1305,
    header: ContainerHeader,
    buffer: Vec<u8>,
    frame_index: u64,
    written_plaintext: u64,
}

impl<W: Write> EncryptingWriter<W> {
    fn new(mut output: W, key: &[u8; 32], plaintext_len: u64) -> Result<Self> {
        if plaintext_len == 0 {
            bail!("no se cifra una grabación vacía");
        }
        let mut nonce_prefix = [0u8; NONCE_PREFIX_LEN];
        getrandom::getrandom(&mut nonce_prefix)
            .map_err(|e| anyhow!("sin CSPRNG para el audio: {e}"))?;
        let header = ContainerHeader {
            plaintext_len,
            nonce_prefix,
        };
        output.write_all(&header.encode())?;
        Ok(Self {
            output,
            cipher: XChaCha20Poly1305::new(Key::from_slice(key)),
            header,
            buffer: Vec::with_capacity(FRAME_SIZE),
            frame_index: 0,
            written_plaintext: 0,
        })
    }

    fn flush_frame(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let plain_len = self.buffer.len() as u64;
        let nonce = nonce_for(&self.header.nonce_prefix, self.frame_index);
        let aad = aad_for(self.header, self.frame_index, plain_len);
        let encrypted = self
            .cipher
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &self.buffer,
                    aad: &aad,
                },
            )
            .map_err(|_| anyhow!("falló el cifrado de un frame de audio"))?;
        self.output.write_all(&encrypted)?;
        self.written_plaintext += plain_len;
        self.frame_index += 1;
        self.buffer.clear();
        Ok(())
    }

    fn finish(mut self) -> Result<W> {
        self.flush_frame()?;
        if self.written_plaintext != self.header.plaintext_len {
            bail!(
                "largo claro inesperado: esperados {}, escritos {}",
                self.header.plaintext_len,
                self.written_plaintext
            );
        }
        self.output.flush()?;
        Ok(self.output)
    }
}

impl<W: Write> Write for EncryptingWriter<W> {
    fn write(&mut self, mut bytes: &[u8]) -> io::Result<usize> {
        let original_len = bytes.len();
        while !bytes.is_empty() {
            let available = FRAME_SIZE - self.buffer.len();
            let take = available.min(bytes.len());
            self.buffer.extend_from_slice(&bytes[..take]);
            bytes = &bytes[take..];
            if self.buffer.len() == FRAME_SIZE {
                self.flush_frame().map_err(io::Error::other)?;
            }
        }
        Ok(original_len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

fn restrict_file_to_owner(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = path;
}

fn write_atomically(destination: &Path, write: impl FnOnce(&mut File) -> Result<()>) -> Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow!("la grabación no tiene carpeta padre"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    restrict_file_to_owner(temporary.path());
    write(temporary.as_file_mut())?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(destination)
        .map_err(|e| e.error)
        .with_context(|| format!("no se pudo publicar {}", destination.display()))?;
    restrict_file_to_owner(destination);
    Ok(())
}

fn wav_header(sample_count: usize) -> Result<[u8; 44]> {
    let data_len = sample_count
        .checked_mul(2)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| anyhow!("grabación demasiado grande para WAV PCM"))?;
    let riff_len = 36u32
        .checked_add(data_len)
        .ok_or_else(|| anyhow!("grabación demasiado grande para WAV PCM"))?;
    let mut header = [0u8; 44];
    header[0..4].copy_from_slice(b"RIFF");
    header[4..8].copy_from_slice(&riff_len.to_le_bytes());
    header[8..12].copy_from_slice(b"WAVE");
    header[12..16].copy_from_slice(b"fmt ");
    header[16..20].copy_from_slice(&16u32.to_le_bytes());
    header[20..22].copy_from_slice(&1u16.to_le_bytes());
    header[22..24].copy_from_slice(&1u16.to_le_bytes());
    header[24..28].copy_from_slice(&16_000u32.to_le_bytes());
    header[28..32].copy_from_slice(&32_000u32.to_le_bytes());
    header[32..34].copy_from_slice(&2u16.to_le_bytes());
    header[34..36].copy_from_slice(&16u16.to_le_bytes());
    header[36..40].copy_from_slice(b"data");
    header[40..44].copy_from_slice(&data_len.to_le_bytes());
    Ok(header)
}

/// Guarda WAV PCM 16 kHz mono directamente dentro de ESCAUD1. No existe un
/// WAV temporal en disco ni siquiera durante la escritura.
pub fn save_encrypted_wav(path: &Path, samples: &[f32]) -> Result<()> {
    let key = audio_key().ok_or_else(|| anyhow!("llave del historial no disponible"))?;
    save_encrypted_wav_with_key(path, samples, &key)
}

fn save_encrypted_wav_with_key(path: &Path, samples: &[f32], key: &[u8; 32]) -> Result<()> {
    let wav_header = wav_header(samples.len())?;
    let plaintext_len = 44u64
        .checked_add(
            (samples.len() as u64)
                .checked_mul(2)
                .ok_or_else(|| anyhow!("overflow"))?,
        )
        .ok_or_else(|| anyhow!("overflow"))?;
    write_atomically(path, |file| {
        let mut writer = EncryptingWriter::new(file, key, plaintext_len)?;
        writer.write_all(&wav_header)?;
        let mut pcm = [0u8; 8192];
        for chunk in samples.chunks(pcm.len() / 2) {
            for (index, sample) in chunk.iter().enumerate() {
                let value = (sample * i16::MAX as f32) as i16;
                pcm[index * 2..index * 2 + 2].copy_from_slice(&value.to_le_bytes());
            }
            writer.write_all(&pcm[..chunk.len() * 2])?;
        }
        writer.finish()?;
        Ok(())
    })
}

/// Completa o recupera la migración de un único WAV heredado. Es re-ejecutable:
/// si el destino cifrado ya fue publicado, se valida y solo se retira la copia
/// clara que pudo quedar por un kill entre pasos.
pub fn migrate_legacy_wav(source: &Path, destination: &Path) -> Result<()> {
    let key = audio_key().ok_or_else(|| anyhow!("llave del historial no disponible"))?;
    migrate_legacy_wav_with_key(source, destination, &key)
}

fn migrate_legacy_wav_with_key(source: &Path, destination: &Path, key: &[u8; 32]) -> Result<()> {
    if destination.is_file() {
        validate_encrypted_wav_with_key(destination, key)?;
    } else {
        if !source.is_file() {
            bail!("grabación heredada no encontrada");
        }
        if is_encrypted(source)? {
            validate_encrypted_wav_with_key(source, key)?;
            fs::rename(source, destination)?;
        } else {
            encrypt_existing_wav_with_key(source, destination, key)?;
        }
    }
    if source.exists() {
        fs::remove_file(source)?;
    }
    Ok(())
}

fn encrypt_existing_wav_with_key(source: &Path, destination: &Path, key: &[u8; 32]) -> Result<()> {
    // Hound valida la estructura antes de reemplazar o borrar nada.
    hound::WavReader::open(source)
        .with_context(|| format!("WAV heredado inválido: {}", source.display()))?;
    let plaintext_len = fs::metadata(source)?.len();
    write_atomically(destination, |output| {
        let mut input = BufReader::new(File::open(source)?);
        let mut writer = EncryptingWriter::new(output, key, plaintext_len)?;
        io::copy(&mut input, &mut writer)?;
        writer.finish()?;
        Ok(())
    })
}

pub fn is_encrypted(path: &Path) -> Result<bool> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 8];
    match file.read_exact(&mut magic) {
        Ok(()) => Ok(&magic == MAGIC),
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e.into()),
    }
}

fn read_and_validate_header(file: &mut File) -> Result<ContainerHeader> {
    file.seek(SeekFrom::Start(0))?;
    let header = ContainerHeader::read_from(&mut *file)?;
    let actual = file.metadata()?.len();
    let expected = header.expected_file_len()?;
    if actual != expected {
        bail!("contenedor ESCAUD1 truncado: esperados {expected} bytes, hay {actual}");
    }
    Ok(header)
}

struct EncryptedReader {
    file: File,
    cipher: XChaCha20Poly1305,
    header: ContainerHeader,
    position: u64,
    cached_frame: Option<u64>,
    cached_plaintext: Vec<u8>,
}

impl EncryptedReader {
    fn new(mut file: File, key: &[u8; 32], position: u64) -> Result<Self> {
        let header = read_and_validate_header(&mut file)?;
        if position > header.plaintext_len {
            bail!("posición clara fuera de rango");
        }
        Ok(Self {
            file,
            cipher: XChaCha20Poly1305::new(Key::from_slice(key)),
            header,
            position,
            cached_frame: None,
            cached_plaintext: Vec::with_capacity(FRAME_SIZE),
        })
    }

    fn load_frame(&mut self, frame_index: u64) -> Result<()> {
        if self.cached_frame == Some(frame_index) {
            return Ok(());
        }
        let plain_len = self.header.frame_plain_len(frame_index)?;
        let encrypted_offset = HEADER_LEN
            .checked_add(
                frame_index
                    .checked_mul(FRAME_SIZE as u64 + TAG_LEN)
                    .ok_or_else(|| anyhow!("overflow al resolver frame cifrado"))?,
            )
            .ok_or_else(|| anyhow!("overflow al resolver frame cifrado"))?;
        let encrypted_len = plain_len + TAG_LEN;
        let mut encrypted = vec![0u8; encrypted_len as usize];
        self.file.seek(SeekFrom::Start(encrypted_offset))?;
        self.file.read_exact(&mut encrypted)?;
        let nonce = nonce_for(&self.header.nonce_prefix, frame_index);
        let aad = aad_for(self.header, frame_index, plain_len);
        self.cached_plaintext = self
            .cipher
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &encrypted,
                    aad: &aad,
                },
            )
            .map_err(|_| anyhow!("frame de audio corrupto o llave incorrecta"))?;
        if self.cached_plaintext.len() != plain_len as usize {
            bail!("largo descifrado inesperado");
        }
        self.cached_frame = Some(frame_index);
        Ok(())
    }
}

impl Read for EncryptedReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if output.is_empty() || self.position >= self.header.plaintext_len {
            return Ok(0);
        }
        let mut written = 0;
        while written < output.len() && self.position < self.header.plaintext_len {
            let frame_index = self.position / FRAME_SIZE as u64;
            self.load_frame(frame_index).map_err(io::Error::other)?;
            let within_frame = (self.position % FRAME_SIZE as u64) as usize;
            let available = self.cached_plaintext.len() - within_frame;
            let take = available.min(output.len() - written);
            output[written..written + take]
                .copy_from_slice(&self.cached_plaintext[within_frame..within_frame + take]);
            written += take;
            self.position += take as u64;
        }
        Ok(written)
    }
}

/// Lee una grabación heredada o ESCAUD1 para re-transcribirla. Hound consume
/// el stream; no se crea una copia WAV temporal.
pub fn read_wav_samples(path: &Path) -> Result<Vec<f32>> {
    if is_encrypted(path)? {
        let key = audio_key().ok_or_else(|| anyhow!("llave del historial no disponible"))?;
        read_encrypted_samples_with_key(path, &key)
    } else {
        crate::audio_toolkit::read_wav_samples(path)
    }
}

fn read_encrypted_samples_with_key(path: &Path, key: &[u8; 32]) -> Result<Vec<f32>> {
    let reader = EncryptedReader::new(File::open(path)?, key, 0)?;
    let wav = hound::WavReader::new(reader)?;
    wav.into_samples::<i16>()
        .map(|sample| sample.map(|v| v as f32 / i16::MAX as f32))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

/// Verifica cabecera, todos los tags AEAD y el conteo de muestras sin retener
/// el WAV claro completo.
pub fn verify_encrypted_wav(path: &Path, expected_samples: usize) -> Result<()> {
    let key = audio_key().ok_or_else(|| anyhow!("llave del historial no disponible"))?;
    verify_encrypted_wav_with_key(path, expected_samples, &key)
}

fn validate_encrypted_wav_with_key(path: &Path, key: &[u8; 32]) -> Result<()> {
    let reader = EncryptedReader::new(File::open(path)?, key, 0)?;
    let mut wav = hound::WavReader::new(reader)?;
    for sample in wav.samples::<i16>() {
        sample?;
    }
    Ok(())
}

fn verify_encrypted_wav_with_key(
    path: &Path,
    expected_samples: usize,
    key: &[u8; 32],
) -> Result<()> {
    let reader = EncryptedReader::new(File::open(path)?, key, 0)?;
    let mut wav = hound::WavReader::new(reader)?;
    if wav.len() as usize != expected_samples {
        bail!(
            "WAV sample count mismatch: expected {}, got {}",
            expected_samples,
            wav.len()
        );
    }
    for sample in wav.samples::<i16>() {
        sample?;
    }
    Ok(())
}

pub fn is_safe_file_name(file_name: &str) -> bool {
    !file_name.is_empty()
        && file_name.len() <= 160
        && file_name != "."
        && file_name != ".."
        && file_name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
}

pub fn encrypted_file_name(legacy_name: &str) -> Option<String> {
    if !is_safe_file_name(legacy_name) {
        return None;
    }
    let stem = Path::new(legacy_name).file_stem()?.to_str()?;
    Some(format!("{stem}.escaudio"))
}

/// Comprueba que el comando puede entregar una URL útil antes de que el
/// elemento `<audio>` haga la petición.
pub fn ensure_playable(path: &Path) -> Result<()> {
    if !path.is_file() {
        bail!("grabación no encontrada");
    }
    if is_encrypted(path)? {
        let key = audio_key().ok_or_else(|| anyhow!("llave del historial no disponible"))?;
        let mut reader = EncryptedReader::new(File::open(path)?, &key, 0)?;
        let mut wav_magic = [0u8; 12];
        reader.read_exact(&mut wav_magic)?;
        if &wav_magic[..4] != b"RIFF" || &wav_magic[8..] != b"WAVE" {
            bail!("contenido WAV cifrado inválido");
        }
    } else {
        hound::WavReader::open(path)?;
    }
    Ok(())
}

pub fn playback_url(file_name: &str) -> Result<String> {
    if !is_safe_file_name(file_name) {
        bail!("nombre de grabación inválido");
    }
    #[cfg(windows)]
    return Ok(format!("http://escriba-audio.localhost/{file_name}"));
    #[cfg(not(windows))]
    Ok(format!("escriba-audio://localhost/{file_name}"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ByteRange {
    start: u64,
    end_inclusive: u64,
}

fn requested_range(value: Option<&str>, total: u64) -> Result<ByteRange> {
    if total == 0 {
        bail!("archivo vacío");
    }
    let Some(value) = value else {
        return Ok(ByteRange {
            start: 0,
            end_inclusive: (total - 1).min(MAX_RESPONSE_BYTES - 1),
        });
    };
    let spec = value
        .strip_prefix("bytes=")
        .ok_or_else(|| anyhow!("unidad Range no soportada"))?;
    if spec.contains(',') {
        bail!("rangos múltiples no soportados");
    }
    let (start, end) = spec
        .split_once('-')
        .ok_or_else(|| anyhow!("Range inválido"))?;
    let (start, requested_end) = if start.is_empty() {
        let suffix: u64 = end.parse().context("sufijo Range inválido")?;
        if suffix == 0 {
            bail!("sufijo Range vacío");
        }
        (total.saturating_sub(suffix), total - 1)
    } else {
        let start: u64 = start.parse().context("inicio Range inválido")?;
        if start >= total {
            bail!("inicio Range fuera del archivo");
        }
        let end = if end.is_empty() {
            total - 1
        } else {
            end.parse().context("fin Range inválido")?
        };
        (start, end.min(total - 1))
    };
    if requested_end < start {
        bail!("Range invertido");
    }
    Ok(ByteRange {
        start,
        end_inclusive: requested_end.min(start + MAX_RESPONSE_BYTES - 1),
    })
}

fn read_plain_range(path: &Path, range: ByteRange) -> Result<Vec<u8>> {
    let count = range.end_inclusive - range.start + 1;
    let mut output = vec![0u8; count as usize];
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(range.start))?;
    file.read_exact(&mut output)?;
    Ok(output)
}

fn read_encrypted_range(path: &Path, range: ByteRange, key: &[u8; 32]) -> Result<Vec<u8>> {
    let count = range.end_inclusive - range.start + 1;
    let mut output = vec![0u8; count as usize];
    let mut reader = EncryptedReader::new(File::open(path)?, key, range.start)?;
    reader.read_exact(&mut output)?;
    Ok(output)
}

fn response(status: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    let mut result = Response::new(body);
    *result.status_mut() = status;
    result.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    result.headers_mut().insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_static("*"),
    );
    result.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );
    result
}

fn error_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    let mut result = response(status, message.as_bytes().to_vec());
    result.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    result
}

/// Handler del protocolo `escriba-audio`. Solo acepta nombres planos dentro de
/// `recordings/`; no expone un lector de archivos arbitrario a la webview.
pub fn protocol_response(
    app: &AppHandle,
    webview_label: &str,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    if webview_label != "main" {
        return error_response(StatusCode::FORBIDDEN, "webview not allowed");
    }
    if request.method() == Method::OPTIONS {
        return response(StatusCode::NO_CONTENT, Vec::new());
    }
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return error_response(StatusCode::METHOD_NOT_ALLOWED, "method not allowed");
    }
    let file_name = request.uri().path().trim_start_matches('/');
    if !is_safe_file_name(file_name) {
        return error_response(StatusCode::BAD_REQUEST, "invalid recording name");
    }
    let Some(history) = app.try_state::<Arc<crate::managers::history::HistoryManager>>() else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "history unavailable");
    };
    match history.contains_audio_file(file_name) {
        Ok(true) => {}
        Ok(false) => return error_response(StatusCode::NOT_FOUND, "recording not found"),
        Err(_) => return error_response(StatusCode::SERVICE_UNAVAILABLE, "history unavailable"),
    }
    let path = match history.get_audio_file_path(file_name) {
        Ok(path) => path,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "invalid recording name"),
    };
    if !path.is_file() {
        return error_response(StatusCode::NOT_FOUND, "recording not found");
    }

    let encrypted = match is_encrypted(&path) {
        Ok(value) => value,
        Err(_) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, "recording unreadable"),
    };
    let (total, key) = if encrypted {
        let Some(key) = audio_key() else {
            return error_response(StatusCode::SERVICE_UNAVAILABLE, "history key unavailable");
        };
        let mut file = match File::open(&path) {
            Ok(file) => file,
            Err(_) => return error_response(StatusCode::NOT_FOUND, "recording not found"),
        };
        let header = match read_and_validate_header(&mut file) {
            Ok(header) => header,
            Err(_) => return error_response(StatusCode::UNPROCESSABLE_ENTITY, "recording corrupt"),
        };
        (header.plaintext_len, Some(key))
    } else {
        match fs::metadata(&path) {
            Ok(metadata) => (metadata.len(), None),
            Err(_) => return error_response(StatusCode::NOT_FOUND, "recording not found"),
        }
    };

    if request.method() == Method::HEAD {
        let mut result = response(StatusCode::OK, Vec::new());
        result.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("audio/wav"),
        );
        result.headers_mut().insert(
            header::ACCEPT_RANGES,
            header::HeaderValue::from_static("bytes"),
        );
        if let Ok(value) = header::HeaderValue::from_str(&total.to_string()) {
            result.headers_mut().insert(header::CONTENT_LENGTH, value);
        }
        return result;
    }

    let range_header = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let range = match requested_range(range_header, total) {
        Ok(range) => range,
        Err(_) => {
            let mut result = error_response(StatusCode::RANGE_NOT_SATISFIABLE, "invalid range");
            if let Ok(value) = header::HeaderValue::from_str(&format!("bytes */{total}")) {
                result.headers_mut().insert(header::CONTENT_RANGE, value);
            }
            return result;
        }
    };
    let body = match key {
        Some(key) => read_encrypted_range(&path, range, &key),
        None => read_plain_range(&path, range),
    };
    let Ok(body) = body else {
        return error_response(StatusCode::UNPROCESSABLE_ENTITY, "recording corrupt");
    };
    let partial = range_header.is_some() || range.start > 0 || range.end_inclusive + 1 < total;
    let mut result = response(
        if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        },
        body,
    );
    result.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("audio/wav"),
    );
    result.headers_mut().insert(
        header::ACCEPT_RANGES,
        header::HeaderValue::from_static("bytes"),
    );
    if let Ok(value) =
        header::HeaderValue::from_str(&(range.end_inclusive - range.start + 1).to_string())
    {
        result.headers_mut().insert(header::CONTENT_LENGTH, value);
    }
    if partial {
        if let Ok(value) = header::HeaderValue::from_str(&format!(
            "bytes {}-{}/{}",
            range.start, range.end_inclusive, total
        )) {
            result.headers_mut().insert(header::CONTENT_RANGE, value);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASTER: [u8; 32] = [17u8; 32];

    fn audio_test_key() -> [u8; 32] {
        derive_audio_key(&MASTER)
    }

    #[test]
    fn encrypted_wav_is_not_a_wav_and_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("voice.escaudio");
        let samples: Vec<f32> = (0..80_000)
            .map(|index| ((index as f32 / 53.0).sin() * 0.75).clamp(-1.0, 1.0))
            .collect();
        let key = audio_test_key();
        save_encrypted_wav_with_key(&path, &samples, &key).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(&bytes[..8], MAGIC);
        assert!(!bytes.windows(4).any(|window| window == b"RIFF"));

        verify_encrypted_wav_with_key(&path, samples.len(), &key).unwrap();
        let decoded = read_encrypted_samples_with_key(&path, &key).unwrap();
        assert_eq!(decoded.len(), samples.len());
        for (actual, expected) in decoded.iter().zip(samples.iter()).step_by(997) {
            assert!((actual - expected).abs() < 0.000_1);
        }
    }

    #[test]
    fn range_crosses_frames_without_materialising_the_whole_file() {
        let dir = tempfile::tempdir().unwrap();
        let plain_path = dir.path().join("legacy.wav");
        let encrypted_path = dir.path().join("voice.escaudio");
        let samples = vec![0.25f32; 500_000];
        crate::audio_toolkit::save_wav_file(&plain_path, &samples).unwrap();
        let expected = fs::read(&plain_path).unwrap();
        let key = audio_test_key();
        encrypt_existing_wav_with_key(&plain_path, &encrypted_path, &key).unwrap();

        let range = ByteRange {
            start: FRAME_SIZE as u64 - 13,
            end_inclusive: FRAME_SIZE as u64 + 79,
        };
        let actual = read_encrypted_range(&encrypted_path, range, &key).unwrap();
        assert_eq!(
            actual,
            expected[range.start as usize..=range.end_inclusive as usize]
        );

        let capped = requested_range(Some("bytes=0-"), expected.len() as u64).unwrap();
        assert_eq!(capped.end_inclusive + 1, MAX_RESPONSE_BYTES);
    }

    #[test]
    fn modified_frame_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("voice.escaudio");
        let samples = vec![0.5f32; 40_000];
        let key = audio_test_key();
        save_encrypted_wav_with_key(&path, &samples, &key).unwrap();
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.seek(SeekFrom::Start(HEADER_LEN + 123)).unwrap();
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte).unwrap();
        file.seek(SeekFrom::Current(-1)).unwrap();
        byte[0] ^= 0x80;
        file.write_all(&byte).unwrap();
        drop(file);
        assert!(verify_encrypted_wav_with_key(&path, samples.len(), &key).is_err());
    }

    #[test]
    fn legacy_migration_is_idempotent_and_recovers_after_publish() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("voice.wav");
        let destination = dir.path().join("voice.escaudio");
        let samples = vec![0.125f32; 70_000];
        crate::audio_toolkit::save_wav_file(&source, &samples).unwrap();
        let key = audio_test_key();

        // Simula kill después de publicar el cifrado pero antes de borrar el WAV.
        encrypt_existing_wav_with_key(&source, &destination, &key).unwrap();
        assert!(source.exists());
        assert!(destination.exists());

        migrate_legacy_wav_with_key(&source, &destination, &key).unwrap();
        assert!(!source.exists());
        validate_encrypted_wav_with_key(&destination, &key).unwrap();

        // Repetir con el destino ya asentado no cifra dos veces ni falla.
        let before = fs::read(&destination).unwrap();
        migrate_legacy_wav_with_key(&source, &destination, &key).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), before);
    }

    #[test]
    fn range_parser_supports_open_and_suffix_ranges() {
        assert_eq!(
            requested_range(Some("bytes=10-19"), 100).unwrap(),
            ByteRange {
                start: 10,
                end_inclusive: 19
            }
        );
        assert_eq!(
            requested_range(Some("bytes=-10"), 100).unwrap(),
            ByteRange {
                start: 90,
                end_inclusive: 99
            }
        );
        assert!(requested_range(Some("bytes=100-"), 100).is_err());
        assert!(requested_range(Some("items=0-1"), 100).is_err());
    }

    #[test]
    fn file_names_cannot_escape_recordings() {
        for invalid in [
            "",
            ".",
            "..",
            "../secret",
            "a/b.wav",
            "a%2Fb.wav",
            "voz ñ.wav",
        ] {
            assert!(!is_safe_file_name(invalid), "debía rechazar {invalid:?}");
        }
        assert!(is_safe_file_name("escriba-123.escaudio"));
    }
}
