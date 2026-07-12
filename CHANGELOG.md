# Changelog

Todas las novedades notables de **Escriba**. El formato sigue el estilo de
[Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y el versionado es
[SemVer](https://semver.org/lang/es/).

Escriba es un _rework_ de [Handy](https://github.com/cjpais/handy) (© CJ Pais,
MIT). Esta historia arranca en el fork del 7 de julio de 2026 y recoge lo que
la dupla **Alejandro & Flor** construyó encima para los Juegos Imperiales:
convertir una app de dictado en un motor de IA **100% local y gratis**.

## [1.2.0] — 2026-07-12

Rework completo de la interfaz y nacimiento del **sistema de diseño de Escriba**.
Se recorrieron las once pantallas una por una y, al terminar, se destiló todo en
un sistema reutilizable para que Escriba se sienta coherente incluso cuando sume
nuevas funciones.

### Diseño

- **Rework de las 11 pantallas:** Inicio, Modelos, Historial, Estudio, Traductor,
  Intérprete en vivo, Agentes (MCP), Escritura Inteligente, General, Avanzado y
  Acerca de. Cada una repensada como experiencia, no como formulario.
- **Escriba Design Guide (sistema de diseño v2):** filosofía y principios, color,
  tipografía, iconografía, espaciado y _layout_, forma, **motion**, estados,
  **accesibilidad**, componentes, patrones y microcopy.
- **Consolidación en código:** tokens únicos de forma (un radio, dos sombras, un
  borde), primitiva `Card` (config/hero/metric), biblioteca de estados
  (vacío/cargando) y comportamiento definido (transiciones 150–220 ms, estado
  _pressed_ en botones, `prefers-reduced-motion` global).
- **Inicio como experiencia:** _"Habla. Escriba hace el resto."_ con estado del
  sistema y estadísticas; barra lateral reorganizada por contexto.

### Añadido

- **Dashboard de Agentes (MCP) en vivo:** tiempo activo, número de llamadas,
  actividad reciente y agentes conectados, con **datos reales** del servidor.
- **Panel de estado del sistema** en Avanzado (modelo, MCP, overlay, inicio) y
  herramientas de usuario avanzado (abrir carpeta de logs y de datos).
- **Escritura Inteligente:** las plantillas de IA como biblioteca visual, con
  ejemplos de entrada → salida.
- **Acerca de:** tarjeta de identidad y _"Escriba en números"_.

### Cambiado

- **Microcopy** con tono más humano y tranquilo (_"No pudimos… Inténtalo de nuevo."_).
- **Lema** ajustado a _"Tu voz en tinta"_ (sin coma), en el _wordmark_ y Acerca de.
- **Firma de macOS:** el `.app` deja de reutilizar un certificado ajeno; en el
  repo va ad-hoc (portable, compatible con CI) y los _builds_ locales usan un
  certificado propio **Escriba Self-Signed**.

## [1.1.0] — 2026-07-11

Auditoría de seguridad completa antes de repartir la app. La promesa de Escriba
—**tu voz nunca sale de tu computador**— ahora es literal y auditable.

### Seguridad

- **Agentes (MCP) autenticados:** el servidor local ahora exige un token secreto
  (en la URL que copias al agente). Sin él, cualquier otro programa de tu equipo
  podía leer tu historial de dictados; ahora responde `401`. El token es estable
  entre reinicios, así que configuras el agente una sola vez.
- **Protección anti-rebinding:** una página web maliciosa ya no puede hablarle al
  servidor local para robar tu historial (se validan `Host`/`Origin` → `403`).
- **Intérprete en vivo protegido:** la sala usa un token largo aleatorio (además
  del código de 4 dígitos) y limita los intentos por IP, para que nadie en el
  mismo WiFi adivine el código y espíe la traducción.
- **Menos permisos:** la interfaz ya no puede leer ni escribir toda tu carpeta de
  usuario, solo los datos de la app; se activó una CSP estricta.
- **Privacidad en los logs:** el texto que dictas ya no se guarda en los archivos
  de registro (los que uno adjunta a un reporte de error).

## [1.0.0] — 2026-07-11

Primera versión mayor: la app queda con marca propia completa y toda la interfaz
en español. De aquí en adelante avanzamos 1.0 → 1.9 → 2.0.

### Cambiado

- **Rebrand visual completo:** la app adopta la paleta pergamino / tinta / oro de
  la landing y el logo oficial de la marca (pluma, onda de voz y firma) en la
  barra lateral, el ícono del Dock y el banner del repo.
- **Todo en español:** los idiomas de transcripción y los dispositivos de audio se
  muestran en español ("Auto Detect" → "Detección automática", "Default" →
  "Predeterminado").

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

## [0.8.5] — 2026-07-11

### Agregado

- **Intérprete en vivo:** el equipo levanta una sala en la red local y muestra un
  **QR**; cada asistente lo abre en su teléfono y lee los subtítulos **en su
  propio idioma**, traducidos por el motor local (una vez por idioma). Para guías
  turísticos, clases y charlas con extranjeros.
- Micrófono en vivo del guía (dictado → sala), selector de idioma de origen,
  cambio de idioma en vivo desde el teléfono sin reiniciar y mejor voz TTS.

## [0.8.0] — 2026-07-10

### Agregado

- **Supresión de ruido de fondo (RNNoise):** limpia ventilador, teclado y tráfico
  del micrófono antes de transcribir. 100% local.
- **Buscar y reemplazar:** reglas propias (texto literal o expresión regular) que
  se aplican al texto dictado después de la corrección con IA.
- **Pausar la música al dictar:** pausa Música/Spotify mientras hablas y las
  reanuda al terminar; solo se pausa lo que estaba sonando (macOS por ahora).
- **Reparación de configuración:** si un campo de ajustes se corrompe, se repara
  solo ese campo en vez de perder toda la configuración.

## [0.5.0] — 2026-07-09

### Agregado

- **Estudio de transcripción:** arrastra un audio o video → transcripción con
  marcas de tiempo → exporta **SRT / VTT / TXT / JSON** + **resumen con IA**.
  Incluye soporte de audios **`.opus` de WhatsApp**.
- **Onboarding es-first:** recomendaciones de modelo por idioma y catálogo en
  español (antes recomendaba un modelo solo-inglés a todos).
- **Estadísticas de uso** (palabras, racha, tiempo ahorrado).

## [0.4.0] — 2026-07-08

### Agregado

- **Háblale a cualquier texto:** selecciona texto en cualquier app, mantén el
  atajo y dicta la instrucción (_"hazlo más formal"_, _"resúmelo en 3 líneas"_,
  _"tradúcelo al portugués"_); el texto se reemplaza en el lugar.

## [0.3.0] — 2026-07-08

### Agregado

- **Poderes de dictado:** dictado natural (limpia muletillas y repeticiones),
  presets de comunidad y diccionario personal conectado a la corrección.
- **Traducción al dictar:** hablas en un idioma y el texto se pega en otro.

### Cambiado

- Post-proceso con IA des-enterrado: pasa de ser una función experimental
  escondida a una capacidad principal de la app.

## [0.2.0] — 2026-07-07

### Agregado

- **Motor de IA local:** sidecar de LLM local incluido, con cascada de respaldo
  (local → Ollama → Apple Intelligence → texto crudo). Sin claves de API en el
  camino feliz.

### Cambiado

- **Rebrand completo de Handy a Escriba:** identidad, marca y wordmark propios.

---

La versión 0.1 corresponde a la base original de
[Handy](https://github.com/cjpais/handy); la historia propia de Escriba empieza
en 0.2.0.
