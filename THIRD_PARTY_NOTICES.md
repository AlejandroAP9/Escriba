# Avisos de terceros

Escriba se distribuye bajo licencia MIT (ver [LICENSE](LICENSE)). Este archivo
recoge el software y los modelos de terceros que Escriba **empaqueta, descarga o
instala**, con su licencia y su origen.

Para lo que no se empaqueta sino que se descarga en tiempo de ejecución, la
licencia es la del origen: aquí se dice qué es y de dónde sale, para que puedas
comprobarla antes de usarlo.

---

## Proyecto base

**[Handy](https://github.com/cjpais/Handy)** — © 2025 CJ Pais, licencia MIT.
Escriba es un rework de Handy. El texto completo de la licencia MIT que ampara
tanto a Handy como a Escriba está en [LICENSE](LICENSE) y viaja dentro del
paquete distribuido (`resources/LICENSE.txt`).

## Se empaqueta dentro de la app

| Componente                                                     | Para qué                                             | Origen                                                                                               |
| -------------------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| **Silero VAD** (`resources/models/silero_vad_v4.onnx`)         | Detectar cuándo estás hablando y cuándo hay silencio | [snakers4/silero-vad](https://github.com/snakers4/silero-vad)                                        |
| **Inter** (`src/assets/fonts/inter-*.woff2`)                   | Tipografía de la interfaz                            | [rsms/inter](https://github.com/rsms/inter), SIL Open Font License 1.1                               |
| **EB Garamond** (`src/assets/fonts/eb-garamond-*.woff2`)       | Tipografía editorial de titulares y de la marca      | [octaviopardo/EBGaramond12](https://github.com/octaviopardo/EBGaramond12), SIL Open Font License 1.1 |
| **ggml / transcribe-cpp**, **transcribe-rs**, **ONNX Runtime** | Motores de inferencia local                          | Ver `src-tauri/Cargo.toml`                                                                           |
| Dependencias de Rust y de npm                                  | —                                                    | Sus licencias respectivas, declaradas en `Cargo.lock` y `bun.lock`                                   |

## Se descarga desde la app

| Componente                                          | Cuándo                                | Licencia y origen                                                                                                                                                               |
| --------------------------------------------------- | ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Modelos de transcripción** (catálogo, 65 modelos) | Cuando eliges un modelo               | Varía por modelo, ver la tabla de abajo                                                                                                                                         |
| **Qwen3-4B-Instruct-2507** (motor de IA local)      | Al instalar el motor local            | Apache-2.0, [Qwen](https://huggingface.co/Qwen)                                                                                                                                 |
| **llama.cpp** (servidor del motor local)            | Al instalar el motor local            | MIT, [ggml-org/llama.cpp](https://github.com/ggml-org/llama.cpp)                                                                                                                |
| **sherpa-onnx** y voces Piper (voz del Intérprete)  | Al instalar la voz neural             | [k2-fsa/sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)                                                                                                                     |
| **BlackHole 2ch** (micrófono virtual)               | Solo si instalas el micrófono virtual | **GPL-3.0**, [ExistentialAudio/BlackHole](https://github.com/ExistentialAudio/BlackHole). Escriba descarga el instalador oficial y lo ejecuta; no lo redistribuye ni lo enlaza. |

## Licencias de los modelos del catálogo

Los datos salen de `src-tauri/src/catalog/catalog.json`, que los toma de la
ficha de cada repositorio en Hugging Face.

### ⚠️ Modelo con restricción de uso comercial

**`handy-computer/canary-1b-gguf`** (derivado de `nvidia/canary-1b`) es
**CC-BY-NC-4.0**: _no comercial_. Está en el catálogo y se puede descargar desde
la app, pero **no lo uses en un contexto comercial**. No es un modelo
recomendado ni viene seleccionado por omisión.

### Modelos que exigen atribución (`cc-by-4.0`, 14 modelos)

CC-BY-4.0 obliga a dar crédito al autor original. Afecta sobre todo a la familia
**Parakeet** y **Canary** de **NVIDIA**, y a modelos derivados de
`openai/whisper-*`. Si publicas algo hecho con ellos, cita el modelo base.

### Modelos con licencia `other` (7 modelos)

Estos no declaran una licencia estándar y hay que mirar su ficha en Hugging Face
caso por caso:
`nemotron-3.5-asr-streaming-0.6b`, `nemotron-speech-streaming-en-0.6b` (NVIDIA),
`Fun-ASR-MLT-Nano-2512`, `Fun-ASR-Nano-2512`, `SenseVoiceSmall` (FunAudioLLM),
`medasr` (Google).

### Resto del catálogo

**23 modelos Apache-2.0** (Qwen, Voxtral/Mistral, IBM Granite, Cohere, Breeze,
y los derivados de Whisper de OpenAI) y **21 modelos MIT**. Ambas licencias son
permisivas y no imponen condiciones más allá de conservar el aviso.

---

Si detectas una atribución que falta o una licencia mal declarada, abre un issue:
se corrige de inmediato.

---

## Proveedores remotos opcionales (BYOK)

Escriba **no envía nada por omisión**. Si activas un proveedor remoto para la
corrección con IA, se le manda el **texto transcrito**; el audio nunca sale del
equipo en ningún caso.

Qué hace cada proveedor con ese texto (si lo retiene, por cuánto, o si lo usa
para entrenar) **es su política, no la de Escriba**, y no se puede determinar
desde este código. Antes de configurar uno, léela:

| Proveedor               | Política de privacidad                     |
| ----------------------- | ------------------------------------------ |
| OpenAI                  | https://openai.com/policies/privacy-policy |
| Anthropic               | https://www.anthropic.com/legal/privacy    |
| Groq                    | https://groq.com/privacy-policy/           |
| OpenRouter              | https://openrouter.ai/privacy              |
| Cerebras                | https://www.cerebras.ai/privacy            |
| z.ai                    | https://z.ai                               |
| Proveedor personalizado | La del servidor que tú configures          |

Los motores **locales** (el motor de IA local de Escriba, Ollama y Apple
Intelligence) no envían nada a ninguna parte: corren en tu máquina.
