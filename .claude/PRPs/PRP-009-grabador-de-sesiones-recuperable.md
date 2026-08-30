# PRP-009: Grabador de sesiones recuperable (SessionRecorder)

> **Estado**: APROBADO (Alejandro, 30-ago-2026, sobre 70d25ccd)
> **Fecha**: 2026-08-30 (ajustes de la revisión de Alejandro del mismo día)
> **Proyecto**: Escriba
> **Origen**: análisis de `reunion-local` (flopez1977, MIT, repo nacido el
> 29-ago-2026) del 30-ago-2026. Se toma la idea (journal de estado + conservar
> el audio), no el código: su stack (Python/Torch/pyannote/mlx-whisper) no
> encaja en un Tauri distribuible. Si en alguna fase se porta código
> sustancial, se conserva su aviso de copyright y se agrega el crédito con el
> estándar de la casa.

## Objetivo

Que una sesión de Escriba (reunión, clase, entrevista) sobreviva a un cierre
inesperado: al reabrir la app, los turnos ya transcritos se recuperan y el
audio de ambas pistas queda guardado cifrado para reprocesarlo, con una
política de retención que no llena el disco de nadie.

## Por Qué

| Problema                                                                                                                                                        | Solución                                                                    |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| Los turnos viven solo en RAM (`static TURNS`, conversation.rs:65). Un crash de una reunión de 1 h se lleva todo.                                                | Journal append-only en disco; cada turno se persiste al llegar.             |
| El documento final solo existe en React tras `conversation_finish` (conversation.rs:1302 → ConversationSettings.tsx:669). Un crash post-acta también lo pierde. | El documento se cifra al journal ANTES de considerar la sesión cerrada.     |
| El audio se drena y se descarta (system_audio.swift:68). Si la transcripción sale mal, no hay nada que reprocesar.                                              | Ambas pistas se graban cifradas en un contenedor incremental (ESCAUD2).     |
| El VAD descarta lo que clasifica como no-voz, incluida habla que clasificó mal.                                                                                 | Se graba la señal **antes** del VAD: la pasada final no hereda sus errores. |
| `STARTED` es un `Instant` (monótono): no se puede serializar ni reconstruir tras reinicio.                                                                      | Ancla doble hora de pared + monótono, escrita en el journal al arrancar.    |

