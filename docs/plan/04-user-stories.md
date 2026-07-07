# User Stories — Escriba (MoSCoW)

> Avatar primario: builder hispanohablante de la comunidad Imperio (vibecodea,
> crea contenido, responde decenas de mensajes al día). Avatar secundario:
> usuario no técnico tipo Flor (estándar de UX: si ella no puede instalarlo y
> usarlo sola, no está terminado).

## MUST (sin esto no hay entrega)

1. Como usuario nuevo, **instalo Escriba sin cuenta ni login** y dicto mi primer texto en <5 minutos desde la descarga. *AC: onboarding permisos → modelo → listo; máquina virgen macOS y Windows.*
2. Como usuario, **mantengo un atajo, hablo, y el texto aparece donde está mi cursor**, en cualquier app. *AC: flujo original de Handy sin regresión.*
3. Como usuario, **activo la corrección con IA sin pagar ni configurar API keys**: la app descarga su propio modelo con un botón y barra de progreso. *AC: dictado corregido con wifi apagado; si la descarga falla, cascada Ollama → Apple Intelligence → texto crudo con aviso claro.*
4. Como vibecoder, **dicto ideas desordenadas y se pega un prompt estructurado** (rol, contexto, tarea, restricciones, formato) listo para Cursor/Claude/ChatGPT. *AC: 5 dictados caóticos reales → prompts que un LLM ejecuta bien a la primera.*
5. Como usuario, **dicto con muletillas y autocorrecciones y sale texto limpio** ("mándalo el lunes... no mejor, el martes" → "mándalo el martes"). *AC: batería es-CL/es-ES contra Qwen3-4B local.*
6. Como usuario, **selecciono texto en cualquier app y lo edito hablando** ("hazlo más formal", "resúmelo en 3 líneas"). *AC: funciona en Notas, WhatsApp Web y VS Code; clipboard restaurado; fallback sin selección.*
7. Como visitante de la landing, **entiendo en 10 segundos qué es, cuánto cuesta ($0) y descargo para mi OS**, con instrucciones honestas de instalación (Gatekeeper/SmartScreen con GIFs). *AC: Lighthouse >90, responsive móvil.*
8. Como juez, **veo un video de ≤2 min** que me hace querer instalarla. *AC: subtítulos siempre; clímax demoable en vivo.*

## SHOULD (ganan el concurso)

9. Como usuario, **dicto en español y se pega en el idioma que elegí** con su propio atajo. *AC: es→en correcto; no traduce dos veces si el toggle nativo Whisper está activo.*
10. Como usuario, **la app ajusta el tono según dónde escribo** (WhatsApp casual, Mail formal, Cursor → Prompt Maestro) sin tocar settings. *AC: mismo dictado, 3 apps, 3 salidas.*
11. Como usuario, **mis nombres propios y términos se respetan** en la corrección (diccionario personal conectado al phraser). *AC: término custom sobrevive la corrección.*
12. Como usuario no angloparlante, **el onboarding me recomienda un modelo que hable MI idioma** y las descripciones están en español. *AC: UI es → Whisper multilingüe #1; Parakeet con badge "solo inglés".*
13. Como usuario, **encuentro la corrección con IA sin arqueología**: sección visible en el menú, no enterrada en "experimental". *AC: post-proceso visible siempre, toggle dentro de la página.*
14. 🤝 (si D) Como creador, **arrastro un audio/video y obtengo transcripción + subtítulos SRT/VTT + resumen**, gratis y local. *AC: mp3, .m4a de WhatsApp y mp4 → SRT válido importable en CapCut/YouTube.*
15. 🤝 (si B) Como guía/profe, **muestro un QR y cada asistente ve subtítulos en SU idioma** en su teléfono. *AC: 2 teléfonos, 2 idiomas, latencia <5s por segmento, en hotspot.*

## COULD (si hay aire)

16. Como usuario, **veo mis estadísticas**: palabras dictadas, racha, tiempo ahorrado vs teclear (45wpm vs ~200wpm). *AC: card sobre el historial, datos de la DB existente.*
17. Como usuario, **busco en mi historial** y filtro los guardados. 
18. Como usuario, **pregunto por voz sobre texto de solo lectura** (web/PDF) y la respuesta aparece en el overlay sin reemplazar nada.
19. Como usuario de Mac de 8GB, **la app me sugiere el modelo ligero** y descarga el LLM de la memoria a los 2 min de inactividad.

## WON'T (esta edición)

- Diarización/speaker labels · móvil · extensión de navegador · equipos/cuentas · chat con transcripciones · aprendizaje automático de estilo · firma Apple/Windows · merges de upstream.
