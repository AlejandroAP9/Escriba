# PRP-006: Escriba 2.3.0: español profundo, CLI headless, contexto y cifrado

> **Estado**: APROBADO (Alejandro, 5-ago-2026). Decisión de producto: emojis
> dictados APAGADOS por defecto, igual que numerales; anunciar en changelog y
> tip de descubrimiento.
> **Fecha**: 2026-08-05
> **Proyecto**: Escriba
> **Origen**: PLAN-POST-HACKATHON.md, fases 0 (0.1 y 0.2) + 1 + 3. El corte de
> release lo decide Alejandro explícitamente; los commits a main fluyen al ritmo
> del trabajo.

## Objetivo

Que Escriba 2.3.0 borre la razón de existir de Abrax (español profundo: tildes,
emojis dictados, numerales a cifras) y el eje CLI de Dictum (batch headless con
benchmarks reproducibles), y cierre la deuda propia aplazada: contexto
conversacional en el Traductor y el Intérprete, y cifrado en reposo del
historial.

## Por Qué

| Problema                                                                                                              | Solución                                                                                           |
| --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Whisper devuelve castellano sin tildes según el audio; Abrax (rival n°2) sí lo corrige                                | Mapa léxico determinista (TSV offline, solo formas inequívocas) siempre activo                     |
| Dictar un emoji obliga a parar el dictado y buscarlo con el mouse                                                     | "emoji cara feliz" → 🙂 desde las anotaciones es de CLDR + alias manuales                          |
| Planillas con muchos datos exigen cifras, no palabras (petición real de Juan Francisco Ceccarelli, 5-ago)             | Parser determinista + modo agresivo automático cuando la app al frente es una planilla             |
| "prueba" salió como _proof_ en una conversación escolar: cada frase se traduce en aislamiento total                   | Últimos 2-3 turnos como contexto en `translate_with_timeout` y `converse_translate`                |
| Los dictados guardados viven en claro en el disco (el propio código lo declara pendiente en `history.rs:196`)         | Cifrado por campo con llave en el llavero del SO                                                   |
| Una dupla rival publicó mediciones contra nosotros y no tenemos benchmarks reproducibles propios; Dictum sí los tiene | CLI batch universal (`--transcribe-file` con cualquier formato del Estudio) + `--json` documentado |

**Valor para el concurso/comunidad**: si esto se completa, ningún rival
conserva su eje lingüístico ni su eje CLI. Las mediciones públicas se vuelven
reproducibles por cualquiera (`--json --repeat`), y dos features salen de
peticiones reales de la comunidad.

## Qué

### Criterios de Éxito

Español profundo (Fase 1 del plan):

- [ ] Batería congelada de dictados difíciles en español (≥40 casos, audios
      reales) pasa **vía CLI sobre el motor real**, no solo en tests unitarios
      (la disciplina de las plantillas de 2.2.2).
- [ ] "el medico llego rapido y pidio quedarse" → "el medico llego rápido y
      pidió quedarse" sin LLM (rapido/pidio deterministas; medico/llego son
      formas válidas de medicar/llegar y quedan intactos POR DISEÑO: ver
      Aprendizajes 5-ago), en dictado, Sesiones y Estudio.
- [ ] Los pares ambiguos quedan intactos: "esta casa esta aqui" no cambia
      ninguna "esta"; "si", "mas", "aun", "practico", "hacia" tampoco.
- [x] "emoji cara feliz" → 🙂 con el interruptor activo; "me mandó un emoji"
      queda intacto.
- [x] "tres millones y medio" → 3.500.000 con numerales activos; "uno de los
      problemas" y "hora y media" quedan intactos.
- [ ] Con Excel/Numbers/LibreOffice Calc al frente y el auto-planilla activo,
      "cuarenta y dos coma cinco" → 42,5 sin encender nada a mano.
- [ ] CLDR y la fuente del lexicón de tildes declarados en
      THIRD_PARTY_NOTICES.md con licencia y origen.

CLI headless (Fase 3 del plan):

- [ ] `escriba --transcribe-file prueba.opus --json --repeat 3` corre sin GUI,
      sin micrófono, sin tocar los ajustes guardados, y emite `audio_secs`,
      `transcribe_ms`, `best_ms` y `rtf`.
- [ ] Un archivo corrupto o de formato no soportado termina con mensaje claro y
      código de salida 2, sin panic.
- [ ] La tabla de banderas de AGENTS.md y el README documentan TODAS las
      banderas (el README se ha olvidado 2 veces; esta vez no).

Contexto conversacional (Fase 0.1):

- [ ] En una conversación escolar con turnos previos sobre el colegio,
      "mañana tengo una prueba" se traduce como _test/quiz_, no _proof_.
- [ ] La batería del QA de Flor (dirección es↔en) pasa sin regresión: la
      detección de dirección sigue corriendo solo sobre la frase nueva.
