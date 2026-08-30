use crate::settings::PostProcessProvider;
use log::{debug, warn};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, REFERER, USER_AGENT};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

/// Tiempo máximo para establecer la conexión con el proveedor.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Tiempo máximo para la petición COMPLETA (camino interactivo).
///
/// Antes no había ninguno: un proveedor que aceptaba la conexión y no respondía
/// dejaba la tarea colgada indefinidamente, y con ella el dictado del usuario,
/// que está mirando la pantalla esperando su texto. 30 s da margen de sobra a un
/// modelo con razonamiento sin dejar la app en un limbo permanente.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Tiempo máximo para las peticiones de FORMATO LARGO: el documento de sesión,
/// resumir, pulir o traducir un texto entero.
///
/// Esas peticiones meten el transcript completo en el prompt y piden un
/// documento de vuelta: en el motor local, solo procesar una sesión de una hora
/// puede tardar más de 30 s antes de generar el primer token. Con el timeout
/// interactivo, "Terminar y crear documento" moría a los 30 s exactos y el
/// usuario veía "revisa el motor local" con el motor perfectamente sano
/// (reporte real, 27-jul-2026). Cinco minutos cubre sesiones largas en
/// hardware modesto sin renunciar a la protección contra el cuelgue infinito.
pub const LONG_FORM_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// Espera para lo que ocurre EN VIVO, donde llegar tarde equivale a no llegar.
///
/// El Intérprete traduce frase a frase mientras el guía habla: medido con el
/// motor local, cada traducción cuesta de 1 a 2 segundos, y 30 dejan margen de
/// sobra incluso si el modelo tiene que cargarse en la primera. Pasado eso, esa
/// traducción ya no le sirve a nadie y se descarta para no bloquear la
/// siguiente frase — con la espera larga, una petición atascada congelaba la
/// sala cinco minutos.
pub const LIVE_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Error del cliente, con la causa distinguida para que quien llama pueda
/// reaccionar distinto ante un límite de tasa que ante un fallo cualquiera.
#[derive(Debug)]
pub enum LlmError {
    /// El proveedor devolvió 429. `retry_after` viene de la cabecera homónima
    /// cuando el proveedor la manda.
    RateLimited { retry_after: Option<Duration> },
    /// Se agotó el tiempo de espera.
    Timeout,
    /// Cualquier otro fallo (HTTP, red, parseo).
    Other(String),
}

fn should_retry_managed_request(uses_managed_sidecar: bool, timed_out: bool) -> bool {
    uses_managed_sidecar && !timed_out
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::RateLimited { retry_after } => match retry_after {
                Some(d) => write!(
                    f,
                    "límite de peticiones alcanzado (reintentar en {}s)",
                    d.as_secs()
                ),
                None => write!(f, "límite de peticiones alcanzado"),
            },
            LlmError::Timeout => write!(f, "el proveedor no respondió a tiempo"),
            LlmError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<LlmError> for String {
    fn from(e: LlmError) -> String {
        e.to_string()
    }
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct JsonSchema {
    name: String,
    strict: bool,
    schema: Value,
}

#[derive(Debug, Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: JsonSchema,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct ReasoningConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
struct ChatMessageResponse {
    content: Option<String>,
}

/// Build headers for API requests based on provider type
fn build_headers(provider: &PostProcessProvider, api_key: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();

    // Common headers
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://github.com/AlejandroAP9/Escriba"),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("Escriba/1.0 (+https://github.com/AlejandroAP9/Escriba)"),
    );
    headers.insert("X-Title", HeaderValue::from_static("Escriba"));

    // Provider-specific auth headers
    if !api_key.is_empty() {
        if provider.id == "anthropic" {
            headers.insert(
                "x-api-key",
                HeaderValue::from_str(api_key)
                    .map_err(|e| format!("Invalid API key header value: {}", e))?,
            );
            headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        } else {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| format!("Invalid authorization header value: {}", e))?,
            );
        }
    }

    Ok(headers)
}

/// Create an HTTP client with provider-specific headers
fn create_client(
    provider: &PostProcessProvider,
    api_key: &str,
    request_timeout: Duration,
) -> Result<reqwest::Client, String> {
    let headers = build_headers(provider, api_key)?;
    reqwest::Client::builder()
        .default_headers(headers)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(request_timeout)
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {}", e))
}

/// Lee `Retry-After`, que puede venir en segundos o como fecha HTTP. Solo se
/// interpreta la forma en segundos, que es la que usan los proveedores LLM.
fn parse_retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

