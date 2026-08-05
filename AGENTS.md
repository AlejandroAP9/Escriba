# AGENTS.md

Guía para los asistentes de IA que trabajen sobre este repositorio.

## Comandos de desarrollo

**Requisitos previos:**

- [Rust](https://rustup.rs/) (última estable)
- Gestor de paquetes [Bun](https://bun.sh/)

**Desarrollo:**

```bash
# Instalar dependencias
bun install

# Ejecutar en modo desarrollo
bun run tauri dev
# Si da error de cmake en macOS:
CMAKE_POLICY_VERSION_MINIMUM=3.5 bun run tauri dev

# Compilar para producción
bun run tauri build

# Solo frontend
bun run dev        # Levanta el servidor de Vite
bun run build      # Compila el frontend (TypeScript + Vite)
bun run preview    # Previsualiza el frontend compilado
```

**Linter y formato (correr antes de cada commit):**

```bash
bun run lint              # ESLint del frontend
bun run lint:fix          # ESLint con arreglo automático
bun run format            # Prettier + cargo fmt
bun run format:check      # Comprueba el formato sin tocar nada
bun run format:frontend   # Solo Prettier
bun run format:backend    # Solo cargo fmt
```

**Modelo de VAD (hace falta para desarrollar):**

```bash
mkdir -p src-tauri/resources/models
curl -o src-tauri/resources/models/silero_vad_v4.onnx https://blob.handy.computer/silero_vad_v4.onnx
```

Para el detalle de compilación por plataforma, ver [BUILD.md](BUILD.md).

## Panorama de la arquitectura

Escriba es una app de escritorio multiplataforma de voz a texto, hecha con Tauri 2.x (backend en Rust + frontend en React/TypeScript). Es un rework de [Handy](https://github.com/cjpais/handy) (© CJ Pais, MIT).

### Estructura del backend (src-tauri/src/)

- `lib.rs` — Punto de entrada, arranque de Tauri, inicialización de los managers
- `managers/` — Lógica principal:
  - `audio.rs` — Grabación y gestión de dispositivos
  - `model.rs` — Descarga y gestión de modelos
  - `transcription.rs` — Pipeline de voz a texto
  - `history.rs` — Almacenamiento del historial
- `audio_toolkit/` — Procesamiento de audio de bajo nivel:
  - `audio/` — Enumeración de dispositivos, grabación, remuestreo
  - `vad/` — Detección de actividad de voz (Silero VAD)
- `commands/` — Comandos de Tauri, la frontera con el frontend
- `cli.rs` — Definición de argumentos de línea de comandos (clap derive)
- `shortcut.rs` — Atajos de teclado globales
- `settings.rs` — Ajustes de la aplicación
- `overlay.rs` — Ventana de grabación superpuesta (depende de la plataforma)
- `signal_handle.rs` — La función reusable `send_transcription_input()`
- `utils.rs` — Ayudas de detección de plataforma

### Estructura del frontend (src/)

- `App.tsx` — Componente principal, con el flujo de bienvenida
- `components/` — Componentes de React:
  - `settings/` — Pantallas de ajustes
  - `model-selector/` — Gestión de modelos
  - `onboarding/` — Asistente de primera vez
  - `overlay/` — Interfaz de la ventana de grabación
  - `update-checker/` — Avisos de actualización
  - `shared/`, `ui/`, `icons/`, `footer/` — Componentes compartidos
- `hooks/useSettings.ts` — Hook de estado de los ajustes
- `stores/settingsStore.ts` — Store de Zustand para los ajustes
- `bindings.ts` — Tipos generados por tauri-specta. OJO: solo se exportan en builds de depuración, así que fuera de ahí se editan a mano
- `overlay/` — Punto de entrada de la ventana de grabación
- `lib/types.ts` — Tipos de TypeScript compartidos

### Patrones de arquitectura

**Managers:** la funcionalidad principal se organiza en managers (Audio, Model, Transcription) que se inicializan al arrancar y viven en el estado de Tauri.

**Comandos y eventos:** del frontend al backend por comandos de Tauri; del backend al frontend por eventos.

**Pipeline:** audio → VAD → Whisper/Parakeet → texto → portapapeles/pegado.

**Flujo del estado:** Zustand → comando de Tauri → estado en Rust → persistencia (tauri-plugin-store).

### Tecnologías

**Librerías principales:**

- `transcribe-cpp` — Inferencia local de la familia Whisper (GGML/GGUF) con aceleración por GPU
- `transcribe-rs` — Reconocimiento de voz ONNX (Parakeet, Moonshine, SenseVoice…)
- `cpal` — Entrada/salida de audio multiplataforma
- `vad-rs` — Detección de actividad de voz
- `rdev` — Atajos de teclado globales
- `rubato` — Remuestreo de audio
- `rodio` — Reproducción de los sonidos de aviso

### Flujo de la aplicación

1. **Arranque:** la app abre minimizada en la bandeja, carga ajustes e inicializa los managers
2. **Modelo:** en la primera vez se descarga el modelo elegido
3. **Grabación:** el atajo global dispara la grabación, con filtrado por VAD
4. **Proceso:** el audio va al modelo para transcribirse
5. **Salida:** el texto se pega en la app activa

### Sistema de ajustes

Los ajustes se guardan con el plugin de store de Tauri, con actualización reactiva:

- Atajos de teclado (configurables, admiten pulsar para hablar)
- Dispositivos de audio (micrófono y salida)
- Preferencias de modelo
- Sonidos de aviso y opciones de traducción

### Instancia única

La app admite una sola instancia: abrirla estando ya en marcha trae la ventana al frente en vez de crear otro proceso. Las banderas de control remoto (`--toggle-transcription` y demás) funcionan lanzando una segunda instancia que pasa sus argumentos a la que ya corre vía `tauri_plugin_single_instance`, y termina.

## Internationalization (i18n)

Todo texto que vea el usuario tiene que pasar por i18next. ESLint lo exige: no se admiten literales en el JSX. Son 21 archivos de idioma y `bun run check:translations` falla si a alguno le falta una clave.

**Para añadir texto nuevo:**

1. Añade la clave en `src/i18n/locales/en/translation.json`
2. Úsala en el componente: `const { t } = useTranslation(); t('ruta.de.la.clave')`

**Estructura:**

```
src/i18n/
├── index.ts           # i18n setup
├── languages.ts       # Language metadata
└── locales/
    ├── en/translation.json  # Inglés (origen)
    ├── de/, es/, fr/, ja/, ru/, zh/, ...
    └── ...
```

Para contribuir traducciones, ver [CONTRIBUTING_TRANSLATIONS.md](CONTRIBUTING_TRANSLATIONS.md).

## Estilo de código

**Rust:**

- Corre `cargo fmt` y `cargo clippy` antes de cada commit
- Maneja los errores de forma explícita (nada de `unwrap` en producción)
- Nombres descriptivos, y comentarios de documentación en las APIs públicas

**TypeScript/React:**

- TypeScript estricto, sin `any`
- Componentes funcionales con hooks
- Tailwind CSS para los estilos
- Alias de rutas: `@/` → `./src/`

## Protocolo de verificación (auditoría y cambios de código)

Motivo: en varias rondas de auditoría sobre este proyecto, todo hallazgo que partió de **leer la función completa, sus consumidores y su propósito** se sostuvo al aplicarlo. Todo hallazgo o recomendación que partió de **un grep o un match de texto sin leer el contexto completo** falló. Esta sección existe para que cualquier agente que audite o modifique este repo aplique la misma disciplina.

### Regla central

**Todo lo que salga de un grep es una hipótesis, no un hallazgo.** Antes de afirmar algo sobre una línea de código — que un panic es alcanzable, que una variable no se usa, que un comportamiento se puede cambiar sin romper nada — hay que:

1. Leer la función completa donde vive esa línea, no solo la línea señalada.
2. Identificar quién consume ese valor/comportamiento en el resto del código, y para qué existe.
3. Si depende de una librería externa, verificar su código fuente o documentación real — nunca asumir el comportamiento por el nombre de la función o el tipo.
4. Si el hallazgo depende de que una condición de fallo sea alcanzable (panic, unwrap, indexación fuera de rango), listar cada guard/condición que la precede y decir explícitamente si la descarta o no.

Si después de esto queda una duda genuina, repórtalo como **"plausible, no confirmado"** — nunca como "verificado". Ambas etiquetas son útiles; confundirlas no lo es.

### Verificación adversarial

Antes de cerrar cualquier hallazgo de severidad Alta, intenta refutarlo: relee la función completa buscando la razón por la que _no_ sería alcanzable. Si la encuentras, baja la severidad o cambia el veredicto. Si no la encuentras tras intentarlo genuinamente, el hallazgo se mantiene — y el informe puede decir que se buscó una vía de invalidación y no apareció.

### Antes de aplicar cualquier parche

Busca comentarios existentes cerca del código que vas a cambiar. Si hay uno, es probable que documente un invariante que alguien ya descubrió de la forma difícil — leerlo antes de tocar el código de al lado es más barato que redescubrirlo por un bug. (Ejemplo real de este repo: el comentario en `schedule_lazy_close` explicando por qué esa función no toma el lock de estado evitó — cuando se leyó — un self-deadlock que un parche anterior habría introducido si se hubiera aplicado sin leerlo.)

### Formato de evidencia para hallazgos con disparador afirmado

```text
Afirmación: [qué se dice que es alcanzable/cierto/seguro de cambiar]
Línea señalada: [archivo:línea — contenido literal, no reconstruido de memoria]
Función completa leída: [sí/no]
Consumidores/propósito trazados: [quién usa esto y para qué, con archivo:línea]
Guards trazados (si aplica disparador de fallo): [cada condición previa, y si descarta o no el caso]
Dependencia externa verificada: [sí/no — qué se consultó]
Intento de refutación: [qué se buscó para invalidarlo]
Veredicto: Verificado / Plausible no confirmado
```

## Parámetros de línea de comandos

Escriba admite parámetros de línea de comandos en todas las plataformas, para integrarse con scripts, gestores de ventanas y arranque automático.

**Dónde vive:** `cli.rs` (definiciones), `main.rs` (parseo), `lib.rs` (aplicación), `signal_handle.rs` (lógica compartida)

| Bandera                           | Qué hace                                                                                                                                                                                                                                                         |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--toggle-transcription`          | Alterna la grabación en una instancia en marcha                                                                                                                                                                                                                  |
| `--toggle-post-process`           | Alterna la grabación con post-proceso                                                                                                                                                                                                                            |
| `--cancel`                        | Cancela la operación en curso                                                                                                                                                                                                                                    |
| `--start-hidden`                  | Arranca sin mostrar la ventana (queda el icono de bandeja)                                                                                                                                                                                                       |
| `--no-tray`                       | Arranca sin bandeja (cerrar la ventana termina la app)                                                                                                                                                                                                           |
| `--debug`                         | Modo de depuración con log detallado (Trace)                                                                                                                                                                                                                     |
| `-f, --transcribe-file <ARCHIVO>` | Transcribe un archivo headless y termina. WAV 16 kHz mono va directo; todo lo demás (mp3, m4a, opus, ogg, flac, mp4/video) pasa por el decode local del Estudio. Sin micrófono, sin VAD, sin descarga: el modelo debe estar instalado. Error de entrada → exit 2 |
| `--model <ID>`                    | Modelo a cargar para `--transcribe-file` (por defecto, el seleccionado en la app). Los ids salen de `--list-models`                                                                                                                                              |
| `--device-index <N>`              | Fija el dispositivo de cómputo por índice del registro (ver `--list-devices`). Solo modelos transcribe-cpp; no se persiste                                                                                                                                       |
| `--list-devices`                  | Lista los dispositivos de cómputo con índices y termina. Honra `--json`                                                                                                                                                                                          |
| `--list-models`                   | Lista los modelos disponibles con sus ids y termina. Honra `--json`                                                                                                                                                                                              |
| `--repeat <N>`                    | Repite la transcripción N veces (`best_ms` reporta la más rápida): benchmarks reproducibles                                                                                                                                                                      |
| `--export-srt`                    | Estudio por CLI: escribe un `.srt` junto al archivo de entrada                                                                                                                                                                                                   |
| `--json`                          | Salida JSON para `--transcribe-file` / `--list-models` / `--list-devices` (con `audio_secs`, `load_ms`, `transcribe_ms`, `best_ms`, `rtf`)                                                                                                                       |

Ejemplo de benchmark reproducible:

```bash
escriba --transcribe-file prueba.opus --json --repeat 3
```

**Decisiones de diseño:**

- Las banderas solo valen para esa ejecución: NO modifican los ajustes guardados
- El control remoto va por `tauri_plugin_single_instance`: la segunda instancia pasa sus argumentos y termina
- `send_transcription_input()`, en `signal_handle.rs`, es común a las señales y a la línea de comandos

## Modo de depuración

Se activa con `Cmd+Shift+D` (macOS) o `Ctrl+Shift+D` (Windows/Linux). OJO: `debug_mode` es un ajuste PERSISTENTE que se envía a los usuarios, no una herramienta interna: lo que cuelga de ese panel son funciones reales de diagnóstico.

## Notas por plataforma

- **macOS**: aceleración Metal; los atajos de teclado necesitan permiso de Accesibilidad
- **Windows**: aceleración Vulkan, firma de código
- **Linux**: OpenBLAS + Vulkan, soporte limitado de Wayland; la ventana superpuesta usa GTK layer shell (se desactiva con `HANDY_NO_GTK_LAYER_SHELL=1`)

## Solución de problemas

Para problemas de compilación, ver [BUILD.md](BUILD.md). Para los permisos de la primera apertura en macOS y Windows, las notas de instalación del [README.md](README.md).

## Flujo de GitHub para asistentes de IA

**OBLIGATORIO. Antes de abrir un PR, una issue o una discusión en este repo: hay que leer la plantilla correspondiente y seguirla al pie de la letra.** Incluidas las secciones que parecen de trámite: listas de comprobación, declaración de uso de IA, "Human Written Description". Un resumen genérico con plan de pruebas no vale.

- **Abrir un PR:** lee [`.github/PULL_REQUEST_TEMPLATE.md`](.github/PULL_REQUEST_TEMPLATE.md). Todas sus secciones son obligatorias. Si alguna pide un párrafo escrito por una persona (por ejemplo "Human Written Description"), deja un TODO visible y pídeselo — no inventes su voz.
- **Abrir una issue:** lee [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/). Las issues en blanco están desactivadas; elige la plantilla que toque (`bug_report.md` para errores). Las peticiones de features no van en issues: van a [Discussions](https://github.com/AlejandroAP9/Escriba/discussions) (ver `.github/ISSUE_TEMPLATE/config.yml`).
- **Proponer una feature:** discútela primero en [Discussions](https://github.com/AlejandroAP9/Escriba/discussions). (El Handy original está congelado en features; esa política es suya, no de Escriba.)
- **Traducciones:** sigue [CONTRIBUTING_TRANSLATIONS.md](CONTRIBUTING_TRANSLATIONS.md).
- **Flujo completo para colaborar:** [CONTRIBUTING.md](CONTRIBUTING.md).

**Commits:** usa los prefijos convencionales (`feat:`, `fix:`, `docs:`, `refactor:`, `chore:`). El mensaje explica el _porqué_, no el _qué_.