- [ ] Apagar la escucha o cambiar el par de idiomas limpia el contexto
      (verificable: el turno siguiente no arrastra nada).
- [ ] Una instrucción maliciosa dictada en el turno 1 no altera la traducción
      del turno 2 (el contexto entra delimitado como referencia, no como orden).

Cifrado en reposo (Fase 0.2):

- [x] `strings history.db` no revela el texto de ningún dictado nuevo; las
      grabaciones nuevas en `recordings/` tampoco son WAV legibles.
- [ ] La 2.2.4 abre esa misma base sin crash (tolerancia de downgrade intacta,
      el incidente e59158d3 no se repite).
- [ ] Borrar la llave del llavero no impide arrancar ni dictar: las entradas
      ilegibles se marcan como no descifrables en la UI.
- [ ] Matar el proceso a mitad de la migración de texto claro a cifrado y
      relanzar completa la migración sin doble cifrado.

Transversales:

- [x] `bun run check:translations` pasa con las claves nuevas en los 21 idiomas.
- [ ] `cargo build` + `tsc` pasan; prueba reina: todo lo anterior con wifi
      apagado.

### Comportamiento Esperado

**Dictado en español**: hablas normal; si el motor devuelve "el medico
pregunto", en pantalla aparece "el médico preguntó". No configuraste nada: la
restauración corre siempre, en todos los caminos (dictado, Sesiones, Estudio,
CLI), igual que quedó unificado el pipeline en 2.2.2. Si dices "emoji pulgar
arriba" (con el interruptor activo) aparece 👍. Si activaste numerales y dictas
"doscientos treinta y cuatro mil quinientos", aparece 234.500. Si estás en una
planilla y el auto-planilla está activo, hasta un numeral suelto se vuelve
cifra.

**Traductor cara a cara**: la conversación fluye por turnos; cada traducción ve
los 2-3 turnos anteriores como contexto y elige la acepción correcta. Al cerrar
la sesión, el contexto muere en RAM.

**CLI**: `escriba --transcribe-file audio.opus --json --repeat 3` decodifica
igual que el Estudio (wav, mp3, m4a, opus, video), transcribe con el mismo
pipeline y las mismas correcciones que la app, imprime JSON con métricas y
termina. Sirve de arnés para la batería de español y para benchmarks públicos.

**Historial**: nada cambia a la vista. Por debajo, texto y audio nuevos se
escriben cifrados; el historial existente se migra al primer arranque.

## Contexto

### Referencias

Verificado leyendo el código (protocolo AGENTS.md), no por grep suelto:

- `src-tauri/src/managers/transcription.rs:1843` — `post_process_transcription_text`,
  el punto único de corrección (custom words → filler → question marks). Llamado
  desde los 3 caminos: 1112 (streaming), 1264 (batch parcial), 1629 (dentro de
  `transcribe()`, que también sirve al CLI y al Estudio). **Aquí se enchufan
  tildes, emojis y numerales, y llegan gratis a todos los caminos.**
- `src-tauri/src/audio_toolkit/text.rs` — módulo de corrección de texto:
  `apply_custom_words` (con `preserve_case_pattern` y `extract_punctuation`
  reutilizables), `filter_transcription_output`, `fix_spanish_question_marks`.
  Tests en español con la disciplina post-Diapasón (`no_devora_castellano...`).
- `src-tauri/src/actions.rs:178-281` — `frontmost_app()` (macOS `lsappinfo`,
  Windows `QueryFullProcessImageNameW`, Linux None) y `app_context_prompt()`:
  la detección por app de los Tonos, a reusar para el modo planilla.
- `src-tauri/src/actions.rs:2354` — `translate_with_timeout` (privada; callers:
  `translate_text` 2326 → Traductor/MCP, `translate_live` 2344 → Intérprete).
  `converse_translate` 2663 arma su propio prompt y detecta dirección en Rust
  (`detect_pair_language` 2502, fix QA Flor 18-jul).
- `src-tauri/src/commands/translator.rs` — estado del Traductor: 2 estáticos
  (LISTENING, LANGS). Los turnos hoy viven SOLO en React
  (`src/components/translator/TranslatorSettings.tsx:112`, últimos 30).
- `src-tauri/src/commands/conversation.rs:144` — `push_turn` + estático TURNS:
  el patrón de ring buffer de turnos en backend que el Traductor debe imitar.
- `src-tauri/src/commands/interpreter.rs:80` — `publish_translated`: original
  primero, traducciones en serie por idioma. El contexto del Intérprete son las
  últimas líneas publicadas.
- `src-tauri/src/managers/history.rs` — `history.db` + `recordings/` en el app
  data dir; `restrict_to_owner` (0700/0600); comentario en 196-201: "No
  sustituye al cifrado en reposo, que sigue pendiente". **Líneas 244-275: la
  tolerancia a una base MÁS NUEVA es un invariante ganado con un bucle de
  crashes real (e59158d3); el cifrado no puede romperlo.**