/// Send a chat completion request to an OpenAI-compatible API
/// Returns Ok(Some(content)) on success, Ok(None) if response has no content,
/// or Err on actual errors (HTTP, parsing, etc.)
/// `request_timeout`: `None` usa el timeout interactivo (30 s); las peticiones
/// de formato largo pasan [`LONG_FORM_REQUEST_TIMEOUT`].
#[allow(clippy::too_many_arguments)]
pub async fn send_chat_completion(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    prompt: String,
    reasoning_effort: Option<String>,
    reasoning: Option<ReasoningConfig>,
    temperature: Option<f32>,
    request_timeout: Option<Duration>,
) -> Result<Option<String>, String> {
    send_chat_completion_with_schema(
        provider,
        api_key,
        model,
        prompt,
        None,
        None,
        reasoning_effort,
        reasoning,
        temperature,
        request_timeout,
    )
    .await
}

/// Send a chat completion request with structured output support
/// When json_schema is provided, uses structured outputs mode
/// system_prompt is used as the system message when provided
/// reasoning_effort sets the OpenAI-style top-level field (e.g., "none", "low", "medium", "high")
/// reasoning sets the OpenRouter-style nested object (effort + exclude)
#[allow(clippy::too_many_arguments)]
pub async fn send_chat_completion_with_schema(
    provider: &PostProcessProvider,
    api_key: String,
    model: &str,
    user_content: String,
    system_prompt: Option<String>,
    json_schema: Option<Value>,
    reasoning_effort: Option<String>,
    reasoning: Option<ReasoningConfig>,
    temperature: Option<f32>,
    request_timeout: Option<Duration>,
) -> Result<Option<String>, String> {
    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/chat/completions", base_url);

    // El sidecar se descarga tras unos minutos sin uso para liberar RAM. Una
    // generación larga (acta, resumen) también puede durar varios minutos y
    // antes el watcher la confundía con inactividad, matando llama-server a
    // mitad de respuesta. El lease vive hasta que termina este future.
    // `local_llm` también puede degradar a Ollama. Solo el puerto efímero que
    // administra Escriba debe adquirir lease o reiniciarse automáticamente;
    // nunca se toma control de un servidor Ollama del usuario.
    let local_manager = crate::managers::local_llm::global();
    let uses_managed_sidecar = provider.id == crate::settings::LOCAL_LLM_PROVIDER_ID
        && local_manager.is_some_and(|manager| manager.owns_base_url(base_url));
    let _local_request_lease = uses_managed_sidecar
        .then(|| local_manager.map(|manager| manager.begin_request()))
        .flatten();

    debug!("Sending chat completion request to: {}", url);

    let client = create_client(
        provider,
        &api_key,
        request_timeout.unwrap_or(REQUEST_TIMEOUT),
    )?;

    // Build messages vector
    let mut messages = Vec::new();

    // Add system prompt if provided
    if let Some(system) = system_prompt {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: system,
        });
    }

    // Add user message
    messages.push(ChatMessage {
        role: "user".to_string(),
        content: user_content,
    });

    // Build response_format if schema is provided
    let response_format = json_schema.map(|schema| ResponseFormat {
        format_type: "json_schema".to_string(),
        json_schema: JsonSchema {
            name: "transcription_output".to_string(),
            strict: true,
            schema,
        },
    });

    let request_body = ChatCompletionRequest {
        model: model.to_string(),
        messages,
        temperature,
        response_format,
        reasoning_effort,
        reasoning,
    };

    let response = match client.post(&url).json(&request_body).send().await {
        Ok(response) => response,
        // Si el proceso local murió por presión de memoria o por la carrera
        // entre ensure_running y el inicio de la petición, se reinicia y se
        // reintenta UNA vez. Un timeout no se repite: podría duplicar cinco
        // minutos de cómputo con un servidor que sí sigue trabajando.
        Err(first_error)
            if should_retry_managed_request(uses_managed_sidecar, first_error.is_timeout()) =>
        {
            warn!(
                "Petición al motor local interrumpida; reiniciando y reintentando una vez: {}",
                first_error
            );
            let manager = crate::managers::local_llm::global()
                .ok_or_else(|| "Motor local no inicializado para reintento".to_string())?;
            let retry_base = manager.recover_after_request_failure(model).await?;
            let retry_url = format!("{}/chat/completions", retry_base.trim_end_matches('/'));
            client
                .post(&retry_url)
                .json(&request_body)
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() {
                        LlmError::Timeout.to_string()
                    } else {
                        LlmError::Other(format!("HTTP request failed after local retry: {}", e))
                            .to_string()
                    }
                })?
        }
        Err(error) => {
            return Err(if error.is_timeout() {
                LlmError::Timeout.to_string()
            } else {
                LlmError::Other(format!("HTTP request failed: {}", error)).to_string()
            });
        }
    };

    let status = response.status();

    // 429 se distingue del resto: no es un fallo del que reintentar de
    // inmediato tenga sentido, y quien llama abre el cortacircuitos con él.
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = parse_retry_after(response.headers());
        warn!(
            "Proveedor '{}' devolvió 429 (Retry-After: {:?})",
            provider.id, retry_after
        );
        return Err(LlmError::RateLimited { retry_after }.to_string());
    }

    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read error response".to_string());
        // El cuerpo del proveedor puede traer eco de la petición (y con ella la
        // clave en algunos gateways), así que se acota antes de propagarlo.
        let error_text: String = error_text.chars().take(300).collect();
        return Err(LlmError::Other(format!(
            "API request failed with status {}: {}",
            status, error_text
        ))
        .to_string());
    }

    let completion: ChatCompletionResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    Ok(completion
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .map(|texto| quitar_razonamiento(&texto))
        // Si tras quitar el razonamiento no queda nada, el modelo penso y no
        // respondio. None degrada al dictado crudo, que es mejor que pegar
        // una cadena vacia encima de lo que dijo el usuario.
        .filter(|texto| !texto.is_empty()))
}

