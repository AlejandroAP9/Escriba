# PRP-003: "Háblale a cualquier texto" (edición por voz universal)

> **Estado**: APROBADO (Fase 5 del Blueprint; dupla dio go 8-jul)
> **Fecha**: 2026-07-08
> **Proyecto**: Escriba

## Objetivo

Seleccionas texto en CUALQUIER app, mantienes un atajo, dictas una instrucción
("hazlo más formal", "resúmelo en 3 líneas", "tradúcelo al portugués") y el
texto seleccionado se reemplaza por el resultado, procesado por el motor local.

## Por Qué

| Problema                                                         | Solución                                           |
| ---------------------------------------------------------------- | -------------------------------------------------- |
| Editar texto existente exige reescribir o copiar a un chat de IA | Lo editas hablando, donde está, sin cambiar de app |

**Valor para el concurso:** Typeless cobra $12-30/mes por esto; seríamos la
primera versión open source 100% local. Demo estrella del video.

## Qué

### Criterios de Éxito

- [ ] Funciona en Notas, WhatsApp Web (navegador) y VS Code.
- [ ] Clipboard del usuario restaurado SIEMPRE (incluso si el LLM falla).
- [ ] Sin selección → aviso claro y NO se toca nada (v1; fallback a último dictado queda post-hackathon).
- [ ] Con wifi apagado (motor local).
- [ ] Instrucción y texto nunca se loggean.

### Comportamiento Esperado

Mantener atajo `voice_edit` (default alt+shift+e) → la app copia la selección
(Cmd/Ctrl+C sintético vía enigo, guardando el clipboard previo) → grabas la
instrucción → STT → prompt: instrucción + texto seleccionado → LLM local →
resultado reemplaza la selección (paste) → clipboard original restaurado.

## Contexto

### Referencias

- `actions.rs`: `TranscribeMode` ya es enum (agregar `Edit`); ACTION_MAP :~944.
- `transcription_coordinator.rs:41`: gate de bindings (agregar id).
- `shortcut/mod.rs|tauri_impl.rs|handy_keys.rs`: gates post_process_enabled (mismo patrón que translate).
- `clipboard.rs` + enigo: copia/pega sintético existente; `paste_delay_ms` setting.
- `settings.rs`: binding default + merge automático ya probados con translate.
- Cascada local + temperatura 0.2: reutilizar tal cual.

### Arquitectura Propuesta

1. Binding `voice_edit` + `TranscribeMode::Edit`.
2. En `start()` del modo Edit: capturar selección ANTES de grabar (Cmd+C sintético + read clipboard + restaurar), guardarla en el action state.
3. En el pipeline: prompt constante "Aplica esta instrucción al texto. Responde SOLO con el texto resultante.\nInstrucción: {dictado}\nTexto:\n{selección}".
4. Sin selección (clipboard sin cambio tras Cmd+C): usar última entrada del historial como texto objetivo.
5. Paste del resultado (flujo normal); restaurar clipboard con guard.

## Premortem

| Amenaza                              | Defensa                                                          | Verificación                                              |
| ------------------------------------ | ---------------------------------------------------------------- | --------------------------------------------------------- |
| Clipboard del usuario perdido        | guard de restauración en TODOS los paths (éxito/error/cancel)    | copiar algo → editar → error inducido → clipboard intacto |
| App bloquea Cmd+C sintético          | detectar clipboard sin cambios → fallback último dictado + toast | probar en app que bloquee                                 |
| Selección con datos sensibles a logs | jamás loggear contenido (solo longitudes)                        | grep del log tras sesión                                  |
| Instrucción vacía/ruido              | si STT vacío → no tocar nada, avisar                             | dictar silencio                                           |
| Texto enorme (>contexto 4096)        | truncar con aviso o rechazar >N chars                            | seleccionar 50k chars                                     |

## Blueprint

### Fase 1: binding + modo Edit + captura de selección

**Validación:** log muestra selección capturada (longitud) y clipboard restaurado.

### Fase 2: pipeline de edición (prompt + LLM + paste)

**Validación:** "hazlo más formal" en Notas funciona end-to-end offline.

### Fase 3: fallbacks (sin selección → último dictado) + hardening + UI (ShortcutInput en Post-Proceso) + i18n 21 locales

**Validación:** criterios de éxito completos + premortem verificado.

## Gotchas

- [ ] `bindings.ts` autogenerado; `check:translations` exige 21 locales.
- [ ] El gate del coordinator (:41) y los 3 gates de registro DEBEN incluir el id nuevo (aprendizaje de translate).
- [ ] Delay tras Cmd+C sintético antes de leer clipboard (~150ms) o se lee el valor viejo.

## Anti-Patrones

- NO nube, NO loggear contenido, NO tocar el texto si algo falla (mejor no hacer nada que romper lo seleccionado).