- `src-tauri/src/redaction.rs` — redacción (Luhn) ANTES de persistir; se
  mantiene como primera línea, el cifrado es la segunda.
- `src-tauri/src/lib.rs:411-656` — `run_headless_transcription`: **la Fase 3
  del plan ya está construida en su mayoría** (`--transcribe-file`,
  `--list-models`, `--list-devices`, `--repeat`, `--json` con audio_secs /
  transcribe_ms / best_ms / rtf, `--export-srt`). Lo que falta: (a) sin
  `--export-srt`, solo acepta WAV 16 kHz mono (538-557); (b) `--list-devices`
  ignora `--json`; (c) cero documentación en AGENTS.md/README.
- `src-tauri/src/studio/decode.rs:46` — `decode_to_16k_mono`: symphonia + ogg/
  libopus para .opus. El decode universal que `--transcribe-file` debe adoptar.
- `src-tauri/src/settings.rs:1143-1168` — merge idempotente de settings con
  defaults (el patrón obligatorio para campos nuevos); 945: siembra de prompts
  por id en instalaciones existentes.
- `src-tauri/Cargo.toml` — ya están: rusqlite (bundled), sha2, getrandom,
  regex, serde_json. NO están: keyring ni ningún AEAD.
- `THIRD_PARTY_NOTICES.md` — formato de declaración de terceros (CLDR y la
  fuente del lexicón entran aquí).
- CLDR annotations es: licencia Unicode-3.0 (se declara). Lexicón de tildes:
  fuente por decidir en la fase con licencia compatible declarada (candidatos:
  RLA-ES/hunspell es vía opción MPL, o lista de frecuencias con licencia libre).

### Arquitectura Propuesta

**Español profundo (backend)**

- Módulo nuevo `src-tauri/src/audio_toolkit/spanish.rs`:
  `restore_tildes()`, `apply_dictated_emojis()`, `spoken_numbers_to_digits()`.
  Datos en `src-tauri/resources/es/tildes.tsv` y `emojis.tsv`, generados
  offline por `scripts/gen-tildes.ts` y `scripts/gen-emojis.ts` (mapa staged
  auditable, revisable fila a fila; los scripts NO corren en build ni en
  runtime). Carga perezosa con `OnceLock`.
- Wiring en `post_process_transcription_text`: custom_words → filler/stutters →
  **tildes** (solo si el idioma efectivo del dictado es español) → **numerales**
  (si toggle o planilla) → **emojis** (si toggle) → `fix_spanish_question_marks`.
- Tildes: SOLO formas inequívocas (una única forma válida en español, p. ej.
  "medico"→"médico", "llego"→"llegó" NO entra porque "llego" existe). Match por
  token exacto, más largo primero, conservando mayúsculas y puntuación con los
  helpers existentes. Lo ambiguo queda para el LLM de post-proceso, que ya lo
  cubre cuando está activo.
- Emojis: patrón disparador "emoji " + nombre; match exacto por tokens contra
  tabla CLDR es normalizada (minúsculas y sin tildes, porque el dictado puede
  llegar sin ellas) + alias manuales ("carita feliz", "pulgar arriba"…).
- Numerales, 3 niveles: (1) secuencias largas no ambiguas por parser
  determinista de tokens (sin regex de corte); (2) numeral suelto solo vía LLM
  contextual (los prompts de post-proceso ya lo piden; no se duplica); (3) modo
  agresivo automático si `frontmost_app()` es planilla (lista cerrada:
  "Microsoft Excel", "Numbers", "LibreOffice Calc", "excel"/"soffice" en
  Windows; Google Sheets en navegador NO es detectable por nombre de app y se
  documenta la limitación). Formato es-CL: miles con punto, decimales con coma.
- Settings nuevos (default + merge): `dictated_emojis_enabled`,
  `spoken_numerals_enabled` (false por defecto, regla del plan),
  `numerals_spreadsheet_auto`. Frontend: 2-3 toggles en
  `src/components/settings/` con claves i18n en 21 locales.

**Contexto conversacional (backend, RAM solamente)**

- Ring buffer de turnos en `commands/translator.rs` (patrón TURNS de
  conversation.rs): push al emitir `translator-result`, clear en
  `translator_set_listening(false)` y `translator_set_langs`.
- `converse_translate` y `translate_with_timeout` ganan un parámetro
  `context: Option<String>` (bloque pre-formateado y delimitado como
  "conversación previa, solo referencia"). `translate_text` para MCP pasa
  `None`: la herramienta MCP es stateless y no cambia.
- Intérprete: la sala guarda las últimas 2-3 líneas (original + traducción por
  idioma) y `publish_translated` las pasa a `translate_live`. Clear al cerrar
  la sala.
- `detect_pair_language` NO recibe contexto: la dirección se decide con la
  frase nueva, como hasta hoy (invariante del QA de Flor).

