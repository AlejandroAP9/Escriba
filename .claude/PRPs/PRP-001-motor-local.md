# PRP-001: Motor LLM local zero-install (sidecar llama-server)

> **Estado**: APROBADO (plan maestro validado 7-jul; ejecutar tras gate F2)
> **Fecha**: 2026-07-07
> **Proyecto**: Escriba

## Objetivo

El post-procesado con IA funciona sin API key y sin instalar nada a mano: la app
descarga runtime + modelo con un botón y corrige dictados 100% offline.

## Por Qué

| Problema | Solución |
|----------|----------|
| El phraser de Handy exige API key de pago u Ollama configurado por terminal | La app gestiona su propio LLM local (descarga, arranque, apagado) |
| Las demás duplas harán BYOK (receta del enunciado) | "Cero API keys, cero nube" = diferenciador estructural del concurso |

**Valor para el concurso/comunidad:** habilita TODOS los poderes y capacidades
(Prompt Maestro, dictado natural, traducción, voice edit) gratis para 2.500+
miembros. Es la prueba reina del video: corrección con IA con el wifi apagado.

## Qué

### Criterios de Éxito
- [ ] Botón "Descargar y activar" → runtime (~pocos MB) + Qwen3-4B Q4_K_M (~2.5GB) con progreso, SHA256 verificado.
- [ ] Dictado con post-proceso funciona **con wifi apagado**.
- [ ] LLM se descarga de RAM a los 2 min de inactividad (setting propio).
- [ ] `kill -9` a Escriba → cero procesos `llama-server` vivos; reapertura detecta/limpia huérfanos.
- [ ] Cascada verificada: sidecar caído → Ollama si existe → Apple Intelligence → texto crudo + toast. BYOK jamás automático.
- [ ] RAM ≤8GB detectada → sugiere Qwen3-1.7B.

### Comportamiento Esperado
Usuario activa el phraser en settings → un botón, una barra de progreso → dicta
con el atajo de post-proceso → texto corregido pegado. Primer uso tras arranque:
1-3s extra (carga del modelo); siguientes: inmediato.

## Contexto

### Referencias
- `src-tauri/src/actions.rs:167` — rama Apple Intelligence: plantilla exacta de provider no-HTTP.
- `src-tauri/src/managers/model.rs:1826` — `download_model` (progreso/SHA256/cancelación, reusar tal cual).
- `src-tauri/src/managers/transcription.rs:159,290,385` — patrón residencia + watcher unload (replicar).
- `src-tauri/src/settings.rs:558,637,691` — providers, custom/Ollama, merge idempotente.
- `docs/plan/02-tech-spec.md` §1-3 — diseño completo; `docs/plan/07-security-plan.md` superficies 1-2.
- llama.cpp releases (binarios llama-server por plataforma) — pinnear tag exacto + SHA256.

### Arquitectura Propuesta
Nuevo `managers/local_llm.rs` (`LocalLlmManager`: child/port/last_used;
`ensure_running() -> base_url`, `shutdown()`), registro en `lib.rs`, kill en
`RunEvent::Exit`. Provider `local_llm` sentinela `local-llm://managed` con
`supports_structured_output: true`. Catálogo: campo `model_kind: ModelKind{Stt|Llm}`
(NO un EngineType nuevo), filtro del picker STT, comando `get_available_llm_models`.
Frontend: estado en `PostProcessingSettings` (descargando/listo/corriendo) + i18n.

### Modelo de Datos
Settings nuevos con default + merge: `llm_unload_timeout` (default Min2),
`post_process_models["local_llm"]` = id GGUF. Sin migración de DB.

## Premortem

Blindajes previos consultados: conflicto ggml (NO co-linkear llama.cpp:
transcribe-cpp-sys vendoriza ggml estático → símbolos duplicados + doble backend
Metal; por eso sidecar). Fix build.rs CLT (FoundationModels sin plugin de macros).

| Amenaza | Cómo la mata el diseño | Cómo se verifica |
|---|---|---|
| Zip runtime alterado (MITM) | tag pinneado + SHA256 hardcodeado, verificar antes de extraer | alterar 1 byte → rechazo |
| zip malicioso con `../` | sanitizar rutas al extraer | zip con traversal → error |
| Server expuesto a LAN | `--host 127.0.0.1` + puerto efímero, nunca 0.0.0.0 | curl desde otra máquina → sin conexión |
| Huérfano tras crash | PID file + limpieza al arrancar + kill en Exit | kill -9 → ps limpio |
| OOM en Mac 8GB (Whisper+LLM residentes) | unload LLM 2 min + sugerir 1.7B vía sysinfo | sesión continua monitoreada en 8GB |
| Cascada degrada a cloud sin permiso | BYOK excluido por diseño de `resolve_post_process_route` | matar sidecar y Ollama sin Apple Intelligence → texto crudo + toast, cero tráfico |
| Windows sin Vulkan | reintento con zip CPU / `--device none` | VM Windows sin GPU |

## Blueprint

### Fase 1: Spike (GATE go/no-go, viernes 11-jul)
**Objetivo:** llama-server + Qwen3-4B respondiendo chat completion desde Rust en macOS y Windows.
**Validación:** completion correcta vía `llm_client` a localhost. NO-GO → Plan B.

### Fase 2: LocalLlmManager
**Objetivo:** spawn/health/kill/idle-unload/PID robustos, registrado en lib.rs.
**Validación:** criterios de huérfanos y unload.

### Fase 3: Descarga del runtime + catálogo Llm
**Objetivo:** runtime y GGUF descargables desde la UI con progreso; picker STT filtrado.
**Validación:** descarga limpia + SHA256; GGUF no aparece en selector de transcripción.

### Fase 4: Provider + cascada
**Objetivo:** `local_llm` como provider default; `resolve_post_process_route` completo.
**Validación:** matriz de cascada (4 escenarios) + wifi apagado.

### Fase 5: UI + hardening
**Objetivo:** botón único de activación con estados; Windows CPU-fallback; 8GB.
**Validación:** Premortem completo re-verificado + criterios de éxito + `raiz blindar` de clases confirmadas.

### Plan B (si Fase 1 = NO-GO): Ollama-first (~2 días)
Provider `ollama` dedicado (clon de custom) · detección (`/v1/models`, 500ms) ·
botón "Instalar Ollama" (abre ollama.com/download) + poll · pull vía
`POST /api/pull {"model":"qwen3:4b"}` con stream de progreso en la UI.
Criterios de éxito idénticos salvo "un doble clic externo permitido".

## Aprendizajes (Self-Annealing)

### 2026-07-06: Apple Intelligence no compila con Command Line Tools
- **Error**: swiftc falla: `FoundationModelsMacros` plugin no existe en CLT aunque el SDK trae el framework.
- **Fix**: build.rs verifica el plugin además del framework; fallback a stub.
- **Aplicar en**: cualquier build macOS de este repo; candidato a PR upstream; blindaje.

## Gotchas

- [ ] `bindings.ts` es autogenerado (tauri-specta): agregar comandos en Rust y regenerar.
- [ ] El campo `model` es libre por provider (Apple Intelligence lo usa como token_limit): documentar que en `local_llm` es el id del GGUF.
- [ ] `reasoning_effort: none` como en provider custom (actions.rs:148).
- [ ] Settings nuevos SIEMPRE con default + merge (usuarios de builds previas).

## Anti-Patrones

- NO llama-cpp-2 ni ningún crate que linkee ggml.
- NO 0.0.0.0, NO puerto fijo, NO webui.
- NO degradar a BYOK automáticamente.
- NO loggear contenido de transcripciones.
