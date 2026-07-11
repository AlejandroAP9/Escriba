# PRP-005: Intérprete en vivo (Capacidad B, desktop)

> **Estado**: APROBADO (decisión de dupla 11-jul: dentro de Escriba escritorio)
> **Fecha**: 2026-07-11
> **Proyecto**: Escriba

## Objetivo

El guía habla en su idioma; el Mac/laptop transcribe y traduce en vivo con el
motor local, levanta una sala con código + QR, y cada asistente abre una web
en su teléfono (misma red / hotspot) para leer y ESCUCHAR la traducción en SU
idioma. 100% local, sin nube.

## Por Qué

| Problema                                                                                                | Solución                                                        |
| ------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- |
| Traducción simultánea para grupos cuesta miles por evento (Wordly) o depende de la nube (Turavi/Gemini) | Escriba lo hace gratis y 100% local sobre el motor que ya corre |

**Valor concurso:** el clímax de video (3 teléfonos, 3 idiomas, tú hablando
español). Ninguna app de dictado lo tiene. Turavi lo hizo con Gemini cloud →
se cae sin internet; el nuestro no.

**Alcance:** el guía usa Escriba escritorio (laptop en bus/sala/mochila). El
"guía con solo un teléfono" = producto PWA post-hackathon (visión de landing).

## Qué

### Criterios de Éxito

- [ ] Guía inicia sala → código de 4 dígitos + QR con la URL de la LAN.
- [ ] 2+ teléfonos en la misma red se unen por navegador (sin instalar nada).
- [ ] Cada teléfono elige su idioma y ve subtítulos en vivo en ese idioma.
- [ ] Voz: cada teléfono reproduce la traducción con TTS del navegador (offline).
- [ ] Latencia < 5s por segmento de voz.
- [ ] Cero nube: verificable con monitor de red (solo tráfico LAN).
- [ ] El servidor solo escucha mientras la sala está activa; se apaga al detener.

### Comportamiento Esperado

Escriba → sección "Intérprete" → el guía elige idioma de origen → "Iniciar
sala" → muestra código + QR + nº de oyentes. Habla → VAD segmenta → Whisper
transcribe → por cada idioma de oyente conectado, el LLM local traduce → se
emite por SSE a los navegadores → subtítulo + TTS. "Detener sala" apaga todo.

## Contexto

### Referencias

- Motor: `transcription.rs` (Whisper), `actions.rs` (traducción vía cascada local).
- Streaming/VAD ya existe (overlay live usa `streamTextEvent`).
- Patrón cola/eventos: `commands/studio.rs`, `managers/local_llm.rs`.

### Arquitectura Propuesta

- `managers/interpreter.rs` (NUEVO): servidor axum embebido en 0.0.0.0:{puerto}
  SOLO mientras la sala está activa; canal `tokio::sync::broadcast` por donde el
  guía publica {texto_origen, timestamp}; cada visitante SSE se suscribe y el
  frontend visitante traduce/pide traducción por idioma.
  - Traducción: el LLM traduce por idioma de oyente en el backend (una vez por
    idioma por segmento, no por oyente) y emite {lang -> texto} por SSE.
- Rutas del servidor:
  - `GET /` → página del visitante (HTML embebido con include_str!).
  - `GET /events?room=XXXX&lang=YY` → SSE con las líneas traducidas a YY.
  - `POST /join` / heartbeat → contador de oyentes + set de idiomas activos.
- Guía UI: sección "Intérprete" en el sidebar (idioma origen, iniciar/detener,
  código, QR, nº oyentes). QR generado en Rust (`qrcode` → SVG) con la URL LAN.
- Deps nuevas: `axum` (server+SSE), `qrcode` (QR→SVG), `local-ip-address` (IP LAN).
- Visitante: HTML/JS puro servido por el backend; TTS con Web Speech API
  (`speechSynthesis`, offline); selector de idioma; auto-reconexión SSE.

## Premortem

| Amenaza                                               | Defensa                                                                                                               | Verificación                                    |
| ----------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| Servidor LAN abierto de más                           | bind solo con sala activa; código de sala obligatorio; solo lectura (SSE), sin inputs del visitante que ejecuten algo | escanear puertos con sala detenida → cerrado    |
| Cualquiera en la red escucha                          | código de 4 dígitos en la URL; sin código → 403                                                                       | entrar sin código → rechazo                     |
| Traducción N idiomas × N oyentes explota              | traducir 1 vez por idioma por segmento (no por oyente); idiomas activos = set                                         | 10 oyentes, 2 idiomas → 2 traducciones/segmento |
| Latencia inaceptable                                  | VAD por frase + traducción local en paralelo por idioma                                                               | medir < 5s                                      |
| TTS pisa subtítulos / se encola                       | cortar TTS al llegar frase nueva (cancel + speak)                                                                     | frases rápidas seguidas                         |
| Contenido a logs                                      | solo conteos/idiomas, nunca el texto hablado                                                                          | grep log                                        |
| RAM: Whisper + LLM simultáneos activos toda la sesión | modo intérprete asume equipo con RAM suficiente; avisar en 8GB                                                        | monitor 8GB                                     |

## Blueprint

### Fase 1: esqueleto de conectividad (SIN audio)

**Objetivo:** servidor embebido + página visitante + código + QR + UI guía; el guía publica un texto de PRUEBA y el visitante lo ve por SSE.
**Validación:** 2do dispositivo en la LAN abre la URL del QR, entra con código, ve el mensaje de prueba en vivo; detener sala cierra el puerto.

### Fase 2: audio → transcripción → broadcast

**Objetivo:** mic del guía → VAD → Whisper → publicar texto origen al canal.
**Validación:** hablar → el visitante ve el texto (en idioma origen) en vivo.

### Fase 3: traducción por idioma + selector visitante

**Objetivo:** traducir por idioma de oyente (LLM local) + el visitante elige idioma.
**Validación:** 2 teléfonos, 2 idiomas distintos, subtítulos correctos.

### Fase 4: voz (TTS navegador) + pulido + i18n

**Objetivo:** TTS offline en el visitante; auto-reconexión; UI guía completa.
**Validación:** criterios de éxito completos; premortem verificado; 3 teléfonos en video.

## Gotchas

- [ ] `speechSynthesis` en iOS Safari requiere un gesto del usuario para arrancar (el botón "Escuchar" cuenta).
- [ ] axum + tokio ya presentes; cuidar que el server viva en el runtime de Tauri (`tauri::async_runtime`).
- [ ] La IP LAN puede cambiar; regenerar QR si cambia de red.
- [ ] `check:translations` 21 locales para los textos de la UI del guía.

## Anti-Patrones

- NO nube · NO bind permanente (solo sala activa) · NO traducir por-oyente ·
  NO loggear el contenido hablado · NO servir sin código de sala.
