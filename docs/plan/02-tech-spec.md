# Tech Spec — Escriba

> Adaptación del paso 02 del pipeline Raíz: aquí no hay tablas+RLS; la
> arquitectura es managers de Rust + comandos Tauri + settings versionados.
> Base: auditoría completa del repo (2 agentes) + diseño de arquitecto validado.

## Arquitectura heredada (lo que Handy ya trae y NO se toca)

```
Audio (cpal) → VAD (Silero) → STT local (transcribe-cpp Whisper GGUF / transcribe-rs ONNX)
     → texto → [post-proceso LLM opcional] → paste (enigo) / clipboard
```

- **Coordinación:** `transcription_coordinator.rs` (hilo único, estados Idle/Recording/Processing). Gate de bindings en línea 41.
- **Managers residentes** con `Arc<Mutex<Option<T>>>` + watcher de unload por timeout: patrón a replicar para el LLM.
- **Post-proceso existente:** `actions.rs:77 post_process_transcription()`; providers en `settings.rs:558`; cliente HTTP OpenAI-compat en `llm_client.rs`; provider no-HTTP de referencia: Apple Intelligence (`apple_intelligence.rs:33`, bifurca en `actions.rs:167`).
- **Catálogo de modelos:** `managers/model.rs` — descarga HF con progreso/SHA256/cancelación, agnóstica al tipo (`download_model:1826`).
- **Frontend:** React 18 + Tailwind 4; theming en 8 CSS vars (`src/styles/theme.css`); sidebar data-driven (`Sidebar.tsx SECTIONS_CONFIG`); bindings tauri-specta autogenerados (`src/bindings.ts`, no editar).
- **Historial:** SQLite (`managers/history.rs`), migraciones rusqlite incrementales.

## Componentes NUEVOS

### 1. Motor local: `managers/local_llm.rs` (LocalLlmManager)

```rust
pub struct LocalLlmManager {
    child: Mutex<Option<std::process::Child>>,
    port: AtomicU16,            // TcpListener a 127.0.0.1:0
    last_used: Mutex<Instant>,  // watcher, timeout independiente (default 2 min)
}
// ensure_running(model_path) -> Result<String>  // "http://127.0.0.1:{p}/v1"
// shutdown() en RunEvent::Exit + Drop; PID file contra huérfanos
```