**Cifrado en reposo**

- Crates nuevos: `chacha20poly1305` (RustCrypto puro, cero linkage C, sin
  riesgo de conflicto de símbolos ggml) y `keyring` (llavero de macOS /
  Credential Manager de Windows / secret-service en Linux).
- Módulo nuevo `src-tauri/src/history_crypto.rs`. Llave de 256 bits creada en
  el primer arranque y guardada en el llavero. La llave jamás se loguea.
- Texto: cifrado por CAMPO, en las columnas existentes, con prefijo de formato
  `esc1:` + base64(nonce ‖ ciphertext). Sin cambio de esquema. El prefijo es a
  la vez marcador de migración (fila sin prefijo → se cifra; con prefijo → no
  se toca: idempotente y re-ejecutable). `usage_daily` son números agregados y
  queda en claro a propósito (se documenta).
- Audio: contenedor cifrado por tramos (frames AEAD de 64 KiB) para que una
  Sesión de 1 hora se reproduzca en streaming sin cargarla entera en RAM.
- Frontera única: `HistoryManager` cifra en `save_entry`/`update_transcription`
  y descifra en `map_history_entry`; todos los consumidores (UI, export
  Obsidian, MCP, estadísticas) quedan intactos. La redacción (redaction.rs)
  sigue corriendo ANTES de cifrar.
- Sin llave disponible (llavero borrado/inaccesible): la app arranca, el
  dictado funciona, las entradas ilegibles se muestran como "no descifrable"
  con acción de purga. Fail-open para la función principal, fail-closed para
  los datos.

**CLI universal**

- `run_headless_transcription`: WAV 16 kHz mono sigue por el camino rápido
  actual; cualquier otro archivo pasa por `studio::decode::decode_to_16k_mono`
  (mp3, m4a, opus, ogg, flac, mp4/video). `--list-devices` honra `--json`.
- Documentación: tabla de banderas completa en AGENTS.md y README.

### Modelo de Datos

- `history.db`: **cero migraciones de esquema**. Formato de valor `esc1:` en
  `transcription_history.transcription_text` y `post_processed_text`. Una
  versión vieja abre la base y arranca (ve texto cifrado ilegible en entradas
  nuevas, pero vive): el invariante de downgrade se conserva porque no se toca
  ni el pragma ni las tablas.
- `recordings/`: archivos nuevos en contenedor cifrado propio (magic + frames
  AEAD); los WAV existentes se migran al primer arranque (transformación por
  archivo, idempotente por extensión/magic).
- `AppSettings`: 3 campos nuevos con `#[serde(default)]` + merge (patrón
  settings.rs:1143). Sin bump de `settings_schema_version` salvo que haga falta
  seed condicional.
- Recursos estáticos: `resources/es/tildes.tsv` (token→forma con tilde, solo
  inequívocos), `resources/es/emojis.tsv` (nombre normalizado→emoji, CLDR +
  alias). Empaquetados, sin red en runtime.

## Premortem (matar el proyecto en papel)

Entradas: `raiz blindajes` (matcher-includes-substring-falso-positivo,
catalogo-con-nombres-sucios, regex-lookahead-no-consumir,
no-imprimir-secrets-en-chat, verificar-antes-de-listo) + el historial propio del
repo (diccionario devorador 813a0275, bucle de crashes por downgrade e59158d3,
secuestro por dictado "Sabido y sin resolver" de 2.2.4).

