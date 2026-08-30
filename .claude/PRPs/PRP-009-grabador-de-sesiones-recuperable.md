# PRP-009: Grabador de sesiones recuperable (SessionRecorder)

> **Estado**: PENDIENTE
> **Fecha**: 2026-08-13
> **Proyecto**: Escriba
> **Origen**: análisis de `reunion-local` (flopez1977, MIT) del 13-ago-2026.
> Se toma la idea (journal de estado + conservar el audio), no el código:
> su stack (Python/Torch/pyannote/mlx-whisper) no encaja en un Tauri
> distribuible. Si en alguna fase se porta código sustancial, se conserva
> su aviso de copyright y se agrega el crédito con el estándar de la casa.

## Objetivo

Que una sesión de Escriba (reunión, clase, entrevista) sobreviva a un cierre
inesperado: al reabrir la app, los turnos ya transcritos se recuperan y el
audio de ambas pistas queda guardado cifrado para reprocesarlo, con una
política de retención que no llena el disco de nadie.

## Por Qué

| Problema                                                                                  | Solución                                                                       |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| Los turnos viven solo en RAM (`static TURNS`, conversation.rs:65). Un crash de una reunión de 1 h se lleva todo. | Journal append-only en disco; cada turno se persiste al llegar.                |
| El audio se drena y se descarta (system_audio.swift:68). Si la transcripción sale mal, no hay nada que reprocesar. | Ambas pistas se graban cifradas (contenedor ESCAUD1 ya existente) en paralelo. |
| El VAD descarta lo que clasifica como no-voz, incluida habla que clasificó mal.            | Se graba la señal **antes** del VAD: la pasada final no hereda sus errores.    |
| `STARTED` es un `Instant` (monótono): no se puede serializar ni reconstruir tras reinicio. | Ancla doble hora de pared + monótono, escrita en el journal al arrancar.       |