- Runtime: zip de `llama-server` de release PINNEADO de llama.cpp (SHA256), extraído a `app_data/llm-runtime/`. Args: `-m model.gguf --host 127.0.0.1 --port {p} -c 4096 --no-webui --jinja`. Windows: `CREATE_NO_WINDOW`; fallback Vulkan→CPU.
- **Prohibido** embeber llama.cpp in-process: `transcribe-cpp-sys` vendoriza ggml estático → símbolos duplicados + doble backend Metal (blindaje #2).
- GGUF default: **Qwen3-4B-Instruct-2507 Q4_K_M** (~2.5GB, Apache-2.0); ligero Qwen3-1.7B (~1.1GB) si RAM ≤8GB (via `sysinfo`).

### 2. Catálogo extendido: `ModelKind { Stt | Llm }` en `ModelDescriptor`

- Filtrar picker STT por kind; nuevo comando `get_available_llm_models`.
- `download_model` no cambia (ya agnóstico).

### 3. Provider `local_llm` + cascada

- Entrada en `default_post_process_providers()` con base_url sentinela `local-llm://managed`, `supports_structured_output: true` (llama-server soporta json_schema por gramática). Merge automático vía `ensure_post_process_defaults()`.
- Rama en `post_process_transcription` espejo de Apple Intelligence: resolver GGUF → `ensure_running()` → path HTTP existente con api_key vacía.
- `resolve_post_process_route()`: local_llm → Ollama (`GET 127.0.0.1:11434/v1/models`, timeout 500ms) → Apple Intelligence → crudo + toast. **BYOK nunca entra solo.**

### 4. Modos de acción: `PostProcessMode { None, Prompt, Translate, Edit }`

- `TranscribeAction { mode }` reemplaza el bool; propagar por coordinator (incluir ids nuevos en el gate :41).
- Bindings nuevos en `ACTION_MAP` (:868) + defaults (settings.rs ~758) + merge para instalaciones previas: `transcribe_translate` (alt+shift+t), `voice_edit` (alt+shift+e).
- `Translate`: plantilla constante en Rust + setting `translation_target_language` (ISO, default "en"); fuerza `translate=false` en el paso STT para no traducir dos veces.
- `Edit` (Capacidad A): Cmd/Ctrl+C sintético (enigo) con save/restore de clipboard → selección + instrucción dictada → LLM → paste (editar) o overlay panel (preguntar, reutiliza `streamTextEvent`). Fallback sin selección: actúa sobre el último dictado.

### 5. Poderes de prompt (capa de datos, no de infraestructura)

- Presets seed en `default_post_process_prompts()` (:675) + merge idempotente por id: Dictado natural, Prompt Maestro (plantillas por destino), WhatsApp, Email profesional, Lista/bullets, Traducción (interna).
- Tonos por app: `app_context_rules: Vec<{pattern, prompt_id}>` en settings; app frontmost vía NSWorkspace (macOS) / GetForegroundWindow (Windows); lookup antes de post-proceso.
- Diccionario: inyectar `custom_words` existentes al prompt de corrección.

### 6. Capacidad mayor 🤝 (una de dos)

- **D Estudio:** decodificación `symphonia` (mp3/m4a/mp4/flac/ogg/wav) → resample (rubato, ya está) → STT batch con timestamps (ya soportados) → serializadores SRT/VTT/TXT/JSON → resumen vía phraser. UI drag&drop + cola con progreso.
- **B Intérprete:** servidor HTTP embebido (axum o tiny-http) bind 127.0.0.1+LAN opcional, página estática + SSE, fan-out de traducción por idioma por segmento VAD, QR con IP local.

### 7. UX quirúrgica

- Des-enterrar post-proceso: quitar doble gate experimental (`Sidebar.tsx:63`, `AdvancedSettings.tsx`), sección visible siempre, toggle dentro; migrar claves i18n fuera de "debug".
- Top picks por locale: helper frontend `rankModelsForLocale` (si UI ≠ en → Whisper Large-v3-Turbo #1; Parakeet con badge "solo inglés").
- Catálogo localizado: llenar `onboarding.models.<id>.*` en `es/translation.json` (mecanismo existe, `modelTranslation.ts`); fix buscador `ModelsSettings.tsx:177` (filtrar sobre texto traducido).
- Stats: `get_usage_stats` (SQL sobre historial existente, sin migración); búsqueda: `LIKE` sobre transcription_text/post_processed_text + chip "guardados".

## Variables de entorno / secretos

- App: NINGUNO en runtime (principio local-first). API keys BYOK opcionales viven en `SecretMap` (ya se redactan al serializar).
- CI: `TAURI_SIGNING_PRIVATE_KEY(_PASSWORD)` (keypair minisign NUESTRA, pendiente `tauri signer generate`).

## Distribución

- Repo propio (remote `upstream` → cjpais/handy; tag `upstream-base` = dad37ba congelado).
- CI GitHub Actions: macOS ad-hoc (`signingIdentity: "-"`), Windows NSIS sin firma. Updater endpoint: `github.com/AlejandroAP9/Escriba/releases/latest/download/latest.json` (pubkey PENDIENTE de regenerar; el endpoint actual 404 evita updates accidentales a Handy).
- Estado rebrand R1: HECHO (productName Escriba, identifier com.escriba.app, Cargo/package/título/logs/CLI/headers). R2 (21 locales, SVGs, purga #FAA2CA, overlay vars) post feature-freeze.