| Amenaza (cómo se rompe)                                                                                        | Cómo la mata el diseño                                                                                                                                       | Cómo se verifica                                                                                                     |
| -------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| El mapa de tildes "corrige" pares ambiguos y devora castellano (ya pasó con el diccionario personal, 813a0275) | TSV solo con formas inequívocas, generado por script offline auditable con lista de exclusión explícita de pares ambiguos; mapa staged revisable fila a fila | Tests con "esta/si/mas/aun/practico/hacia" afirmando NO cambio + batería congelada vía CLI en motor real             |
| Match por substring dispara falsos positivos (blindaje matcher-includes)                                       | Emojis: token disparador + nombre por match exacto de tokens, más largo primero. Planilla: lista cerrada de nombres, match de palabra completa               | Tests trampa: "me mandó un emoji" intacto; app "Numbers" dispara, "NumbersLike" no                                   |
| El parser de numerales trunca texto al cortar el resto (blindaje regex-lookahead)                              | Parser por tokens que reconstruye el resto sin regex de `match.end()`                                                                                        | Property test: para toda entrada sin numeral, salida == entrada byte a byte                                          |
| Numerales convierten lo que no debía ("uno de los", "hora y media", fechas dictadas)                           | Apagado por defecto; nivel 1 solo secuencias numéricas largas no ambiguas; nivel 3 solo con planilla al frente                                               | Batería de frases trampa con el modo ENCENDIDO afirmando NO cambio                                                   |
| Cifrado + downgrade = bucle de crashes (incidente real e59158d3)                                               | Nada de SQLCipher: cifrado por campo con prefijo `esc1:` en columnas existentes, cero cambio de esquema; una 2.2.4 abre y arranca                            | Abrir la base cifrada con el binario 2.2.4 real: arranca, no corrompe, no repite el bucle                            |
| Pérdida de la llave = historial perdido en silencio o app muerta                                               | Llave en llavero del SO; sin llave la app arranca, marca entradas "no descifrable" y el dictado nuevo sigue; purga explícita disponible                      | Borrar la llave del llavero → arrancar, dictar, ver el estado marcado; nada crashea                                  |
| El llavero está atado a la firma de código (ya se perdió Accesibilidad una vez al cambiar el cert)             | Misma firma estable "Escriba Self-Signed" ya en uso; el riesgo queda documentado en BUILD.md: cambiar el cert invalida el acceso a la llave                  | Un build nuevo firmado con el cert actual lee la llave creada por el build anterior sin prompt                       |
| Crash a mitad de la migración texto claro → cifrado deja la base mixta                                         | Transformación por fila, idempotente (el prefijo `esc1:` es el marcador), re-ejecutable en cada arranque; nunca doble cifrado                                | Matar el proceso a mitad de una migración simulada → segundo arranque completa; conteo de filas con prefijo correcto |
| La llave o el texto claro se filtran a logs (blindaje no-imprimir-secrets)                                     | La llave no implementa Debug/Display útil y jamás se formatea; el texto claro no se loguea en el camino de cifrado                                           | Ejercitar el flujo con log Trace y grepear el log: cero material de llave, cero texto de dictados                    |
| El contexto del Traductor amplifica la inyección por dictado (el "Sabido y sin resolver" de 2.2.4)             | Contexto delimitado como referencia de solo lectura, tope de 3 turnos y de tamaño, y limpieza al apagar escucha o cambiar idiomas; vive solo en RAM          | Instrucción maliciosa dictada en turno 1 → turno 2 se traduce igual; stop/start deja contexto vacío                  |
| El contexto rompe la detección de dirección (regresión del QA de Flor: "traducía" es→es)                       | `detect_pair_language` corre SOLO sobre la frase nueva, en Rust; el contexto solo entra al prompt de traducción                                              | Re-correr la batería del QA de Flor con contexto cargado en el idioma contrario                                      |
| Un archivo malicioso/corrupto tumba el CLI                                                                     | Mismo `decode_to_16k_mono` que el Estudio ya expone (superficie ya existente, no nueva); todo error → mensaje + exit 2, sin panic                            | Archivo corrupto, truncado y con extensión mentirosa → exit 2 limpio, sin panic                                      |
| Sesión de 1 hora cifrada revienta la RAM al reproducir                                                         | Audio cifrado por frames AEAD de 64 KiB con descifrado en streaming                                                                                          | Reproducir una grabación larga cifrada midiendo memoria: sin pico proporcional al archivo                            |
| Claves i18n nuevas rompen los 21 idiomas                                                                       | Toda string nueva nace en `en/translation.json` y se replica; el check lo exige                                                                              | `bun run check:translations` en verde                                                                                |

## Blueprint (el ciclo de cultivo)

> Solo FASES. Las subtareas se generan al entrar a cada fase (bucle agéntico).
> Orden deliberado: el CLI va primero porque es el arnés con el que se valida
> todo el español profundo ("criterio de listo: batería en el motor real").

### Fase 1: CLI universal y documentado (cierra 3.1-3.4)

**Objetivo**: `--transcribe-file` acepta todo lo que acepta el Estudio vía
`decode_to_16k_mono`; `--list-devices` honra `--json`; tabla de banderas
completa en AGENTS.md y README.
**Validación**: `escriba --transcribe-file prueba.opus --json --repeat 3` sin
GUI ni micrófono ni cambios en ajustes; archivo corrupto → exit 2 sin panic.

### Fase 2: Batería de español difícil (el arnés)

**Objetivo**: batería congelada (audios reales + salidas esperadas + frases
trampa) ejecutable vía CLI contra el motor real; es el criterio de listo de la
Fase 1 del plan y el arnés de regresión de las 3 fases siguientes.
**Validación**: la batería corre en el motor real y FALLA si una salida cambia.

### Fase 3: Restauración de tildes (1.1)

**Objetivo**: `restore_tildes()` con TSV inequívoco generado offline, enchufado
en `post_process_transcription_text`, corriendo siempre en todos los caminos;
fuente y licencia declaradas en THIRD_PARTY_NOTICES.md.
**Validación**: batería + pares ambiguos intactos + caso "el medico llego
rapido" en motor real.

### Fase 4: Emojis dictados (1.2)

