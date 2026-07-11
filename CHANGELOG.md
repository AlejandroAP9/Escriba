# Changelog

Todas las novedades notables de **Escriba**. El formato sigue el estilo de
[Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y el versionado es
[SemVer](https://semver.org/lang/es/).

Escriba es un _rework_ de [Handy](https://github.com/cjpais/handy) (© CJ Pais,
MIT). Esta historia arranca en el fork del 7 de julio de 2026 y recoge lo que
la dupla **Alejandro & Flor** construyó encima para los Juegos Imperiales:
convertir una app de dictado en un motor de IA **100% local y gratis**.

## [0.10.0] — 2026-07-11

### Agregado

- **Agentes (MCP):** servidor local (JSON-RPC sobre HTTP, solo `127.0.0.1`) para
  que Claude Code, Cursor o Cline usen la transcripción, traducción y resumen de
  Escriba como herramientas. 100% local.
- **Re-transcribir con otro modelo** en Historial y Estudio: mismo audio, más
  precisión, sin volver a subir nada.
- **Micrófono en los campos** de la propia app: dicta directo en cualquier campo.

## [0.9.0] — 2026-07-11

### Agregado

- **Traductor cara a cara:** conversación 1-a-1 bidireccional con detección
  automática de idioma, pantalla grande y voz.
- Soporte de **lituano** y conservación del historial de sala al cambiar de
  idioma (feedback de Flor y su señora).
- **Copiar texto** en Estudio e Intérprete y selección de texto en toda la app.
- Nombres de idioma en español entre paréntesis en los selectores.

## [0.8.0] — 2026-07-11

### Agregado

- **Intérprete en vivo:** el equipo levanta una sala en la red local y muestra un
  **QR**; cada asistente lo abre en su teléfono y lee los subtítulos **en su
  propio idioma**, traducidos por el motor local (una vez por idioma). Para guías
  turísticos, clases y charlas con extranjeros.
- Micrófono en vivo del guía (dictado → sala), selector de idioma de origen,
  cambio de idioma en vivo desde el teléfono sin reiniciar y mejor voz TTS.

## [0.7.0] — 2026-07-09

### Agregado

- **Onboarding es-first:** recomendaciones de modelo por idioma y catálogo en
  español (antes recomendaba un modelo solo-inglés a todos).
- **Estadísticas de uso** (palabras, racha, tiempo ahorrado).

## [0.6.0] — 2026-07-09

### Agregado

- **Estudio de transcripción:** arrastra un audio o video → transcripción con
  marcas de tiempo → exporta **SRT / VTT / TXT / JSON** + **resumen con IA**.
- Soporte de audios **`.opus` de WhatsApp** (agregado el 2026-07-10).

## [0.5.0] — 2026-07-08

### Agregado

- **Háblale a cualquier texto:** selecciona texto en cualquier app, mantén el
  atajo y dicta la instrucción (_"hazlo más formal"_, _"resúmelo en 3 líneas"_,
  _"tradúcelo al portugués"_); el texto se reemplaza en el lugar.

## [0.4.0] — 2026-07-08

### Agregado

- **Poderes de dictado:** dictado natural (limpia muletillas y repeticiones),
  presets de comunidad y diccionario personal conectado a la corrección.
- **Traducción al dictar:** hablas en un idioma y el texto se pega en otro.

### Cambiado

- Post-proceso con IA des-enterrado: pasa de ser una función experimental
  escondida a una capacidad principal de la app.

## [0.3.0] — 2026-07-07

### Agregado

- **Motor de IA local:** sidecar de LLM local incluido, con cascada de respaldo
  (local → Ollama → Apple Intelligence → texto crudo). Sin claves de API en el
  camino feliz.

### Cambiado

- **Rebrand completo de Handy a Escriba:** identidad, marca y wordmark propios.

---

Las versiones 0.1–0.2 corresponden a la base original de
[Handy](https://github.com/cjpais/handy); la historia propia de Escriba empieza
en 0.3.0.
