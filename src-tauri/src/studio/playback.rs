//! Reproducción segura de los archivos aceptados por el Estudio.
//!
//! La webview recibe un id opaco, nunca una ruta ni permisos de filesystem. El
//! protocolo vuelve a comprobar que el job existe y que su ruta sigue dentro de
//! las carpetas permitidas; responde por rangos para no cargar un video entero.

use super::decode;
use crate::commands::studio::{JobStatus, StudioState};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use tauri::http::{header, Method, Request, Response, StatusCode};
use tauri::{AppHandle, Manager};

const MAX_RESPONSE_BYTES: u64 = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ByteRange {
    start: u64,
    end_inclusive: u64,
}

fn requested_range(value: Option<&str>, total: u64) -> Result<ByteRange, ()> {
    if total == 0 {
        return Err(());
    }
    let Some(value) = value else {
        return Ok(ByteRange {
            start: 0,
            end_inclusive: (total - 1).min(MAX_RESPONSE_BYTES - 1),
        });
    };
    let spec = value.strip_prefix("bytes=").ok_or(())?;
    if spec.contains(',') {
        return Err(());
    }
    let (start, end) = spec.split_once('-').ok_or(())?;
    let (start, requested_end) = if start.is_empty() {
        let suffix: u64 = end.parse().map_err(|_| ())?;
        if suffix == 0 {
            return Err(());
        }
        (total.saturating_sub(suffix), total - 1)
    } else {
        let start: u64 = start.parse().map_err(|_| ())?;
        if start >= total {
            return Err(());
        }
        let end = if end.is_empty() {
            total - 1
        } else {
            end.parse::<u64>().map_err(|_| ())?.min(total - 1)
        };
        (start, end)
    };
    if requested_end < start {
        return Err(());
    }
    Ok(ByteRange {
        start,
        end_inclusive: requested_end.min(start + MAX_RESPONSE_BYTES - 1),
    })
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

fn media_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("m4a" | "aac") => "audio/mp4",
        Some("mp4" | "mov") => "video/mp4",
        Some("flac") => "audio/flac",
        Some("ogg" | "oga" | "opus") => "audio/ogg",
        Some("aiff" | "aif") => "audio/aiff",
        Some("caf") => "audio/x-caf",
        _ => "application/octet-stream",
    }
}

pub fn playback_url(id: u64) -> String {
    #[cfg(windows)]
    return format!("http://escriba-studio.localhost/{id}");
    #[cfg(not(windows))]
    format!("escriba-studio://localhost/{id}")
}

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

    let Ok(id) = request.uri().path().trim_start_matches('/').parse::<u64>() else {
        return error_response(StatusCode::BAD_REQUEST, "invalid studio job");
    };
    let Some(state) = app.try_state::<Arc<StudioState>>() else {
        return error_response(StatusCode::SERVICE_UNAVAILABLE, "studio unavailable");
    };
    let raw_path = {
        let jobs = state.jobs.lock().unwrap_or_else(|error| error.into_inner());
        let Some(job) = jobs
            .iter()
            .find(|job| job.id == id && job.status == JobStatus::Done)
        else {
            return error_response(StatusCode::NOT_FOUND, "studio job not found");
        };
        std::path::PathBuf::from(&job.path)
    };
    if !decode::supported_extension(&raw_path) {
        return error_response(StatusCode::UNSUPPORTED_MEDIA_TYPE, "media not supported");
    }
    let path = match crate::path_guard::contain_media_path(app, &raw_path, "media unavailable") {
        Ok(path) if path.is_file() => path,
        _ => return error_response(StatusCode::NOT_FOUND, "media unavailable"),
    };
    let total = match fs::metadata(&path) {
        Ok(metadata) => metadata.len(),
        Err(_) => return error_response(StatusCode::NOT_FOUND, "media unavailable"),
    };
    let content_type = media_type(&path);

    if request.method() == Method::HEAD {
        let mut result = response(StatusCode::OK, Vec::new());
        if let Ok(value) = header::HeaderValue::from_str(content_type) {
            result.headers_mut().insert(header::CONTENT_TYPE, value);
        }
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
    let count = range.end_inclusive - range.start + 1;
    let body = (|| -> std::io::Result<Vec<u8>> {
        let mut output = vec![0u8; count as usize];
        let mut file = File::open(&path)?;
        file.seek(SeekFrom::Start(range.start))?;
        file.read_exact(&mut output)?;
        Ok(output)
    })();
    let Ok(body) = body else {
        return error_response(StatusCode::UNPROCESSABLE_ENTITY, "media unreadable");
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
    if let Ok(value) = header::HeaderValue::from_str(content_type) {
        result.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    result.headers_mut().insert(
        header::ACCEPT_RANGES,
        header::HeaderValue::from_static("bytes"),
    );
    if let Ok(value) = header::HeaderValue::from_str(&count.to_string()) {
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

    #[test]
    fn ranges_are_bounded_and_support_suffixes() {
        assert_eq!(
            requested_range(Some("bytes=10-20"), 100),
            Ok(ByteRange {
                start: 10,
                end_inclusive: 20
            })
        );
        assert_eq!(
            requested_range(Some("bytes=-5"), 100),
            Ok(ByteRange {
                start: 95,
                end_inclusive: 99
            })
        );
        let bounded = requested_range(Some("bytes=0-"), MAX_RESPONSE_BYTES * 2).unwrap();
        assert_eq!(bounded.end_inclusive, MAX_RESPONSE_BYTES - 1);
        assert!(requested_range(Some("bytes=200-"), 100).is_err());
    }

    #[test]
    fn content_type_is_derived_only_from_supported_extensions() {
        assert_eq!(media_type(Path::new("audio.mp3")), "audio/mpeg");
        assert_eq!(media_type(Path::new("video.mp4")), "video/mp4");
        assert_eq!(
            media_type(Path::new("archivo.bin")),
            "application/octet-stream"
        );
    }
}