**Objetivo**: tabla CLDR es + alias manuales, interruptor propio en Ajustes con
i18n en 21 idiomas, match exacto por tokens.
**Validación**: "emoji cara feliz" → 🙂; "me mandó un emoji" intacto; CLDR
declarado en THIRD_PARTY_NOTICES.md; check:translations en verde.

### Fase 5: Numerales hablados a cifras (1.3)

**Objetivo**: parser determinista por tokens (nivel 1), integración con el LLM
contextual existente (nivel 2, sin duplicar), y modo planilla automático
reusando `frontmost_app()` (nivel 3). Interruptor apagado por defecto.
**Validación**: batería trampa con el modo encendido + caso Ceccarelli
("tres millones y medio" → 3.500.000) + planilla al frente convierte solo ahí.

### Fase 6: Contexto conversacional del Traductor y el Intérprete (0.1)

**Objetivo**: ring buffer de 2-3 turnos en backend (patrón conversation.rs),
`converse_translate` y `translate_with_timeout` con contexto delimitado,
limpieza en stop/cambio de idiomas/cierre de sala; MCP queda stateless.
**Validación**: caso "prueba" → _test_ con contexto escolar; QA de Flor sin
regresión; inyección en turno 1 no contamina turno 2; re-probar lo validado del
Traductor (la razón por la que esto se aplazó).

### Fase 7: Cifrado en reposo del historial (0.2)

**Objetivo**: llave en el llavero del SO, texto con formato `esc1:` por campo,
audio por frames AEAD, migración idempotente al arrancar, estado "no
descifrable" en la UI. Redacción sigue corriendo antes de cifrar.
**Validación**: `strings history.db` sin texto dictado; 2.2.4 abre la base sin
crash; borrar llave no mata la app; kill a mitad de migración es recuperable.

### Fase 8: Validación Final

- [x] `cargo build` + `tsc` pasan
- [ ] `bun run tauri dev` y ejercitar el flujo real (no solo compilar)
- [ ] Prueba reina: dictado + Traductor + CLI + historial con wifi apagado
- [ ] Criterios de éxito cumplidos, uno por uno, con evidencia
- [ ] Premortem re-verificado con evidencia (comando/test por fila)
- [x] `bun run check:translations` + lint + format en verde
- [ ] Blindajes nuevos capturados (`raiz blindar`)
- [x] CHANGELOG con créditos: features inspiradas en rivales lo dicen
      (cultura de transparencia del plan post-hackathon)
- [x] **NO se corta release**: se avisa "listo para cortar cuando digas" y se
      espera el corte explícito de Alejandro

## Estado de cierre (5-ago-2026)

Implementación de las fases funcionales en 6 commits (d5655012..HEAD), con el
contenedor de audio cerrado en la continuación del 8-ago-2026. Evidencia por
criterio:

**Cumplidos con evidencia automatizada:**

- Batería: 40 casos, 7 voces, sandbox portable con idioma pinneado; DOS
  pasadas completas 40/40 idénticas (determinismo verificado).
- Tildes: 157.895 pares; centinelas bloqueantes en el generador; tests de
  pares ambiguos intactos; criterio ajustado con honestidad (medico/llego
  fuera POR DISEÑO, ver Aprendizajes).
- Emojis: "emoji cara feliz" → 🙂 y trampas intactas (tests); CLDR declarado
  en THIRD_PARTY_NOTICES; apagado por defecto (decisión de Alejandro).
- Numerales: caso Ceccarelli (3.500.000) + 10 frases trampa intactas (tests);
  "mil gracias", "dos tres" y "ciento" huérfano cazados en el diseño.
- CLI: m4a por decode universal validado contra el motor real; corrupto →
  exit 2 sin panic; --list-devices --json; AGENTS.md y README con TODAS las
  banderas; 3 fallas de gramática del parser encontradas y matadas en seco.
- Cifrado de texto y audio: texto con roundtrip, llave equivocada, cuerpo
  corrupto y prefijo idempotente; audio ESCAUD1 con XChaCha20-Poly1305 en
  frames de 64 KiB, autenticación de corrupción, lectura cruzando frames,
  Range acotado, traversal bloqueado y recuperación de migración interrumpida
  (6 tests de audio). Suite completa: 184 tests del backend en verde; build
  frontend + eslint + check:translations (21 idiomas) en verde.

**Pendiente de QA manual (necesitan GUI, motor LLM vivo o presencia):**

- Planilla real al frente (Excel/Numbers) convirtiendo numerales al pegar.
- Caso escolar del Traductor ("prueba" → _test_), batería de dirección de
  Flor, e inyección turno 1 → turno 2, con el motor local corriendo.
- Repetir `strings history.db` sobre una instalación personal real (el sandbox
  portable ya pasó); abrir la base cifrada con el binario 2.2.4 real; borrar la
  llave del llavero y verificar el marcador en la UI.
- Matar el proceso real a mitad de una migración grande de texto. El estado
  intermedio equivalente está cubierto por prefijo y por test, pero aún no se
  hizo el kill manual del proceso.