**Valor para la comunidad**: la promesa de Sesiones ("habla una hora y llévate
un documento") hoy tiene un asterisco invisible: *salvo que algo falle*. Esto
lo quita. Es además el prerrequisito de captura dual, integridad, doctor y
diarización (cola acordada el 13-ago).

## Qué

### Criterios de Éxito

- [ ] `kill -9` a Escriba a mitad de una sesión → al reabrir, la app ofrece
      recuperar; los turnos previos aparecen completos con sus `mm:ss`.
- [ ] La última línea del journal rota por el kill no impide recuperar las
      anteriores (cola incompleta se descarta en silencio, se loguea).
- [ ] Los `.escaud` de ambas pistas truncados por el kill se recuperan hasta
      el último frame AEAD válido y se pueden reproducir/re-transcribir.
- [ ] En disco no existe jamás audio claro ni texto claro de la sesión:
      journal cifrado por línea (`esc1:`), audio en ESCAUD1. `grep` de una
      frase dictada sobre los archivos de sesión → cero resultados.
- [ ] Sin llave del keyring disponible → la sesión funciona como hoy (RAM),
      no se escribe nada claro a disco, y la UI lo dice una vez (fail-closed).
- [ ] Con < umbral de disco libre al arrancar la sesión → aviso; si se agota
      durante: el audio se detiene con aviso, el journal (KB) continúa.
- [ ] Retención aplicada: sesiones cerradas se limpian según el ajuste
      (defecto: audio se borra al generar el documento; huérfanas > N días se
      barren al arrancar). Ajuste nuevo con default + merge.
- [ ] Descartar una recuperación ofrecida elimina journal y audio de esa
      sesión (verificable en disco).
- [ ] `cargo test` con casos de: línea rota, frame truncado, ancla de reloj,
      recuperación re-ejecutable (correr recovery dos veces = mismo estado).
- [ ] i18n: toda cadena nueva en 21 idiomas, `bun run check:translations` verde.

### Comportamiento Esperado

1. El usuario arranca una sesión. Escriba crea `sessions/<id>/` con
   `journal.jsonl` (ancla de reloj + modo como primera línea) y, si hay
   pista(s) activas, `mic.escaud` y/o `sys.escaud`.
2. Cada turno transcrito se apendea al journal al llegar, cifrado por línea.
   El audio crudo (pre-VAD) de cada pista se apendea por frames al `.escaud`.
3. La sesión termina bien → se genera el documento como hoy; según la
   política de retención, el audio se borra o se conserva; el journal se
   marca cerrado (línea final) y se archiva o borra según el ajuste.
4. La sesión muere mal (crash, kill, apagón) → al siguiente arranque Escriba
   detecta el journal sin línea de cierre y ofrece: **Recuperar** (turnos a
   la pantalla de Sesiones, audio disponible para regenerar el documento),
   **Exportar** (documento con lo que hay) o **Descartar** (borra todo).

## Contexto

### Referencias

- `src-tauri/src/commands/conversation.rs:65` — `static TURNS: Mutex<Vec<Turn>>`,
  cero escrituras a disco en todo el archivo. `STARTED` es `Instant`.
- `src-tauri/src/commands/conversation.rs:634-697` — camino del micrófono en
  manos libres: callback con buffer + corte por VAD. Punto de tee del mic.
- `src-tauri/src/commands/conversation.rs:897` — worker del audio del sistema:
  `system_audio::read(&mut chunk)` llena `pending` **antes** del VAD. Punto de
  tee de la pista del sistema.
- `src-tauri/swift/system_audio.swift:68` — `drain` retira muestras del ring;
  `removeAll()` al parar. El tee debe ser en Rust tras el read, no en Swift.
- `src-tauri/src/recording_crypto.rs` — contenedor ESCAUD1: frames
  XChaCha20-Poly1305 de 64 KiB, un tag AEAD por frame, seek por rangos, la
  webview nunca ve rutas. **Falta**: `save_encrypted_wav` escribe la grabación
  completa de una vez; hay que añadir un escritor incremental (append de
  frames + finalize tolerante a crash).
- `src-tauri/src/history_crypto.rs` — cifrado por campo (`esc1:`) con llave en
  keyring: se reusa para las líneas del journal. `cifrado_disponible()` es la
  puerta fail-closed.
- `src-tauri/src/managers/history.rs:295-311` — patrón de migración
  re-ejecutable (`migrate_legacy_wav`): el modelo para una recovery idempotente.
- Blindajes transferidos (raiz): `webhook-idempotencia-atomica` (recovery
  re-ejecutable, nunca "a medias dos veces"), `patron-frontera-de-confianza-
  server-fail-closed` (sin llave → no escribir, jamás degradar a claro).
- Idea de origen: `github.com/flopez1977/reunion-local` — `estado.py`
  (journal), conservación de WAV. Un solo commit, sin tags: referencia, no
  dependencia.

### Arquitectura Propuesta

**Backend (Rust), módulo nuevo `session_recorder.rs`:**

- `SessionRecorder` residente (patrón manager: `Arc<Mutex<Option<...>>>`),
  creado al arrancar sesión, cerrado al terminar. Dos responsabilidades:
  journal y pistas; nada de transcribir (eso sigue donde está).
- **Journal** `journal.jsonl`: una línea JSON por evento
  (`inicio{wall_ms, mono_anchor, modo, version}`, `turno{role, text_cifrado,
  at_ms}`, `cierre{}`), texto cifrado con `history_crypto::cifrar_campo`.
  Append con `write + flush`; fsync en `inicio`, `cierre` y cada N turnos
  (el costo de fsync por turno es despreciable a ritmo humano: medir en Fase 1
  y decidir N=1 si aguanta).
- **Reloj**: en `inicio` se guarda el par (SystemTime en ms, offset monótono
  0). Cada evento lleva `at_ms` monótono desde el ancla. La recuperación
  reconstruye `mm:ss` sin necesitar el `Instant` perdido.
- **Escritor incremental ESCAUD1** en `recording_crypto`: `EncryptedWavWriter`
  con `append(&[f32])` (bufferiza hasta 64 KiB y sella frame con su tag) y
  `finalize()` (escribe el conteo real en cabecera). La cabecera se escribe al
  crear con conteo 0/desconocido; **recovery**: recorrer frames verificando
  tags, truncar en el primero inválido, reescribir cabecera. Re-ejecutable.
- **Tee de pistas**: en el callback del mic y tras `system_audio::read`, las
  muestras crudas se envían por canal (`std::sync::mpsc`) al recorder, que
  escribe en su propio hilo. El camino de transcripción no espera al disco
  jamás (canal con tope: si el disco no da abasto, se descarta audio nuevo y
  se anota en el journal, nunca se bloquea la sesión).
- **Recovery al arrancar** (en el arranque de managers): escanear
  `sessions/*/journal.jsonl` sin línea `cierre` → emitir evento al frontend
  con el resumen (fecha, modo, nº turnos, duración estimada). Comandos
  tauri-specta: `session_recovery_list`, `session_recover(id)`,
  `session_recovery_discard(id)`.
- **Retención**: ajuste `session_audio_retention` (enum: `al_generar` |
  `dias_7` | `dias_30` | `siempre`... decidir en Fase 5, con default
  `al_generar`) + barrido de huérfanas al arrancar. Umbral de disco:
  comprobar espacio libre al arrancar sesión (aviso < 2 GB) y cada N MB
  escritos (corte de audio con aviso, journal sigue).

**Frontend:**

- Diálogo de recuperación al arrancar (evento → modal en Sesiones): tres
  botones, resumen de qué se recupera. Plumín en pose de guía.
- Sección nueva en Ajustes (grupo Sesiones): retención + estado del cifrado.
- i18n: claves nuevas vía `scripts/add-i18n-keys.ts`, 21 idiomas.

**Qué NO toca**: el pipeline de transcripción en vivo, el VAD, los prompts de
actas, el Intérprete. El recorder observa; no altera el camino existente.

### Modelo de Datos

Sin SQLite nuevo: el filesystem es el modelo (un directorio por sesión), igual
que las grabaciones del historial. `AppSettings` gana `session_audio_retention`
(String enum) y `session_recorder_enabled` (bool, default `true`), ambos con
`#[serde(default)]` + merge idempotente (patrón settings.rs:1143).

## Premortem (matar el proyecto en papel)

| Amenaza (cómo se rompe)                                                        | Cómo la mata el diseño                                                                                    | Cómo se verifica                                                                     |
| ------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Kill a mitad de línea del journal → JSON roto → recovery revienta o pierde todo | jsonl append-only; el parser tolera cola incompleta: descarta la última línea si no parsea, loguea, sigue  | test con archivo truncado en mitad de línea → recupera N-1 turnos                    |
| Kill a mitad de frame ESCAUD1 → contenedor corrupto                            | un tag AEAD por frame: recovery verifica frame a frame y trunca en el primero inválido; cabecera se reescribe | test: truncar a bytes arbitrarios → recovery deja archivo reproducible               |
| Recovery corre dos veces (doble arranque rápido, crash durante recovery)       | recovery idempotente estilo `migrate_legacy_wav`: verifica-antes-de-tocar, re-ejecutable (blindaje raiz)   | correr recovery 2× en test → estado idéntico, sin duplicar turnos                    |
| Keyring sin llave (sesión de SO recién creada, permiso denegado)               | fail-closed: sin `cifrado_disponible()` no se escribe NADA a disco; sesión sigue en RAM como hoy + aviso único | test unitario con llave ausente → cero archivos creados                              |
| Texto o audio claro tocan el disco                                             | journal cifrado por línea (`esc1:`), audio solo dentro de ESCAUD1 (ya garantiza no materializar WAV claro) | grep de frase dictada sobre `sessions/` → 0 hits; `is_encrypted` en ambas pistas     |
| Disco lleno a la hora de reunión (≈115 MB/h por pista en PCM16 ×2)             | chequeo al arrancar (aviso <2 GB) + corte de audio con aviso si se agota; journal (KB) nunca se corta      | simular disco lleno (cuota/imagen chica) → sesión sigue, audio parado, aviso emitido |
| El disco lento bloquea la transcripción en vivo                                | tee por canal con tope + hilo propio de escritura; lleno → descarta audio nuevo y lo anota, jamás bloquea   | test de canal saturado → el productor nunca espera                                   |
| La webview ve rutas del audio de sesión                                        | mismo protocolo privado de `recording_crypto` (lib.rs:858); ningún comando devuelve rutas                  | revisión de bindings generados: ninguna ruta en tipos expuestos                      |
| Huérfanas acumuladas llenan el disco con el tiempo                             | barrido al arrancar: sin `cierre` y > N días → según retención; descartadas → borrado inmediato verificado | crear huérfana vieja artificial → arranque la limpia                                 |
| Cambio de hora del sistema a mitad de sesión rompe los `mm:ss`                 | offsets monótonos contra ancla; la hora de pared solo etiqueta el inicio                                   | test: eventos con ancla fija → `mm:ss` estables ante wall alterado                   |

## Blueprint (el ciclo de cultivo)

> Solo FASES. Las subtareas se generan al entrar a cada fase (bucle agéntico).

### Fase 1: Reloj y journal

**Objetivo**: toda sesión escribe `inicio` + turnos + `cierre` cifrados en
`journal.jsonl`, con ancla wall+monótono. Sin UI nueva aún.
**Validación**: sesión real → archivo existe, líneas descifrables en test,
grep de texto claro = 0. Medir costo de fsync por turno.

### Fase 2: Recuperación

**Objetivo**: al arrancar, journals sin cierre se detectan y el diálogo
Recuperar/Exportar/Descartar funciona de punta a punta (turnos a pantalla).
**Validación**: `kill -9` en sesión real → reabrir → recuperar → turnos con
sus `mm:ss`. Recovery 2× = mismo estado. Descartar borra en disco.

### Fase 3: Escritor incremental ESCAUD1

**Objetivo**: `EncryptedWavWriter` con append por frames + finalize + recovery
por truncado a último frame válido, en `recording_crypto`, con tests propios.
**Validación**: truncar a bytes arbitrarios → siempre recupera reproducible;
`verify_encrypted_wav` pasa sobre lo recuperado.

### Fase 4: Tee de las dos pistas

**Objetivo**: mic (pre-VAD, callback manos libres) y sistema (pre-VAD, tras
`system_audio::read`) fluyen por canal al recorder sin bloquear jamás la
transcripción. La recuperación ofrece re-transcribir desde el audio.
**Validación**: sesión con ambas pistas → dos `.escaud` reproducibles; canal
saturado en test → productor no espera; kill a mitad → Fase 3 recupera.

### Fase 5: Retención, disco y ajustes

**Objetivo**: ajustes con default+merge, barrido de huérfanas, umbrales de
disco, sección de Ajustes e i18n 21 idiomas. Crédito a `reunion-local` según
el estándar de la casa (pantalla de Sesiones + Gracias) si corresponde.
**Validación**: `check:translations` verde; huérfana artificial limpiada;
simulación de disco lleno.

### Fase 6: Validación Final

- [ ] `cargo build` + `tsc` pasan
- [ ] `bun run tauri dev` y ejercitar el flujo real: sesión → kill -9 →
      recuperar → regenerar documento (no solo compilar)
- [ ] Prueba reina: todo funciona con wifi apagado
- [ ] Criterios de éxito cumplidos, uno a uno con evidencia
- [ ] Premortem re-verificado con evidencia
- [ ] Blindajes capturados (`raiz blindar`)

## Aprendizajes (Self-Annealing)

_(se llena durante la implementación)_

## Gotchas

- [ ] `bindings.ts` es autogenerado por tauri-specta: NO editar a mano;
      regenerar corriendo el binario debug desde src-tauri
- [ ] Strings en JSX prohibidos por ESLint: todo por i18next (21 locales)
- [ ] `STARTED` seguirá existiendo para la UI en vivo; el journal usa su
      propia ancla. No unificar en esta pasada: dos consumidores, dos relojes
- [ ] El tee del sistema va en Rust tras `read`, NO en Swift: el ring de
      Swift tiene tope de 60 s y su contrato es drenar-y-borrar

## Anti-Patrones

- NO agregar crates que linkeen ggml (conflicto de símbolos con transcribe-cpp)
- NO llamadas de red en el camino feliz (100% local)
- NO editar `src/bindings.ts` a mano
- NO strings hardcodeados en JSX
- NO settings nuevos sin default + merge
- NO unwrap() en producción
- NO escribir audio o texto claro a disco "temporalmente" — fail-closed sin llave
- NO bloquear el camino de transcripción esperando al disco

_PRP pendiente aprobación. No se ha modificado código._
