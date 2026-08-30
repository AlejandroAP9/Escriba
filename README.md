<p align="center">
  <img src="./.github/banner.svg" width="760" alt="Escriba — Tu voz en tinta" />
</p>

<p align="center">
  <a href="https://github.com/AlejandroAP9/Escriba/releases/latest"><img src="https://img.shields.io/badge/versi%C3%B3n-2.4.1-e6d2a8?style=for-the-badge&labelColor=14102a" alt="última versión" /></a>
  <img src="https://img.shields.io/badge/macOS%20%7C%20Windows%20%7C%20Linux-e6d2a8?style=for-the-badge&labelColor=14102a" alt="macOS, Windows y Linux" />
  <img src="https://img.shields.io/badge/100%25-local-6ee7a0?style=for-the-badge&labelColor=14102a" alt="100% local" />
  <img src="https://img.shields.io/badge/licencia-MIT-e6d2a8?style=for-the-badge&labelColor=14102a" alt="licencia MIT" />
</p>

<p align="center">
  <b>Dictado por voz con IA local, gratis.</b><br/>
  Aprietas un atajo, hablas, y tu voz aparece como texto en cualquier app.<br/>
  Local por defecto. Sin claves de API. Tu voz nunca sale de tu computador.
</p>

<p align="center">
  <a href="https://github.com/AlejandroAP9/Escriba/releases/latest"><b>⬇️  Descargar la última versión</b></a>
</p>

---

## ✒️ Qué es Escriba

Escriba es una app de escritorio que convierte tu voz en texto con inteligencia artificial, **corriendo en tu propia máquina**. No es solo dictado: es un motor de IA local del que cuelgan varias herramientas —corrección, traducción, transcripción de archivos, interpretación en vivo— que funcionan por defecto sin conexión y sin pagar una suscripción. (Puedes conectar un proveedor remoto opcional si quieres; viene apagado.)

La idea es simple: **lo que otras apps cobran por mes y procesan en su nube, aquí es gratis, ilimitado y privado.**

## ⬇️ Descarga