- Prueba reina con wifi apagado; `bun run tauri dev` ejercitando los flujos.

**Sin corte de release**: los 6 commits están en main; el corte lo decide
Alejandro.

## Continuación (8-ago-2026): audio cifrado cerrado

- Contenedor `ESCAUD1`: magic + largo claro + nonce aleatorio; XChaCha20-
  Poly1305 con subllave derivada y un tag por frame de 64 KiB.
- Escritura directa WAV→AEAD: no existe un WAV temporal claro. Publicación con
  tempfile en la misma carpeta + rename atómico + permisos 0600.
- Reproducción mediante protocolo privado `escriba-audio`, con HTTP Range y
  respuesta máxima de 512 KiB. React ya no recibe rutas ni tiene permiso de
  filesystem; se retiraron `plugin-fs`, asset protocol y su scope. El handler
  solo acepta la webview principal y archivos referenciados por una fila viva
  del historial, además de rechazar traversal y nombres no canónicos.
- Migración por archivo reanudable: valida el destino completo antes de borrar
  el WAV, recupera el estado "cifrado publicado / DB aún vieja" y cambia la
  fila a `.escaudio` sin migración de esquema.
- Re-transcripción consume un lector descifrado streaming; llave ausente o tag
  corrupto fallan cerrados y la UI muestra "Audio no disponible".
- Evidencia real en sandbox portable con `bun x tauri dev`:
  `escriba-1.wav` → `escriba-1.escaudio`, magic `ESCAUD1`, texto fixture
  ausente de `strings history.db`, WAV claro ausente. Segundo arranque:
  SHA-256 idéntico `9fa8a3e4…80692500f`, una sola fila cifrada.
- Validación: 184/184 tests Rust, `cargo check --tests`, `cargo clippy
--all-targets`, `bun run build`, eslint, Prettier y 21 locales en verde.

## Aprendizajes (Self-Annealing)

### 2026-08-05: bindings.ts editado a mano ocultaba un desfase real con Rust

- **Error**: `process_typed_text` ganó `target_lang: Option<String>` en Rust,
  pero el `bindings.ts` editado a mano seguía declarando 2 argumentos.
  `TypedTextInput.tsx` compilaba contra la firma vieja (funcionaba en runtime
  solo porque tauri trata el Option ausente como None). Al correr el binario
  debug, specta regeneró bindings y `tsc` destapó el desfase.
- **Fix**: call site con `null` explícito y comentario del porqué; bindings
  regenerado commiteado.
- **Aplicar en**: después de tocar cualquier comando, correr el binario debug
  una vez y dejar que specta regenere ANTES de commitear; el bindings a mano
  solo para hotfixes sin toolchain.

### 2026-08-05: la batería heredaba los ajustes personales del usuario

- **Error**: la primera congelada salió con el diccionario personal de la
  instalación convirtiendo "escrita" → "Escriba" (TIL-05): el CLI usa los
  ajustes guardados por diseño, así que el arnés no era comparable entre
  máquinas. De paso destapó un caso real límite del rescate fonético de
  2.2.4 (distancia 1: escrita/escriba), QUEDA PENDIENTE evaluar un guard de
  validez para el rescate.
- **Fix**: la batería corre en MODO PORTABLE en un sandbox propio (hardlink
  del binario + marcador `portable`): ajustes de fábrica siempre.
- **Aplicar en**: cualquier arnés futuro que use el CLI: sandbox portable
  primero.

### 2026-08-05: cabeceras vs reglas en el parser del .aff (gen-tildes)

- **Error**: una regla con add="0" ("SFX E r 0 [ae]r") se confundía con
  cabecera y RESETEABA la clase, borrando reglas ya leídas: "llegó" se
  generaba pero "llego" no, y el mapa incluía el par PELIGROSO llego→llegó.
- **Fix**: cabecera = exactamente 4 campos con Y/N + conteo numérico. Y los
  centinelas del generador ahora son bloqueantes (exit 1 si fallan).
- **Aplicar en**: cualquier parser de formatos "posicionales con variantes";
  nunca clasificar por un solo campo.

### 2026-08-05: el largo mínimo del mapa escondía las joyas de 3 letras

- **Error**: LARGO_MINIMO=4 excluía "dia"→"día", "ahi"→"ahí", "aca"→"acá"
  (restauraciones únicas y frecuentísimas). Los monosílabos diacríticos
  temidos (él/sí/qué/más) ya quedaban fuera por la regla de validez sola.
- **Fix**: umbral en 3; centinelas dia/ahi/aca agregados al generador.
- **Aplicar en**: al poner cinturones "por si acaso", verificar qué excluyen
  de verdad antes de confiar en la intuición.

### 2026-08-05: criterio del PRP ajustado con honestidad (medico/llego)