**Valor para la comunidad**: la promesa de Sesiones ("habla una hora y llévate
un documento") hoy tiene un asterisco invisible: _salvo que algo falle_. Esto
lo quita. Es además el prerrequisito de captura dual, integridad, doctor y
diarización (cola acordada el 30-ago).

## Qué

### Criterios de Éxito

- [ ] `kill -9` a Escriba a mitad de una sesión → al reabrir, la app ofrece
      recuperar; los turnos previos aparecen completos con sus `mm:ss`.
- [ ] `kill -9` DESPUÉS de generar el acta pero antes de que el usuario la
      guarde → el documento se recupera del journal, no solo los turnos.
- [ ] La última línea del journal rota por el kill no impide recuperar las
      anteriores (cola incompleta se descarta en silencio, se loguea).
- [ ] Los `.escaud2` de ambas pistas truncados por el kill se recuperan hasta
      el último frame AEAD válido y se pueden reproducir/re-transcribir, sin
      reescribir ningún frame existente.
- [ ] Apagar y reencender el micrófono a mitad de sesión NO comprime el
      tiempo: la pista registra offset de inicio y huecos, y la alineación
      con la pista del sistema se conserva (test con huecos artificiales).
- [ ] En disco no existe jamás audio claro ni texto claro de la sesión: cada
      línea del journal es UN evento JSON completo cifrado (`esc1:`), el audio
      va en ESCAUD2. `grep` de una frase dictada sobre `sessions/` → 0 hits.
- [ ] Sin llave del keyring (o sin CSPRNG) → la persistencia NO se activa como
      unidad: ni directorio, ni journal, ni pistas; la sesión funciona como
      hoy (RAM) y la UI lo dice una vez (fail-closed, API estricta).
- [ ] Con < umbral de disco libre al arrancar la sesión → aviso; si se agota
      durante: el audio se detiene con aviso, el journal (KB) continúa.
- [ ] Todo descarte de audio por presión (canal lleno, disco) queda registrado
      en el journal con el rango de muestras perdido, y la pista se marca
      incompleta: la UI jamás la presenta como cobertura íntegra.
- [ ] `session_recovery_discard(id)` solo acepta IDs aleatorios del formato
      esperado, resuelve la ruta y verifica que quede contenida bajo
      `sessions/`, y rechaza symlinks y `..` (tests de traversal incluidos).
- [ ] Retención aplicada: default `al_generar`; opciones 7 días, 30 días,
      siempre. Sesiones interrumpidas bajo `al_generar` tienen **7 días de
      gracia de recuperación** antes del barrido, explicado en la UI. Ajuste
      nuevo con default + merge.
- [ ] Descartar una recuperación ofrecida elimina journal y audio de esa
      sesión (verificable en disco).
- [ ] `cargo test` con casos de: línea rota, frame truncado, ancla de reloj,
      huecos de pista, recuperación re-ejecutable (2× = mismo estado),
      traversal en discard, canal saturado sin bloqueo.
- [ ] i18n: toda cadena nueva en 21 idiomas, `bun run check:translations` verde.

### Comportamiento Esperado

1. El usuario arranca una sesión. Si el cifrado estricto está disponible,
   Escriba crea `sessions/<id>/` con `journal.jsonl` (ancla de reloj + modo
   como primera línea) y, si hay pista(s) activas, `mic.escaud2` y/o
   `sys.escaud2`. Si no, nada de esto existe y la sesión va en RAM como hoy.
2. Cada turno transcrito se apendea al journal al llegar (evento JSON completo
   cifrado por línea). El audio crudo (pre-VAD) de cada pista se apendea por
   frames a su contenedor, con offset de inicio y huecos registrados.
3. La sesión termina bien → se genera el documento como hoy y se cifra al
   journal (`documento{...}`) SIN cerrar: generar el acta no prueba que el
   usuario la recibió. `cierre{documento}` lo escribe solo la confirmación
   explícita del frontend (comando de la Fase 2); `cierre{descarte}` el reset
   del usuario, que además borra la carpeta. Todo journal con `documento` y
   sin `cierre` es oferta obligada de la recuperación. Según retención, el
   audio se borra o se conserva.
4. La sesión muere mal (crash, kill, apagón) → al siguiente arranque Escriba
   detecta el journal sin `cierre` y ofrece: **Recuperar** (turnos —y
   documento si ya existía— a la pantalla de Sesiones, audio disponible para
   regenerar), **Exportar** (documento con lo que hay) o **Descartar** (borra
   todo, con validación de ruta).

## Contexto

### Referencias

- `src-tauri/src/commands/conversation.rs:65` — `static TURNS: Mutex<Vec<Turn>>`,
  cero escrituras a disco en todo el archivo. `STARTED` es `Instant`.
- `src-tauri/src/commands/conversation.rs:1302` + `src/components/conversation/ConversationSettings.tsx:669`
  — `conversation_finish` devuelve el `SessionDoc` al frontend y ahí muere el
  rastro: el documento final nunca toca el backend de nuevo.
- `src-tauri/src/managers/transcription.rs:126` — **OJO**: `open_tap()`
  entrega frames "ya filtrados por el VAD". El camino de manos libres NO sirve
  como punto de tee pre-VAD (error de la primera versión de este PRP).
- `src-tauri/src/audio_toolkit/audio/recorder.rs:604` — `handle_frame` es
  donde entra el VAD. El tee del micrófono va en el `AudioRecorder`, después
  del resample a 16 kHz y ANTES de `handle_frame`.
- `src-tauri/src/commands/conversation.rs:897` — worker del audio del sistema:
  `system_audio::read(&mut chunk)` llena `pending` **antes** del VAD. Punto de
  tee de la pista del sistema (en Rust tras el read; el ring de Swift tiene su
  contrato de drenar-y-borrar y no se toca).
- `src-tauri/src/recording_crypto.rs:111` — **restricción dura**: `aad_for`
  autentica `header.plaintext_len` en el AAD de CADA frame. ESCAUD1 no admite
  escritura incremental (reescribir la cabecera invalida todos los tags, y el
  lector rechaza longitud 0). De ahí ESCAUD2. Se reutilizan llave derivada,
  XChaCha20-Poly1305, esquema de nonces (prefijo + índice) y lectura por
  frames; el formato se versiona con magic propio.
- `src-tauri/src/history_crypto.rs:103` — **restricción dura**: `cifrar_campo`
  degrada a texto claro sin llave o sin CSPRNG (fail-open correcto para
  dictado, documentado en el propio código). El journal necesita
  `cifrar_campo_estricto() -> Result<String>`, que jamás devuelve claro.
- `src-tauri/src/managers/history.rs:295-311` — patrón de migración
  re-ejecutable (`migrate_legacy_wav`): el modelo para una recovery idempotente.
- Blindajes transferidos (raiz): `webhook-idempotencia-atomica` (recovery
  re-ejecutable), `patron-frontera-de-confianza-server-fail-closed` (sin
  llave → no escribir, jamás degradar a claro).
- Idea de origen: `github.com/flopez1977/reunion-local` — `estado.py`
  (journal), conservación de WAV. Un solo commit, sin tags: referencia, no
  dependencia.

### Arquitectura Propuesta

**Backend (Rust), módulo nuevo `session_recorder.rs`:**

- `SessionRecorder` residente (patrón manager: `Arc<Mutex<Option<...>>>`),
  creado al arrancar sesión SOLO si el cifrado estricto responde; activación
  todo-o-nada: sin garantía de cifrado no se crea directorio, journal ni
  pistas. Dos responsabilidades: journal y pistas; nada de transcribir.
- **Journal** `journal.jsonl`: una línea por evento, y la línea entera es el
  evento JSON completo cifrado con `cifrar_campo_estricto` (no solo `text`:
  role, at_ms y metadatos también son contenido de la reunión). Eventos:
  `inicio{wall_ms, mono_anchor, modo, version}`, `turno{role, text, at_ms}`,
  `pista{track, evento: inicio|hueco|corte, at_ms, muestras_perdidas?}`,
  `documento{doc, animo, at_ms}`, `cierre{motivo: documento|descarte}`.
  Append con `write + flush`; fsync en `inicio`, `documento`, `cierre` y cada
  N turnos (medir en Fase 1; N=1 si el costo lo permite).
- **Reloj**: en `inicio` se guarda el par (SystemTime en ms, offset monótono
  0). Cada evento lleva `at_ms` monótono desde el ancla. La recuperación
  reconstruye `mm:ss` sin necesitar el `Instant` perdido.
- **`cifrar_campo_estricto()` en `history_crypto`**: `Result<String>` que
  falla si no hay llave, si no hay CSPRNG o si el AEAD falla. `cifrar_campo`
  (fail-open) queda intacto para el historial de dictado.
- **Contenedor ESCAUD2** en `recording_crypto`, compatible hacia atrás
  (ESCAUD1 se sigue leyendo; nada existente se migra):
  - cabecera sin longitud final (magic `ESCAUD2` + versión + prefijo de nonce);
  - frames autocontenidos: índice y longitud del frame autenticados en su
    propio AAD, sin referencia al total;
  - footer opcional de cierre limpio (conteo total autenticado);
  - recovery: verificar frames en orden y descartar la cola incompleta, SIN
    reescribir ningún frame existente; sin footer → longitud = suma de frames
    válidos;
  - el lector sintetiza la cabecera WAV en memoria a partir de esa suma.
- **Tee de pistas**: micrófono en `AudioRecorder` (post-resample 16 kHz,
  pre-`handle_frame`); sistema tras `system_audio::read`. Ambos publican por
  `sync_channel` + `try_send` a **workers separados** (audio y journal nunca
  comparten cola: un disco bloqueado en audio no puede impedir persistir el
  evento de degradación). `try_send` fallido → se descarta el audio nuevo, se
  registra `pista{hueco, muestras_perdidas}` en el journal y la pista queda
  marcada incompleta. El camino de transcripción no espera al disco jamás.
- **Offsets y huecos**: cada pista registra su `at_ms` de inicio y cada
  hueco (mic apagado/encendido, descarte por presión). La reconstrucción
  inserta silencio en los huecos: nunca se concatena PCM comprimiendo tiempo.
- **Recovery al arrancar**: escanear `sessions/*/journal.jsonl` sin `cierre` →
  evento al frontend con resumen (fecha, modo, nº turnos, duración, ¿hay
  documento?). Comandos tauri-specta: `session_recovery_list`,
  `session_recover(id)`, `session_recovery_discard(id)`. Los IDs son
  aleatorios generados por Escriba; todo comando valida formato, resuelve la
  ruta y exige contención bajo `sessions/` (symlinks y `..` rechazados).
- **Retención**: ajuste `session_audio_retention` (enum `al_generar` |
  `dias_7` | `dias_30` | `siempre`, default `al_generar`) + barrido al
  arrancar. Interrumpidas bajo `al_generar`: 7 días de gracia de recuperación
  antes del barrido, dicho en la UI. Umbral de disco: aviso < 2 GB al
  arrancar; corte de audio con aviso si se agota (journal sigue).

**Frontend:**

- Diálogo de recuperación al arrancar (evento → modal en Sesiones): tres
  botones, resumen de qué se recupera (incluido si hay documento). Plumín en
  pose de guía.
- Sección nueva en Ajustes (grupo Sesiones): retención + estado del cifrado +
  aviso de gracia de 7 días.
- i18n: claves nuevas vía `scripts/add-i18n-keys.ts`, 21 idiomas.

**Qué NO toca**: el pipeline de transcripción en vivo, el VAD, los prompts de
actas, el Intérprete, `cifrar_campo` fail-open del historial, ESCAUD1 y sus
grabaciones existentes. El recorder observa; no altera el camino existente.

### Modelo de Datos

Sin SQLite nuevo: el filesystem es el modelo (un directorio por sesión con ID
aleatorio, igual que las grabaciones del historial). `AppSettings` gana
`session_audio_retention` (String enum) y `session_recorder_enabled` (bool,
default `true`), ambos con `#[serde(default)]` + merge idempotente (patrón
settings.rs:1143).

## Premortem (matar el proyecto en papel)

| Amenaza (cómo se rompe)                                                         | Cómo la mata el diseño                                                                                       | Cómo se verifica                                                                   |
| ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------- |
| Kill a mitad de línea del journal → JSON roto → recovery revienta o pierde todo | jsonl append-only; el parser tolera cola incompleta: descarta la última línea si no descifra/parsea, sigue   | test con archivo truncado en mitad de línea → recupera N-1 turnos                  |
| Kill a mitad de frame ESCAUD2 → contenedor corrupto                             | frames autocontenidos: recovery descarta la cola incompleta sin reescribir nada; sin footer → suma de frames | test: truncar a bytes arbitrarios → recovery deja archivo reproducible             |
| Reescritura de cabecera invalida tags (el fallo que mataba a ESCAUD1)           | ESCAUD2 no autentica el total en los frames; el total vive solo en el footer opcional                        | test: archivo sin footer se lee; footer inconsistente → se ignora con warn         |
| Crash tras generar el acta pero antes de guardarla → acta perdida               | `documento{}` se cifra al journal ANTES de `cierre`; `cierre` = durable o descarte, nunca "paró la captura"  | test: journal con `documento` sin `cierre` → recovery entrega el acta              |
| Recovery corre dos veces (doble arranque rápido, crash durante recovery)        | recovery idempotente estilo `migrate_legacy_wav`: verifica-antes-de-tocar, re-ejecutable (blindaje raiz)     | correr recovery 2× en test → estado idéntico, sin duplicar turnos                  |
| Keyring sin llave o sin CSPRNG                                                  | activación todo-o-nada con `cifrar_campo_estricto` (Result): sin garantía no existe ni el directorio         | test con llave ausente → cero archivos creados; test de la API estricta → Err      |
| `cifrar_campo` fail-open filtra texto claro al journal                          | el journal SOLO usa la API estricta; la fail-open queda para el historial de dictado, donde es correcta      | grep de frase dictada sobre `sessions/` → 0 hits                                   |
| `session_recovery_discard` con ID hostil borra fuera de `sessions/`             | IDs aleatorios validados por formato + canonicalize + contención bajo `sessions/` + rechazo de symlink/`..`  | tests de traversal: `../`, symlink, ID inventado → Err, disco intacto              |
| Mic apagado/encendido comprime el tiempo y desalinea las pistas                 | offsets y huecos registrados por pista; la reconstrucción inserta silencio, jamás concatena                  | test con huecos artificiales → duración total correcta, alineación estable         |
| Disco lleno a la hora de reunión (≈115 MB/h por pista en PCM16 ×2)              | chequeo al arrancar (aviso <2 GB) + corte de audio con aviso si se agota; journal (KB) nunca se corta        | simular disco lleno → sesión sigue, audio parado, evento `pista{corte}` en journal |
| El disco lento bloquea la transcripción en vivo                                 | `sync_channel` + `try_send`, workers separados para journal y audio; descarte registrado, jamás espera       | test de canal saturado → productor no espera; evento de hueco persiste igual       |
| Pista incompleta presentada como cobertura íntegra                              | cada descarte registra rango perdido y marca la pista; la UI muestra el estado incompleto                    | test: sesión con huecos → el resumen de recuperación lo declara                    |
| Huérfanas acumuladas llenan el disco con el tiempo                              | barrido al arrancar con gracia de 7 días para interrumpidas bajo `al_generar`; descartadas → borrado ya      | crear huérfana >7 días artificial → arranque la limpia; <7 días → la conserva      |
| Cambio de hora del sistema a mitad de sesión rompe los `mm:ss`                  | offsets monótonos contra ancla; la hora de pared solo etiqueta el inicio                                     | test: eventos con ancla fija → `mm:ss` estables ante wall alterado                 |
| La webview ve rutas del audio de sesión                                         | mismo protocolo privado de `recording_crypto` (lib.rs:858); ningún comando devuelve rutas                    | revisión de bindings generados: ninguna ruta en tipos expuestos                    |

## Blueprint (el ciclo de cultivo)

> Solo FASES. Las subtareas se generan al entrar a cada fase (bucle agéntico).

### Fase 1: Reloj, API estricta y journal

**Objetivo**: `cifrar_campo_estricto` en history_crypto; toda sesión escribe
`inicio` + turnos + `documento` + `cierre` cifrados en `journal.jsonl`, con
ancla wall+monótono y activación todo-o-nada. Sin UI nueva aún.
**Validación**: sesión real → archivo existe, líneas descifrables en test,
grep de texto claro = 0, sin llave → cero archivos. Medir costo de fsync.

### Fase 2: Recuperación

**Objetivo**: journals sin `cierre` se detectan al arrancar y el diálogo
Recuperar/Exportar/Descartar funciona de punta a punta, documento incluido.
Discard con validación de ruta completa. Incluye el comando de confirmación
del frontend que escribe `cierre{documento}` (sin él, ningún journal con
acta se cierra jamás y toda sesión terminada reaparecería como recuperable).
**Validación**: `kill -9` en sesión real → reabrir → recuperar → turnos con
sus `mm:ss` (y el acta si existía). Recovery 2× = mismo estado. Tests de
traversal en discard. Descartar borra en disco.

### Fase 3: Contenedor incremental ESCAUD2

**Objetivo**: formato versionado con frames autocontenidos, footer opcional,
recovery por descarte de cola sin reescritura, lector con cabecera WAV
sintetizada. ESCAUD1 se sigue leyendo intacto.
**Validación**: truncar a bytes arbitrarios → siempre recupera reproducible;
suite propia de round-trip, footer ausente/inconsistente, compatibilidad
ESCAUD1.

### Fase 4: Tee de las dos pistas

**Objetivo**: micrófono (AudioRecorder, post-resample, pre-`handle_frame`) y
sistema (tras `system_audio::read`) fluyen por `sync_channel`+`try_send` a
workers separados, con offsets y huecos registrados. La recuperación ofrece
re-transcribir desde el audio.
**Validación**: sesión con ambas pistas → dos `.escaud2` reproducibles y
alineados; huecos artificiales → silencio insertado, no compresión; canal
saturado → productor no espera y el hueco queda en el journal; kill a mitad →
Fase 3 recupera.

### Fase 5: Retención, disco y ajustes

**Objetivo**: ajustes con default+merge (`al_generar` default; 7d/30d/
siempre), gracia de 7 días para interrumpidas explicada en la UI, barrido de
huérfanas, umbrales de disco, sección de Ajustes e i18n 21 idiomas. Crédito a
`reunion-local` según el estándar de la casa si corresponde.
**Validación**: `check:translations` verde; huérfana >7d limpiada y <7d
conservada; simulación de disco lleno.

### Fase 6: Validación Final

- [ ] `cargo build` + `tsc` pasan
- [ ] `bun run tauri dev` y ejercitar el flujo real: sesión → kill -9 →
      recuperar → regenerar documento (no solo compilar)
- [ ] Prueba reina: todo funciona con wifi apagado
- [ ] Criterios de éxito cumplidos, uno a uno con evidencia
- [ ] Premortem re-verificado con evidencia
- [ ] Blindajes capturados (`raiz blindar`)

## Aprendizajes (Self-Annealing)

### 2026-08-30: fsync por turno, medido y decidido

- **Medición**: 4,2 ms por evento con `sync_data` en cada write (SSD del
  equipo de desarrollo, test `costo_de_fsync_por_turno_es_asumible`).
- **Decisión**: fsync en CADA evento (N=1). A ritmo humano (un turno cada
  varios segundos) es ruido, y elimina la ventana de pérdida entre turnos.

### 2026-08-30: El cierre no se escribe donde se genera el acta

- **Error**: la Fase 1 v1 escribía `cierre` en `conversation_finish`, justo
  tras `documento`. Un kill entre esa línea y que React recibiera el acta
  dejaba el journal cerrado con el acta dentro: la recuperación (que busca
  journals SIN cierre) no la ofrecería jamás. Violaba el criterio post-acta
  del propio PRP. De regalo, al cerrar ahí el reset posterior ya no tenía
  grabador activo y el journal terminado no se borraba nunca.
- **Fix**: revisión de Alejandro; `documento` deja la sesión pendiente y el
  cierre exige confirmación del frontend o descarte explícito.
- **Aplicar en**: cualquier "hecho" que dependa de que OTRO proceso/capa
  reciba el dato: durable significa confirmado por el receptor, no emitido.

### 2026-08-30: El PRP nació con dos supuestos falsos

- **Error**: la v1 proponía escritura incremental sobre ESCAUD1 (imposible:
  `plaintext_len` va en el AAD de cada frame) y el tee del mic en el camino
  de manos libres (imposible como pre-VAD: `open_tap` entrega frames ya
  filtrados, lo dice su propio comentario).
- **Fix**: revisión de Alejandro del 30-ago; ESCAUD2 versionado y tee en
  `AudioRecorder` pre-`handle_frame`.
- **Aplicar en**: todo PRP futuro que toque `recording_crypto` o el camino de
  audio: leer los comentarios del formato/contrato ANTES de proponer, no
  después (protocolo AGENTS.md; el comentario de `open_tap` ya lo decía).

## Gotchas

- [ ] `bindings.ts` es autogenerado por tauri-specta: NO editar a mano;
      regenerar corriendo el binario debug desde src-tauri
- [ ] Strings en JSX prohibidos por ESLint: todo por i18next (21 locales)
- [ ] `STARTED` seguirá existiendo para la UI en vivo; el journal usa su
      propia ancla. No unificar en esta pasada: dos consumidores, dos relojes
- [ ] El tee del sistema va en Rust tras `read`, NO en Swift: el ring de
      Swift tiene tope de 60 s y su contrato es drenar-y-borrar
- [ ] `open_tap()` es post-VAD por contrato documentado: no sirve para el tee
- [ ] `cifrar_campo` es fail-open A PROPÓSITO para el dictado: no "arreglarlo",
      el journal usa su propia API estricta

## Anti-Patrones

- NO agregar crates que linkeen ggml (conflicto de símbolos con transcribe-cpp)
- NO llamadas de red en el camino feliz (100% local)
- NO editar `src/bindings.ts` a mano
- NO strings hardcodeados en JSX
- NO settings nuevos sin default + merge
- NO unwrap() en producción
- NO escribir audio o texto claro a disco "temporalmente" — fail-closed sin llave
- NO bloquear el camino de transcripción esperando al disco
- NO reescribir frames ya sellados de un contenedor: recovery descarta cola,
  jamás edita
- NO concatenar PCM a través de un hueco: el tiempo perdido se rellena, no se
  comprime

_PRP aprobado el 30-ago-2026. Condición de avance: no pasar a Fase 2 sin probar fail-closed, documento durable antes de cierre, y journal truncado recuperable._