Ve a **[releases/latest](https://github.com/AlejandroAP9/Escriba/releases/latest)** y elige el instalador de tu sistema:

| Sistema                                    | Archivo a descargar            |
| ------------------------------------------ | ------------------------------ |
| 🍎 **macOS** (Apple Silicon · M1/M2/M3/M4) | `Escriba_x.y.z_aarch64.dmg`    |
| 🍎 **macOS** (Intel)                       | `Escriba_x.y.z_x64.dmg`        |
| 🪟 **Windows** 10/11                       | `Escriba_x.y.z_x64-setup.exe`  |
| 🐧 **Linux** (Debian/Ubuntu)               | `Escriba_x.y.z_amd64.deb`      |
| 🐧 **Linux** (cualquier distro)            | `Escriba_x.y.z_amd64.AppImage` |

> ### ⚠️ macOS te va a bloquear el archivo la primera vez. Es normal.
>
> Al abrir el `.dmg` verás un aviso de que **no se puede abrir**, y las únicas
> opciones serán **Mover a la papelera** o **Cancelar**. **No lo mandes a la
> papelera: el archivo está bien.** Escriba está firmada, pero no notarizada por
> Apple (eso cuesta una cuota anual de desarrollador), así que macOS la trata
> como de origen desconocido.
>
> **Qué hacer, paso a paso:**
>
> 1. En el aviso, pulsa **Cancelar** (nunca "Mover a la papelera").
> 2. Ve a **Ajustes del Sistema → Privacidad y seguridad**.
> 3. Baja hasta la sección **Seguridad**: verás el aviso de que se bloqueó
>    Escriba, con un botón **Abrir de todos modos**.
> 4. Púlsalo, escribe tu contraseña del Mac y confirma.
> 5. Ahora sí se monta el disco: arrastra Escriba a **Aplicaciones**.
>
> Después de instalar, concede el permiso de **Accesibilidad** (ver más abajo) o
> el atajo no escribirá en ninguna parte.
>
> _(A veces basta con hacer clic derecho sobre el archivo → **Abrir**. Si esa
> opción no aparece o no funciona, usa los cinco pasos de arriba, que funcionan
> siempre.)_
>
> **Windows:** si aparece SmartScreen, haz clic en **Más información** →
> **Ejecutar de todas formas**.

> **macOS: después de cada actualización hay que volver a dar permiso de Accesibilidad.**
> No es un error de la app. Escriba se firma con un certificado propio, y macOS
> considera cada build una identidad distinta, así que revoca el permiso que le
> habías dado a la anterior. Si tras actualizar el atajo deja de responder:
> **Ajustes del Sistema → Privacidad y seguridad → Accesibilidad**, quita Escriba
> de la lista con el botón `−` y vuelve a añadirla con `+`.

## 🚀 Qué puede hacer

|                                  |                                                                                                                                                                                                                                                                                             |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 🎙️ **Dictado con IA**            | Atajo global, hablas, y el texto aparece donde estés. Filtrado de silencios (VAD) + Whisper/Parakeet locales con aceleración por GPU.                                                                                                                                                       |
| 🇪🇸 **Español profundo**          | Restaura tildes inequívocas, convierte nombres de emojis y numerales hablados a cifras sin tocar expresiones como “mil gracias” u “hora y media”.                                                                                                                                           |
| 🗽 **Dictado libre**             | Cero atajos: actívalo y habla. Cada frase se escribe sola donde esté tu cursor, cortada por el detector de voz. Con indicador siempre visible.                                                                                                                                              |
| 👁️ **Revisar antes de pegar**    | Opcional: el dictado se muestra en pantalla antes de escribirse — pégalo, descártalo o dicta una corrección con tu atajo. Para correos delicados.                                                                                                                                           |
| 💬 **Sesiones**                  | Habla una hora y llévate un documento listo: conversa con la IA local (te responde con voz) o deja que solo escuche tu reunión, entrevista o clase. Al terminar: acta, apuntes o nota, redactados por el motor local.                                                                       |
| 🖥️ **Audio del sistema**         | Sesiones también escucha lo que suena en tu computador: la otra parte del Zoom/Meet entra al acta como "Otros", con marca de tiempo. Actas de reunión a dos voces, sin nube. (macOS 13+)                                                                                                    |
| 🎙️ **Intérprete de reuniones**   | Interpretación en vivo de tus videollamadas, ida y vuelta: lo que suena llega en tu idioma; lo que dictas sale traducido y hablado _dentro_ de la llamada por un micrófono virtual integrado (1 clic). Hablas español, el otro escucha inglés. 100% local. (idea de John Walter, macOS 13+) |
| 🔊 **Tu tinta en voz**           | Selecciona texto en cualquier app y ⌥⇧R: Escriba lo lee en voz alta con la mejor voz de tu equipo. Revisa tus borradores con los oídos.                                                                                                                                                     |
| 👓 **Para todos los ojos**       | Tema Día/Noche/Sistema, tamaño de texto ajustable (90-130%), **alto contraste**, **asistencia para daltonismo**, **Modo Calma** (quietud total, más aire, superficies planas) y foco siempre visible. Nacida de la comunidad.                                                               |
| 🪶 **Plumín te ayuda**           | Además de acompañar la primera instalación, responde dudas sobre Escriba desde una guía local, comprueba tu configuración, abre la sección correcta y puede leer la respuesta en voz alta.                                                                                                  |
| 📝 **Obsidian enlazable**        | Exporta notas Markdown con `[[enlaces]]` revisables, mantiene un índice `Escriba.md` y puede enviar dictados a una bandeja diaria. Solo lee nombres de notas; nunca su contenido ni la red.                                                                                                 |
| ⌨️ **Escribir en vez de dictar** | Corrige o **traduce** un texto escrito con el mismo motor local, sin usar la voz. Para el aula compartida, la afonía o quien no puede hablar en ese momento.                                                                                                                                |
| ✨ **Corrección con IA**         | Limpia muletillas y repeticiones, ordena listas y ajusta el tono según la app (WhatsApp casual, Mail formal, prompts para Cursor…).                                                                                                                                                         |
| 🗣️ **Edición por voz**           | Selecciona texto en cualquier app, mantén el atajo y dile qué hacer: _"hazlo más formal"_, _"resúmelo en 3 líneas"_, _"tradúcelo al portugués"_.                                                                                                                                            |
| 🌐 **Traducción al dictar**      | Hablas en un idioma y el texto se pega en otro.                                                                                                                                                                                                                                             |
| 🎬 **Estudio**                   | Arrastra un audio o video (incluso notas de voz `.opus` de WhatsApp) → transcripción con marcas de tiempo → exporta **SRT / VTT / TXT / JSON** + **resumen con IA**. Subtítulos para tus Reels, gratis y sin nube.                                                                          |
| 📡 **Intérprete en vivo**        | Tu Mac levanta una sala y muestra un **QR**; cada asistente lo abre en su teléfono y lee los subtítulos **en su propio idioma**. Para guías turísticos, clases y charlas con extranjeros.                                                                                                   |
| 🔄 **Traductor cara a cara**     | Conversación 1-a-1 bidireccional con **detección automática de idioma**, pantalla grande y voz.                                                                                                                                                                                             |
| 🤖 **Agentes (MCP)**             | Un servidor local (puerto fijo) para que **Claude Code, Cursor o Cline** usen a Escriba como herramientas: transcribir, traducir, resumir, pulir texto y **leer tu historial de dictados**. 100% local.                                                                                     |
| ⌨️ **CLI para scripts**          | Transcribe archivos desde la terminal con el mismo motor de la app: `escriba --transcribe-file audio.opus --json`. Benchmarks reproducibles con `--repeat`. Ver [la tabla completa de banderas](#%EF%B8%8F-desde-la-terminal-cli).                                                          |
| 🎚️ **Re-transcribir**            | Mismo audio, otro modelo: compara precisión sin volver a subir nada.                                                                                                                                                                                                                        |
| 🎤 **Micrófono en los campos**   | Dicta directo dentro de la propia app, en cualquier campo de texto.                                                                                                                                                                                                                         |
| 🔇 **Supresión de ruido**        | Limpia ventilador, teclado y tráfico del micrófono antes de transcribir. 100% local (RNNoise).                                                                                                                                                                                              |
| 🔁 **Buscar y reemplazar**       | Reglas propias (texto literal o expresión regular) que se aplican al texto dictado.                                                                                                                                                                                                         |
| ⏯️ **Pausar la música**          | Pausa Música/Spotify mientras dictas y las reanuda al terminar; solo lo que estaba sonando.                                                                                                                                                                                                 |

## 🔒 Local por defecto, 100% gratis

- **Local por defecto.** La transcripción, la corrección, la traducción y el Intérprete corren en tu computador. El contenido solo sale de tu equipo si tú configuras y seleccionas explícitamente un proveedor remoto (BYOK: OpenAI, Anthropic, etc.), que es opcional y viene apagado.
- **Sin claves de API en el camino feliz.** El motor local viene incluido; no necesitas cuenta ni tarjeta.
- **Ilimitado.** Sin cupos de palabras por semana ni límites de minutos.
- **Open source.** Puedes leer, auditar y extender cada línea.

> Qué usa internet y qué no: el dictado, la transcripción y toda la IA local funcionan sin conexión. Requieren descargar los modelos una vez. El actualizador consulta si hay una versión nueva. Un proveedor remoto (si lo activas tú) envía el texto a ese servicio.

## 🎯 Filosofía

> **Escriba existe para que la interfaz desaparezca.**

Cada elemento responde una sola pregunta: **¿ayuda a escribir?** Si la respuesta es no, desaparece.

Escriba no compite por atención, compite por desaparecer. Cuando el usuario recuerda la interfaz, hemos fallado; cuando recuerda lo que escribió, hemos acertado.

**Cinco principios** gobiernan cada decisión de producto y diseño:

- **Privado** — nada parece conectado a internet; tu voz nunca sale del equipo.
- **Elegante** — pocas distracciones, mucho aire, la tipografía como protagonista.
- **Rápido** — cada acción importante está a un clic, sin pasos innecesarios.
- **Profesional** — la herramienta de un periodista o un escritor, no un experimento.
- **Atemporal** — sin modas; que en cinco años se siga viendo moderna.

> El sistema de diseño vive en el código, en un solo sitio: los tokens de color y tipografía están en [`src/styles/theme.css`](./src/styles/theme.css) y la escala, formas y sombras en [`src/App.css`](./src/App.css). Cambiar la marca es cambiar esos dos archivos.

## ⚙️ Cómo funciona

1. **Aprieta** el atajo configurable (o usa _push-to-talk_).
2. **Habla** mientras el atajo está activo.
3. **Suelta** y Escriba transcribe con el modelo que elijas.
4. **Listo:** el texto se pega en la app que estés usando.

Todo el procesamiento es local: el silencio se filtra con **Silero VAD**, la transcripción usa modelos **Whisper** (Small/Medium/Turbo/Large) o **Parakeet V3** (optimizado para CPU, con detección automática de idioma), y la corrección/traducción usa un **LLM local**.

## ⌨️ Desde la terminal (CLI)

Escriba también funciona sin ventana, para scripts, benchmarks y automatización. El mismo binario de la app es el CLI:

```bash
# Transcribir cualquier archivo (wav, mp3, m4a, opus, ogg, flac, mp4/video)
escriba --transcribe-file reunion.opus

# Salida JSON con métricas, repitiendo 3 veces (benchmark reproducible)
escriba --transcribe-file prueba.opus --json --repeat 3

# Subtítulos: escribe reunion.srt junto al archivo
escriba --transcribe-file reunion.mp4 --export-srt
```

| Bandera                           | Qué hace                                                                                                                              |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| `-f, --transcribe-file <ARCHIVO>` | Transcribe un archivo y termina. Sin micrófono ni conexión; el modelo debe estar instalado. Acepta los mismos formatos que el Estudio |
| `--model <ID>`                    | Modelo a usar (por defecto, el elegido en la app). Los ids salen de `--list-models`                                                   |
| `--list-models`                   | Lista los modelos disponibles y sus ids. Honra `--json`                                                                               |
| `--list-devices`                  | Lista los dispositivos de cómputo (CPU/GPU) con sus índices. Honra `--json`                                                           |
| `--device-index <N>`              | Fuerza un dispositivo de cómputo por índice (solo modelos Whisper)                                                                    |
| `--repeat <N>`                    | Repite la transcripción N veces; `best_ms` reporta la más rápida                                                                      |
| `--export-srt`                    | Escribe un `.srt` de subtítulos junto al archivo de entrada                                                                           |
| `--json`                          | Salida JSON (`text`, `audio_secs`, `load_ms`, `transcribe_ms`, `best_ms`, `rtf`)                                                      |
| `--toggle-transcription`          | Alterna la grabación en una instancia ya abierta (para atajos del sistema)                                                            |
| `--toggle-post-process`           | Alterna la grabación con post-proceso en una instancia ya abierta                                                                     |
| `--cancel`                        | Cancela la operación en curso de la instancia abierta                                                                                 |
| `--start-hidden`                  | Arranca sin mostrar la ventana (queda el icono de bandeja)                                                                            |
| `--no-tray`                       | Arranca sin icono de bandeja (cerrar la ventana termina la app)                                                                       |
| `--debug`                         | Log detallado para diagnóstico                                                                                                        |

Un archivo corrupto o no soportado termina con mensaje claro y código de salida 2, apto para scripts.

## 🌍 Idiomas

Interfaz **completa en español e inglés**. Otros 19 idiomas están **en progreso** (la estructura está, pero muchas cadenas todavía muestran el texto en inglés). La transcripción es multilingüe según el modelo elegido.

## 📜 Historial

Toda la evolución de Escriba, versión por versión (fechas reales de la dupla en los Juegos Imperiales):

| Versión | Fecha  | Novedades                                                                                                                                          |
| ------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| 2.4.1   | 11-ago | 🙌 El crédito de cada función, en la pantalla donde se usa                                                                                         |
| 2.4.0   | 10-ago | 🛡️ Secuestro por dictado cerrado en los 4 modos + 🎯 Parakeet V3 como modelo recomendado + 🪟 Windows dice la verdad cuando bloquea el motor de IA |
| 2.3.1   | 09-ago | 🎙️ Modo fiel + 🔄 actualización desde la app + 🎬 Estudio renovado + 📝 actas largas confiables                                                    |
| 2.3.0   | 09-ago | 🪶 Plumín ayuda de verdad + 🇪🇸 español profundo + 🔐 historial cifrado + 📝 Obsidian enlazable + CLI y benchmarks                                  |
| 2.2.4   | 30-jul | 🐛 Lo que encontró el QA: historial y estadísticas ya no quedan en cero + diccionario que funciona sin comerse palabras                            |
| 2.2.3   | 30-jul | 🔐 Ronda de auditoría externa: las claves fuera del alcance de la interfaz + confirmación al borrar + asistente que exige modelo                   |
| 2.2.2   | 30-jul | 🧹 **La casa ordenada**: cada ajuste donde lo buscarías + actas de sesiones de una hora + plantillas probadas contra el motor real                 |
| 2.2.1   | 28-jul | 🔁 Actualizar con marcha atrás (fin del bucle de cierres) + ⚡ Intérprete sin espera: la frase sale ya y cada idioma llega después                 |
| 2.2.0   | 28-jul | 🪶 **Plumín te recibe**: asistente de bienvenida que termina con tu primera dictada + notas de Obsidian en su propia carpeta                       |
| 2.1.1   | 28-jul | 🔧 Ocho arreglos salidos de probar la app: documento de sesión, "¿" del dictado, alto contraste real y ajustes de Obsidian                         |
| 2.1.0   | 26-jul | 🛡️ **La ronda del blindaje**: 5 auditorías aplicadas + 📝 Enviar a **Obsidian** + ♿ Modo Calma, alto contraste y daltonismo                       |
| 2.0.0   | 21-jul | 🎙️ **Intérprete de reuniones** en vivo, ida y vuelta (idea de John Walter) + micrófono virtual integrado                                           |
| 1.9.1   | 18-jul | 🐧 Motor local en **Linux** + fixes del Traductor (QA Flor)                                                                                        |
| 1.9.0   | 17-jul | **Revisar antes de pegar** + fixes del QA Windows de la dupla                                                                                      |
| 1.8.0   | 16-jul | 🤝 Tanda comunidad: sugerencias de diccionario + Plumín empático                                                                                   |
| 1.7.0   | 15-jul | 🪶 Nace **Plumín** (mascota) + onda de voz viva                                                                                                    |
| 1.6.0   | 15-jul | **Dictado libre** (cero teclas) + bandeja con acciones                                                                                             |
| 1.5.0   | 15-jul | Panel de **Permisos** + estado real + ventana con memoria                                                                                          |
| 1.4.0   | 15-jul | 👓 Tema Día/Noche + tamaño de texto (idea de la comunidad)                                                                                         |
| 1.3.2   | 15-jul | La pluma llega a la barra de menú                                                                                                                  |
| 1.3.1   | 15-jul | QA de la dupla: aviso de motor faltante + indicador de modo activo                                                                                 |
| 1.3.0   | 15-jul | 🖥️ **Audio del sistema** + **Tu tinta en voz** + **Tonos por app**                                                                                 |
| 1.2.0   | 12-jul | 🎨 Rework de las 11 pantallas + Design System                                                                                                      |
| 1.1.0   | 11-jul | 🔒 Auditoría de seguridad (MCP + privacidad)                                                                                                       |
| 1.0.0   | 11-jul | Rebrand visual + español total                                                                                                                     |
| 0.10.0  | 11-jul | **Agentes (MCP)** + re-transcribir + micrófono                                                                                                     |
| 0.9.0   | 11-jul | **Traductor** cara a cara + lituano + copiar                                                                                                       |
| 0.8.5   | 11-jul | **Intérprete en vivo** (QR)                                                                                                                        |
| 0.8.0   | 10-jul | **Supresión de ruido** + buscar/reemplazar                                                                                                         |
| 0.5.0   | 09-jul | **Estudio** (SRT + resumen, `.opus`) + onboarding es-first + estadísticas                                                                          |
| 0.4.0   | 08-jul | Háblale a cualquier texto (edición por voz)                                                                                                        |
| 0.3.0   | 08-jul | Poderes de dictado + traducción al dictar                                                                                                          |
| 0.2.0   | 07-jul | Rebrand + **motor de IA local**                                                                                                                    |

## 🙏 Construido sobre Handy

Escriba es un _rework_ de **[Handy](https://github.com/cjpais/handy)**, la excelente app de dictado open source de **[CJ Pais](https://github.com/cjpais)**, publicada bajo licencia MIT. Gracias a CJ y a la comunidad de Handy por sentar unas bases tan sólidas y forkables. Escriba conserva esa filosofía —libre, privada, local— y le suma una capa de IA local (corrección, traducción, Estudio, Intérprete, Traductor y Agentes).

## 🙌 Gracias

Escriba tiene features que no se nos ocurrieron a nosotros. Cada una lleva el crédito dentro de la propia app, en la pantalla donde se usa y en **Acerca de → Gracias**. Aquí está la lista completa.

**De la comunidad**

| Quién                         | Qué aportó                                                                                                                                                                |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Pedro Sánchez**             | Toda la tanda de inclusión visual (Modo Calma, alto contraste, daltonismo, tamaño de texto), el respaldo de Apple Intelligence y que Plumín perciba el ánimo de la sesión |
| **John Walter**               | El Intérprete de reuniones, que traduce una llamada en las dos direcciones                                                                                                |
| **Juan Francisco Ceccarelli** | Los numerales hablados a cifras, para dictar en planillas                                                                                                                 |
| **Antonio Bocanet**           | Reportó que el modelo recomendado destrozaba palabras cortas en español. Lo medimos, tenía razón, y la recomendación cambió para todos                                    |
| **Alexa Sánchez**             | Descubrió que en Windows la instalación decía que el motor de IA estaba listo cuando el sistema lo había bloqueado                                                        |
| **Flor Vallejo**              | La otra mitad de la dupla: identidad visual, video y landing                                                                                                              |

**Duplas de los Juegos Imperiales 2026**

Compitieron contra nosotros y aun así nos enseñaron algo. Lo tomamos, y lo decimos.

| Dupla           | Qué nos enseñó                                                                                                                                                 |
| --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Takhygraphe** | Los modos visuales de accesibilidad, y un export a Obsidian con nota índice y menciones enlazables que era mejor que el nuestro                                |
| **Abrax**       | Que el dictado en español merecía trabajo lingüístico de verdad, que aquí se volvió tildes, emojis dictados y numerales                                        |
| **Dictum**      | La línea de comandos headless con benchmarks reproducibles, gracias a la cual hoy publicamos nuestras propias mediciones                                       |
| **Fuwa**        | La ayuda dentro de la app que solo responde sobre la app, que aquí se volvió Pregúntale a Plumín                                                               |
| **Diapasón**    | Publicaron una medición que encontró un fallo nuestro: el diccionario personal se comía palabras en castellano. Que te corrijan en público también se agradece |

Y una idea que llegó después del concurso: el grabador de sesiones a prueba de fallos sigue a **[reunion-local](https://github.com/flopez1977/reunion-local)** (flopez1977, MIT): journal del estado y conservar el audio.

## 📄 Licencia

MIT. Ver [LICENSE](./LICENSE). El código original de Handy es © 2025 CJ Pais; el trabajo de Escriba es © 2026 Alejandro Álvarez y Flor Vallejo, bajo la misma licencia.

Los modelos y el software de terceros que Escriba empaqueta, descarga o instala tienen sus propias licencias: están todas en **[THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)**. Dos avisos que conviene leer si vas a usar Escriba en un contexto comercial: el catálogo incluye **un modelo CC-BY-NC-4.0** (`canary-1b`, uso no comercial) y **14 modelos CC-BY-4.0** que exigen atribución.

---

<p align="center">
  Hecho con ✒️ para los <b>Juegos Imperiales</b> por <b>Alejandro</b> &amp; <b>Flor</b>.
</p>
