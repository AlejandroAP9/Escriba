# PRP-004: Estudio de transcripción (Capacidad D)

> **Estado**: APROBADO (decisión de dupla 8-jul: D primero, B stretch)
> **Fecha**: 2026-07-09
> **Proyecto**: Escriba

## Objetivo

Arrastras un archivo de audio o video → transcripción completa local con
timestamps → export SRT/VTT/TXT/JSON + resumen con el motor de IA local.

## Por Qué

| Problema                                                                                   | Solución                                                   |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| Subtitular Reels, transcribir clases o audios largos cuesta $99-750/año (WhisperAI y cía.) | La categoría completa, gratis y offline, dentro de Escriba |

**Valor concurso:** completa el pitch "3 productos pagos en 1 app gratis".
La votación comunitaria (avance 2) ordena los casos de uso de lanzamiento.

## Qué

### Criterios de Éxito

- [ ] mp3, wav, m4a, flac y mp4 (pista de audio) → transcripción correcta.
- [ ] Audio de 30+ min sin cortes ni duplicados en las junturas de chunks.
- [ ] SRT válido importable en CapCut/YouTube; VTT/TXT/JSON correctos.
- [ ] Resumen con IA local (wifi apagado) sobre la transcripción.
- [ ] Cola con progreso por archivo; la UI nunca se congela.
- [ ] Audio del usuario jamás sale del equipo ni va a logs.

### Comportamiento Esperado

Nueva sección "Estudio" en el sidebar: zona drag&drop → cada archivo entra a
una cola (pending/processing/done/error + %) → al terminar: vista de la
transcripción con párrafos y timestamps + botones Exportar (SRT/VTT/TXT/JSON)
y "Resumir con IA" → archivos generados junto al original o vía diálogo save.

## Contexto

### Referencias

- `transcription.rs:1252`: `session.run(&audio, &run_options).map(|t| t.text)`
  DESCARTA los segmentos → el Estudio necesita un run propio que conserve
  `t.segments` (verificar API exacta de transcribe-cpp: segments con start/end).
- `audio_toolkit/audio/resampler.rs` (rubato): reusar para 16kHz mono.
- Patrón de cola + eventos de progreso: `commands/local_llm.rs` (setup) y
  `managers/model.rs` (descargas).
- Resumen: pipeline del phraser existente (cascada local, temperatura 0.2).
- **Algoritmos adoptados (análisis 9-jul de repos externos):**
  - MediaTranscribe (MIT, mención en acknowledgments): chunking por tamaño
    con overlap 5s; **dedup de solape** al reensamblar (descartar segmentos
    cuyo end ≤ max_end_so_far + tolerancia 0.5s); párrafos por gap >3s de
    silencio o >45s acumulados; heurística de calidad (~400 chars/min mínimo
    esperado → aviso de transcripción sospechosa).
  - transcriptor (sin licencia, SOLO inspiración): conversión previa a WAV
    16kHz mono; ffprobe para duración; cola con estados.

### Arquitectura Propuesta

- `src-tauri/src/managers/studio.rs` (NUEVO): cola (Mutex<VecDeque<Job>>),
  worker thread único, eventos `studio-progress` {job_id, stage, pct}.
- `src-tauri/src/studio/decode.rs` (NUEVO): **symphonia** (dep nueva, pure
  Rust: mp3/aac-m4a/mp4/flac/wav/ogg-vorbis) → f32 mono 16kHz (rubato).
  Opus/WhatsApp v2: fallback a ffmpeg del sistema si existe, si no, mensaje
  claro de formato no soportado.
- `src-tauri/src/studio/segments.rs` (NUEVO): tipos {start_s, end_s, text},
  offset por chunk, dedup de solape, agrupado en párrafos, heurística calidad.
- `src-tauri/src/studio/export.rs` (NUEVO): serializadores SRT/VTT/TXT/JSON
  puros con tests unitarios (timestamps HH:MM:SS,mmm / HH:MM:SS.mmm).
- Chunking: ventanas de ~8 min con 5s de overlap (el contexto de whisper.cpp
  maneja 30s internos; el chunk grande es por RAM/progreso, no por el modelo).