/// Quita el razonamiento en voz alta que emiten los modelos "thinking".
///
/// Escriba manda `reasoning_effort` para apagarlo, pero eso solo lo honran
/// algunos servidores: por la ruta de respaldo de Ollama, con un modelo
/// thinking, el monologo llegaba entero al texto del usuario. Este es el unico
/// punto por donde pasa toda respuesta del modelo, asi que saneando aqui
/// quedan cubiertos post-proceso, traduccion, Sesiones, Plumin e Interprete.
fn quitar_razonamiento(texto: &str) -> String {
    const ETIQUETAS: [(&str, &str); 2] = [("<think>", "</think>"), ("<thinking>", "</thinking>")];
    let mut out = texto.to_string();
    for (abre, cierra) in ETIQUETAS {
        while let Some(i) = out.find(abre) {
            match out[i..].find(cierra) {
                Some(rel) => {
                    let fin = i + rel + cierra.len();
                    out.replace_range(i..fin, "");
                }
                // Bloque sin cerrar: se quedo pensando y nunca respondio.
                // Cortar desde la apertura evita pegar el monologo entero.
                None => {
                    out.truncate(i);
                    break;
                }
            }
        }
        // Cierre huerfano: hay plantillas que abren el bloque en el prompt, asi
        // que la respuesta trae solo el cierre y todo lo anterior es rumiar.
        if let Some(i) = out.find(cierra) {
            out = out[i + cierra.len()..].to_string();
        }
    }
    out.trim().to_string()
}

/// Fetch available models from an OpenAI-compatible API
/// Returns a list of model IDs
pub async fn fetch_models(
    provider: &PostProcessProvider,
    api_key: String,
) -> Result<Vec<String>, String> {
    let base_url = provider.base_url.trim_end_matches('/');
    let url = format!("{}/models", base_url);

    debug!("Fetching models from: {}", url);

    let client = create_client(provider, &api_key, REQUEST_TIMEOUT)?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch models: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!(
            "Model list request failed ({}): {}",
            status, error_text
        ));
    }

    let parsed: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let mut models = Vec::new();

    // Handle OpenAI format: { data: [ { id: "..." }, ... ] }
    if let Some(data) = parsed.get("data").and_then(|d| d.as_array()) {
        for entry in data {
            if let Some(id) = entry.get("id").and_then(|i| i.as_str()) {
                models.push(id.to_string());
            } else if let Some(name) = entry.get("name").and_then(|n| n.as_str()) {
                models.push(name.to_string());
            }
        }
    }
    // Handle array format: [ "model1", "model2", ... ]
    else if let Some(array) = parsed.as_array() {
        for entry in array {
            if let Some(model) = entry.as_str() {
                models.push(model.to_string());
            }
        }
    }

    Ok(models)
}

#[cfg(test)]
mod tests {

    #[test]
    fn quita_el_razonamiento_de_los_modelos_thinking() {
        // Bloque completo: se va entero y queda la respuesta.
        assert_eq!(
            super::quitar_razonamiento("<think>El usuario quiere X</think>Hola, que tal."),
            "Hola, que tal."
        );
        // Varios bloques.
        assert_eq!(
            super::quitar_razonamiento("<think>a</think>Uno <think>b</think>dos"),
            "Uno dos"
        );
        // Cierre huerfano: la plantilla abrio el bloque en el prompt.
        assert_eq!(
            super::quitar_razonamiento("mmm a ver</think>La respuesta"),
            "La respuesta"
        );
        // Sin cerrar: penso y nunca respondio, no se pega el monologo.
        assert_eq!(
            super::quitar_razonamiento("Texto util<think>y sigo pensando"),
            "Texto util"
        );
        // La variante <thinking> tambien.
        assert_eq!(
            super::quitar_razonamiento("<thinking>x</thinking>  Listo "),
            "Listo"
        );
        // Texto normal no se toca.
        assert_eq!(
            super::quitar_razonamiento("Un texto cualquiera, sin etiquetas."),
            "Un texto cualquiera, sin etiquetas."
        );
    }

    use super::should_retry_managed_request;

    #[test]
    fn only_non_timeout_local_failures_are_retried() {
        assert!(should_retry_managed_request(true, false));
        assert!(!should_retry_managed_request(true, true));
        assert!(!should_retry_managed_request(false, false));
    }
}