- El criterio original esperaba "el medico llego rapido y pidio quedarse" →
  todo acentuado sin LLM. Bajo la regla estricta (forma desnuda válida =
  fuera), "medico" (yo medico) y "llego" (yo llego) NO son restaurables por
  esta capa: quedan como los emita el motor (que casi siempre acierta) o
  para el LLM. El criterio pasa a: "rapido"→"rápido" y "pidio"→"pidió"
  deterministas; medico/llego intactos por diseño. "musica" y "publica"
  también quedan fuera (musicar y publicar existen).

### 2026-08-05: alcance de la Fase 7 acotado con honestidad (texto sí, audio pendiente)

- El cifrado en reposo quedó completo para el TEXTO (cifrar al guardar,
  descifrar en la frontera única, migración idempotente por prefijo, llave en
  el llavero, fail-open sin llave). El contenedor de AUDIO por frames AEAD
  queda PENDIENTE a propósito: el guardado de audio viene APAGADO de fábrica
  desde 2.2.4 (pocos usuarios afectados) y su reproducción en streaming exige
  diseñar la integración con el asset scope del webview (2.2.3) con calma,
  no a la carrera. El criterio "recordings no son WAV legibles" pasa a
  pendiente declarado.
- Los tests de cifrado JAMÁS tocan el llavero real: núcleos con llave
  inyectada (`cifrar_con`/`leer_con`). Un test que cree la llave de
  producción desde el binario de test siembra prompts de Keychain (la firma
  del test no es la de la app): el premortem "llavero atado a la firma"
  aplicaba a los tests mismos.

### 2026-08-08: el asset protocol no era la frontera correcta para audio cifrado

- **Problema**: `convertFileSrc` y el fallback `readFile` de Linux entregaban a
  la webview una ruta o el archivo completo. Lo primero no puede descifrar un
  contenedor; lo segundo duplica en RAM una sesión larga y exigía permiso de
  filesystem sobre `recordings/`.
- **Fix**: protocolo Tauri dedicado con Range, frames AEAD direccionables y
  tope de 512 KiB por respuesta. El mismo cambio elimina el permiso fs de la
  webview, así que una inyección ya no puede leer grabaciones directamente.
- **Aplicar en**: cualquier asset sensible grande debe cruzar una API de rango
  y propósito único; nunca ampliar el scope general de archivos para que un
  control HTML pueda abrirlo.

### 2026-08-05: build de audiopus_sys falla con install BSD (entorno sandbox)

- **Error**: el `make install` vendored de `audiopus_sys` usa flags GNU de
  `install` y el PATH del shell sandbox solo tiene el BSD de macOS → Error 64.
- **Fix**: `LIBOPUS_LIB_DIR=/opt/homebrew/opt/opus/lib LIBOPUS_STATIC=1`
  enlaza el opus de Homebrew y salta el build vendored por completo.
- **Aplicar en**: cualquier build de Escriba en entornos sin coreutils GNU;
  no cambia nada del repo (es solo entorno).

## Gotchas

- [ ] `src/bindings.ts` es de tauri-specta y solo se regenera en builds de
      depuración; fuera de ahí se edita a mano (AGENTS.md)
- [ ] Strings en JSX prohibidos por ESLint: todo por i18next, 21 locales,
      `bun run check:translations` falla si falta una clave
- [ ] Settings nuevos: default + merge idempotente (settings.rs:1143), nunca un
      campo sin default para instalaciones existentes
- [ ] La tolerancia a base más nueva (history.rs:244-275) es un invariante
      pagado con un bucle de crashes real: leer ese comentario antes de tocar
      migraciones
- [ ] El dictado en el CLI pasa por `transcribe()` y por lo tanto por TODAS las
      correcciones nuevas: la batería mide el pipeline completo, no el motor solo
- [ ] Los turnos del Traductor hoy viven solo en React; el contexto debe nacer
      en backend o no existirá para `converse_translate`

## Anti-Patrones

- NO agregar crates que linkeen ggml (conflicto de símbolos con transcribe-cpp)
- NO SQLCipher ni cambio de formato de la base: rompe el downgrade que ya costó
  un bucle de crashes
- NO llamadas de red en el camino feliz (100% local; los TSV van empaquetados)
- NO `.includes()`/substring para matchear nombres o tokens: match exacto, más
  largo primero (blindaje matcher-includes)
- NO regex que consuma para cortar el resto del texto (blindaje regex-lookahead)
- NO imprimir la llave ni texto claro en logs (blindaje no-imprimir-secrets)
- NO editar `src/bindings.ts` a mano en builds de depuración (se regenera)
- NO strings hardcodeados en JSX (i18next + 21 locales)
- NO settings nuevos sin default + merge
- NO `unwrap()` en producción
- NO declarar "listo" sin correr la verificación real (blindaje
  verificar-antes-de-listo)

_PRP implementado y continuado sin cortar release. El corte sigue requiriendo
la orden explícita de Alejandro._