- Comandos: `studio_enqueue(paths)`, `studio_status()`, `studio_export(job, format)`,
  `studio_summarize(job)`.
- UI: `components/studio/` + sección sidebar; drag&drop vía evento
  tauri://file-drop; i18n 21 locales.

## Premortem

| Amenaza                            | Defensa                                                                                    | Verificación                                       |
| ---------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------- |
| Duplicados/huecos en junturas      | overlap 5s + dedup por max_end + tolerancia 0.5s                                           | audio 30 min: diff contra transcripción sin chunks |
| OOM con archivos enormes           | decode + resample en streaming por chunk, nunca el archivo entero en RAM                   | video 1GB en Mac 8GB                               |
| UI congelada                       | worker thread + eventos; jamás transcribir en el hilo del comando                          | arrastrar 5 archivos seguidos                      |
| Formato exótico rompe el decode    | match de extensiones soportadas + error legible; ffmpeg del sistema como fallback opcional | .opus sin ffmpeg → mensaje claro                   |
| SRT inválido en editores           | tests unitarios de formato + validar en CapCut/YouTube reales                              | criterio de éxito                                  |
| Contenido a logs                   | solo duraciones/conteos, nunca texto                                                       | grep del log                                       |
| Choque de RAM con el LLM residente | transcripción batch descarga el LLM antes (unload) y el resumen lo recarga después         | monitor en 8GB                                     |

## Blueprint

### Fase 1: cimientos puros (export + segmentos con tests)

**Objetivo:** `studio/export.rs` + `studio/segments.rs` con tests unitarios verdes (SRT/VTT/dedup/párrafos sin tocar audio).

### Fase 2: decode + transcripción con timestamps

**Objetivo:** symphonia integrado; run propio que conserva segments; mp3 de 10 min → SRT real.

### Fase 3: cola + comandos + eventos

**Objetivo:** encolar N archivos con progreso; export a disco.

### Fase 4: UI Estudio + resumen IA + i18n

**Objetivo:** drag&drop end-to-end + "Resumir con IA" vía phraser.

### Fase 5: validación final

**Objetivo:** criterios de éxito + premortem verificados; SRT probado en CapCut/YouTube.

## Gotchas

- [x] RESUELTO (9-jul): transcribe-cpp `Transcript.segments: Vec<Segment>` con `t0_ms/t1_ms: i64` y `text: String` → nuestro Segment = t0_ms/1000.0. El pipeline del dictado los descarta con `.map(|t| t.text)`; el Estudio usa `session.run()` directo conservandolos.
- [x] RESUELTO (9-jul): ya existe carga de archivos en el modo headless `--transcribe-file` (lib.rs:333, `read_wav_samples` via hound) pero SOLO WAV 16-bit → symphonia cubre el resto; el patron headless sirve de referencia para el flujo batch sin mic/VAD.
- [ ] `bindings.ts` autogenerado; `check:translations` exige 21 locales.
- [ ] symphonia: features por formato en Cargo.toml (mp3, aac, isomp4, flac, vorbis).
- [ ] Whisper alucina en silencios largos: pasar VAD o descartar segmentos vacíos/repetidos (patrón "Gracias por ver el video").

## Anti-Patrones

- NO ffmpeg obligatorio (solo fallback opcional del sistema) · NO cloud · NO
  cargar archivos completos en RAM · NO strings en JSX sin i18n.

## Aprendizajes (Self-Annealing)

### 2026-07-09: "Model is not loaded for transcription" en el Estudio

- **Error**: la UI del Estudio daba ese error en cada archivo. `transcribe_segments` asumía el modelo cargado, pero el Estudio no pasa por el flujo de dictado (que lo carga on-demand). El modo headless sí funcionaba porque llamaba `load_model_with_device` explícito antes.
- **Fix**: `transcribe_segments` carga `settings.selected_model` si `lock_engine().is_none()` (mismo patrón que `transcribe()`).
- **Aplicar en**: cualquier nuevo consumidor del TranscriptionManager fuera del flujo de dictado/CLI: cargar el modelo primero, no asumirlo residente.
