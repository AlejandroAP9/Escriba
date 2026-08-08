# PRP-007: Obsidian enlazable + benchmark Qwen3-ASR en español

> **Estado**: APROBADO (Alejandro aprobó la tanda el 8-ago; defaults: enlaces e índice ON con vista previa, inbox OFF, índice `Escriba.md`)
> **Fecha**: 2026-08-08
> **Proyecto**: Escriba
> **Origen**: PLAN-POST-HACKATHON.md, fases 6 (Obsidian de verdad, cierra a
> Takhygraphe) y 4 (Qwen3-ASR, eje restante de Dictum). Alcance aprobado por
> Alejandro el 8-ago; este documento espera su visto bueno. El corte de release
> lo decide Alejandro explícitamente; los commits a main fluyen al ritmo del
> trabajo.

## Objetivo

Que el export a Obsidian deje de ser "escribir un archivo" y se vuelva parte
del vault (nota índice MOC, menciones convertidas en enlaces `[[...]]`, bandeja
de entrada diaria opcional), y que la decisión sobre Qwen3-ASR se tome con
números propios y reproducibles (WER español + velocidad vs whisper-large-v3-turbo
y parakeet-v3), gane o pierda.

## Por Qué

| Problema                                                                                                                     | Solución                                                                                                          |
| ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| Las notas exportadas aterrizan sueltas: sin índice ni enlaces, no participan del grafo que es la razón de usar Obsidian      | Nota índice estilo MOC mantenida por Escriba + conversión de menciones a `[[enlaces]]` al exportar                 |
| Takhygraphe demostró un "linkable folder" mejor que nuestra mitad del export                                                 | Misma idea, reimplementada desde cero con nuestro path_guard: nunca salir del vault, jamás tocar la red            |
| Un dictado suelto no amerita una nota con título propio; hoy no tiene destino en el vault                                    | Bandeja de entrada opcional: nota inbox diaria a la que se envían dictados desde el historial                      |
| Dictum integró Qwen3-ASR y nadie ha medido si de verdad supera a turbo/parakeet en español                                   | Benchmark reproducible con el CLI propio (`--transcribe-file --json --repeat`) sobre los 40 audios de la batería   |
| Una dupla rival publicó mediciones contra nosotros; las nuestras deben ser re-corribles por cualquiera                       | Script committeado + JSON crudo committeado + decisión escrita con números, gane o pierda                          |

**Valor para el concurso/comunidad**: cae el último eje de Takhygraphe y el
último de Dictum. Y el benchmark es en sí mismo contenido publicable: números
honestos, reproducibles con dos comandos, sobre audios congelados.

## Qué

### Criterios de Éxito

Alcance A — Obsidian enlazable (fase 6 del plan):

- [ ] Exportar un acta de Sesiones a un vault real produce: nota nueva +
      índice actualizado con enlace válido + menciones convertidas a
      `[[enlaces]]` que resuelven al abrir Obsidian. Con wifi apagado.
- [ ] El matcher no enlaza por substring: con una nota `Ana.md`, la palabra
      "Analía" queda intacta; con `Plan.md` y `Plan Premium.md`, la mención
      "Plan Premium" enlaza a la nota larga (tests unitarios de ambos casos).
- [ ] Un symlink dentro del vault que apunta fuera del home NO aporta
      candidatos al matcher ni se recorre (test con vault sintético).
- [ ] Las ediciones manuales del usuario en la nota índice, fuera del bloque
      gestionado por Escriba, sobreviven a un nuevo export (test).
- [ ] La conversión de menciones se ve y se edita en el diálogo de vista
      previa ANTES de tocar el vault (filosofía existente: nada aterriza sin
      verse).
- [ ] Con la bandeja activada, dos dictados del historial enviados al inbox
      aterrizan en la misma nota diaria como dos entradas con hora, sin pisar
      nada (test).
- [ ] Vault sintético de 20.000 notas: la conversión de menciones tarda < 2 s
      y no lee el contenido de ninguna nota del vault (solo nombres).
- [ ] Los 3 ajustes nuevos llegan con default + merge: una instalación
      existente actualiza sin perder ajustes ni cambiar su comportamiento
      salvo lo anunciado.

Alcance B — benchmark Qwen3-ASR (fase 4 del plan):

- [ ] Qwen3-ASR-0.6B-Q8_0, Qwen3-ASR-1.7B-Q8_0 y parakeet-tdt-0.6b-v3-Q8_0
      descargados a `~/.cache/huggingface/hub` con el layout existente
      (blobs por SHA-256 + snapshots + refs, como
      `models--handy-computer--whisper-large-v3-turbo-gguf`), y
      `escriba --list-models --json` los reporta como instalados.
- [ ] `bun tests/bateria-es/benchmark.ts` corre los 4 modelos × 40 audios por
      el CLI real (`--transcribe-file --json --repeat 3`, idioma pinneado
      `es`, sandbox portable) y emite: WER global y por categoría, `best_ms`,
      RTF, `load_ms` y RAM pico por modelo, más el JSON crudo.
- [ ] El script falla cerrado: si falta un modelo o un audio, aborta con
      mensaje claro; jamás omite un contendiente en silencio.
- [ ] Decisión escrita con números en `docs/benchmarks/qwen3-asr-es.md` +
      entrada en CHANGELOG, **gane o pierda** (criterio del plan: la
      honestidad también es feature).
- [ ] Si Qwen gana en algún eje real: propuesta de cambio de recomendación en
      el catálogo como commit separado, sin release (catalog.json se compila
      en el binario; el corte lo decide Alejandro).

### Comportamiento Esperado

**Enlaces al exportar.** Termino una Sesión y pulso "Enviar a Obsidian". El
diálogo de vista previa se abre como hoy, pero el cuerpo ya trae las menciones
convertidas: donde dicté "hablamos del Plan Premium con Flor", y en mi vault
existen `Plan Premium.md` y `Flor.md`, veo `[[Plan Premium]]` y `[[Flor]]`.
Edito lo que quiera (incluso quitar enlaces) y guardo. En Obsidian, la nota
aparece en `Escriba/`, los enlaces resuelven, y el grafo la conecta.

**Nota índice.** En `Escriba/` vive `Escriba.md`: un MOC con la lista de todas
las notas exportadas (enlace + fecha), mantenido por Escriba dentro de un
bloque delimitado por marcadores. Encima o debajo del bloque puedo escribir lo
que quiera: mis notas sobreviven a cada export.

**Bandeja de entrada.** Activo "Bandeja de entrada diaria" en Ajustes. En el
historial, cada dictado gana la acción "Enviar al inbox": lo manda a
`Escriba/Inbox 2026-08-08.md` como una entrada más con su hora, sin diálogo
(es la vía rápida a propósito; la vía con revisión sigue siendo el export
normal).

**Benchmark.** Corro el script de descarga (una vez, con red, como cualquier
descarga de modelos), luego `bun tests/bateria-es/benchmark.ts`. Minutos
después tengo la tabla: WER español y velocidad de los 4 modelos sobre los
mismos 40 audios. La decisión queda escrita en docs con los números pegados.

## Contexto

### Referencias

Alcance A:

- `src-tauri/src/commands/obsidian.rs` — los 4 comandos existentes
  (`set_obsidian_vault`, `set_obsidian_notes_folder`, `get_obsidian_vault`,
  `export_to_obsidian`), `sanitize_folder`/`sanitize_filename`/`unique_path`,
  y la disciplina de revalidar el vault EN CADA operación (no solo al guardar
  el ajuste). Todo comando nuevo hereda esa disciplina.
- `src-tauri/src/path_guard.rs` — `contain_existing_path` (canonicaliza +
  contiene al home/app-data). La raíz del escaneo del vault pasa por aquí.
- `src-tauri/src/settings.rs:415,507` — `obsidian_notes_folder` (con
  `#[serde(default = ...)]`) y `obsidian_vault_path`: el patrón de default +
  merge para los 3 ajustes nuevos.
- `src/lib/obsidian.ts` — `sendToObsidian`: flujo perezoso de elegir vault.
- `src/stores/obsidianStore.ts` + `src/components/obsidian/ObsidianPreviewDialog.tsx`
  — el diálogo de revisión montado una vez en App.tsx; aquí se inyecta la
  conversión de menciones.
- `src/components/settings/ObsidianVault.tsx` — UI de Ajustes donde van los
  interruptores nuevos.
- `src/components/conversation/ConversationSettings.tsx:709` y
  `src/components/studio/StudioSettings.tsx:295` — los 2 únicos call sites de
  `requestObsidianExport` (no cambian: la conversión vive detrás del diálogo).
- `src/components/settings/history/HistorySettings.tsx` — acciones por entrada
  del historial (patrón `onCopyText`), donde va "Enviar al inbox".
- Blindajes aplicables: `matcher-includes-substring-falso-positivo` (match
  exacto con límites de palabra, largo primero), `catalogo-con-nombres-sucios`
  (la normalización de nombres degrada el matching en silencio: congelar casos
  de regresión), `patron-frontera-de-confianza-server-fail-closed` (los
  ajustes son editables a mano: validar en el backend, fail-closed).

Alcance B:

- `src-tauri/src/cli.rs` — `--transcribe-file`, `--model`, `--repeat`,
  `--json`, `--list-models` ya existen (PRP-006, fase 3). El benchmark NO toca
  el CLI.
- `tests/bateria-es/` — `casos.tsv` (40 casos, columna `texto` =
  verdad-terreno), `audio/` (40 archivos), `run.ts` (patrón a reusar: sandbox
  portable con hardlink del binario, ajustes de fábrica, idioma pinneado "es",
  modelos desde la caché HF compartida). OJO: `esperado.tsv` es el arnés de
  regresión del pipeline, NO se toca desde el benchmark.
- `src-tauri/src/catalog/catalog.json` — entradas ya presentes:
  `handy-computer/Qwen3-ASR-0.6B-gguf` (línea 454, `default_quant` Q8_0,
  `recommended` false, rank 9) y `handy-computer/Qwen3-ASR-1.7B-gguf` (línea
  2495). `qwen3_asr` está en `KNOWN_ARCHES` de
  `src-tauri/src/managers/model_capabilities.rs:32`: transcribe-cpp lo carga;
  por eso NO hay integración, solo medición.
- `src-tauri/src/managers/model.rs:278` — `hf_cached_path`: la app resuelve
  modelos en la caché HF estándar (`~/.cache/huggingface/hub`), y
  `discover_hf_cache_models` (línea 1559) los descubre por el layout
  `models--org--nombre/{blobs,snapshots,refs}`.
- Layout de referencia en disco:
  `~/.cache/huggingface/hub/models--handy-computer--whisper-large-v3-turbo-gguf/`
  → `refs/main` (contiene la revisión), `snapshots/<rev>/<archivo>.gguf`
  (symlink) → `blobs/<sha256>`. Verificado: parakeet-v3 y los Qwen NO están
  descargados en esta máquina; turbo Q8_0 sí.
- `scripts/gen_catalog.py` — sección `CURATION`: ahí vive la recomendación
  editorial. Un cambio de recomendación se propone como parche a CURATION +
  regeneración, o como edición documentada de catalog.json.
- Id de modelo para el CLI: formato `org/repo/archivo.gguf` (ver `MODELO` en
  `tests/bateria-es/run.ts:42`).

### Arquitectura Propuesta

**Alcance A — backend** (`commands/obsidian.rs`, sin manager nuevo: sigue
siendo I/O de archivos puntual, no estado residente):

- `link_obsidian_mentions(content: String) -> LinkedResult { content, links }`
  — comando nuevo. Recorre el vault (walkdir SIN seguir symlinks, saltando
  directorios ocultos como `.obsidian`/`.trash`, con tope de archivos) y junta
  los **nombres** de nota (basename sin `.md`); jamás lee contenidos. Matcher:
  límites de palabra Unicode, candidatos ordenados por longitud descendente,
  mínimo 3 caracteres, sensible a tildes, insensible a mayúsculas (cuando el
  texto difiere en caja se emite `[[Nota|mención]]`). Zonas excluidas: front
  matter YAML, bloques y spans de código, URLs, enlaces `[[...]]` y markdown
  ya existentes. Sin vault configurado devuelve el contenido intacto.
- Índice MOC dentro de `export_to_obsidian`: tras escribir la nota, actualiza
  `Escriba.md` en la carpeta de notas. Solo se reescribe el bloque entre
  `<!-- escriba:indice -->` y `<!-- /escriba:indice -->`; el resto del archivo
  es del usuario. El enlace usa el nombre FINAL del archivo (el de
  `unique_path`, con sufijo si hubo colisión). Idempotente: un enlace ya
  presente no se duplica. Si el archivo índice no existe, se crea.
- `append_to_obsidian_inbox(content: String) -> Result<String>` — comando
  nuevo: agrega al final de `Inbox YYYY-MM-DD.md` (en la carpeta de notas) una
  entrada `## HH:MM` + texto. Append puro: nunca reordena ni reescribe lo
  anterior. Misma revalidación de vault que `export_to_obsidian`.

**Alcance A — settings** (los 3 con default + `#[serde(default = ...)]` +
merge, patrón `obsidian_notes_folder`):

- `obsidian_link_mentions: bool` (default **true**; siempre revisable en el
  diálogo de vista previa, así que activarlo no esconde nada)
- `obsidian_index_note: bool` (default **true**; es la feature)
- `obsidian_daily_inbox: bool` (default **false**; el plan la define opcional)

**Alcance A — frontend**:

- `ObsidianPreviewDialog` llama a `link_obsidian_mentions` al abrirse (si el
  ajuste está activo) y muestra un hint con el número de enlaces detectados;
  el usuario edita libremente antes de guardar.
- `ObsidianVault.tsx`: 3 interruptores nuevos.
- `HistorySettings.tsx`: acción "Enviar al inbox" por entrada, visible solo
  con `obsidian_daily_inbox` activo; toast con la ruta al guardar.
- i18n: claves nuevas en `en` + `es` y al resto con `scripts/add-i18n-keys.ts`
  (`bun run check:translations` en verde).

**Alcance B — sin código de producto.** Dos piezas committeadas:

- `tests/bateria-es/descargar-benchmark.sh` (o sección documentada): descarga
  los 3 GGUF que faltan a la caché HF con el layout estándar (vía
  `hf download` de huggingface_hub, que produce exactamente
  `blobs/<sha256> + snapshots/<rev>/ + refs/main`, verificado contra el turbo
  existente). Todos en Q8_0 — mismo quant que el turbo instalado: comparación
  pareja. La descarga es un paso de setup explícito con red, como cualquier
  descarga de modelos; el benchmark en sí corre offline.
- `tests/bateria-es/benchmark.ts` (bun): reusa el sandbox portable de
  `run.ts`. Por modelo × audio: `--transcribe-file <audio> --model <id> --json
  --repeat 3` con idioma pinneado `es`. Métricas: WER contra `casos.tsv`
  (normalización documentada en el script: NFC, minúsculas, sin puntuación,
  espacios colapsados, **tildes conservadas** — son el eje español; el
  pipeline completo de correcciones aplica igual a los 4 motores, así que la
  comparación es de lo que vive el usuario), `best_ms`, RTF, `load_ms`, y RAM
  pico (`/usr/bin/time -l` en macOS). Salida: tabla Markdown + JSON crudo.
- Resultados y decisión: `docs/benchmarks/qwen3-asr-es.md` (tablas + JSON
  crudo al lado + condiciones de la corrida: máquina, quant, repeat) y entrada
  en CHANGELOG. Si Qwen gana en algún eje: propuesta de recomendación como
  commit separado sobre `CURATION`/catalog.json, marcada como pendiente del
  corte de Alejandro.

### Modelo de Datos

Sin migración de `history.db`. Solo los 3 campos nuevos en `AppSettings` con
default + merge (arriba). Los artefactos del benchmark son archivos
committeados (script + resultados), no estado de la app.

## Premortem (matar el proyecto en papel)

> Entradas: `raiz blindajes` (matcher-includes-substring-falso-positivo,
> catalogo-con-nombres-sucios, patron-frontera-de-confianza-server-fail-closed)
> + superficie real de app desktop: filesystem del usuario, symlinks, ajustes
> editables a mano, descargas.

| Amenaza (cómo se rompe)                                                                                                      | Cómo la mata el diseño                                                                                                                                            | Cómo se verifica                                                                                                    |
| ----------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| El escaneo del vault sale del vault por un symlink (a `/etc`, al home de otro usuario) o entra en un ciclo                    | Walk sin seguir symlinks + raíz pasada por `contain_existing_path` + tope de archivos; además solo se leen NOMBRES de archivo, nunca contenidos                    | Test: vault sintético con symlink a fuera → cero candidatos de ahí; ciclo de symlinks → termina                      |
| Falso enlace por substring: nota `Ana.md` enlaza dentro de "Analía" (blindaje matcher-includes)                               | Límites de palabra Unicode + candidatos por longitud DESC + mínimo 3 caracteres + sensible a tildes                                                                | Tests unitarios: Ana/Analía intacto; Plan vs Plan Premium gana la larga; "mas"/nota `Más.md` no se tocan             |
| Normalización silenciosa degrada el matching (blindaje catalogo-nombres-sucios)                                               | Reglas de normalización explícitas y congeladas en tests de regresión (caja sí, tildes no); cambiarlas exige romper un test a sabiendas                            | Suite de casos de matching congelada; cambio de regla → test rojo                                                    |
| La nota índice pisa contenido del usuario (read-modify-write ingenuo)                                                         | Bloque gestionado entre marcadores; fuera del bloque no se escribe ni un byte; sin marcadores, el bloque se re-agrega al final sin tocar lo demás                  | Test: editar el índice fuera del bloque → exportar → la edición sobrevive byte a byte                                |
| `settings_store.json` editado a mano apunta el vault/carpeta a donde no debe (frontera de confianza)                          | Los comandos nuevos heredan la disciplina de `export_to_obsidian`: revalidar vault y carpeta EN CADA operación con path_guard, fail-closed                          | Poner a mano un vault fuera del home → index/inbox/scan fallan con error claro, no escriben                          |
| Un título dictado con `]]`, `#` o marcadores de bloque rompe el índice o inyecta estructura                                   | `sanitize_filename` ya elimina lo peligroso para rutas; el índice enlaza el nombre de archivo final saneado, nunca texto crudo del dictado                          | Test: exportar título con `]] [[x`, `<!-- escriba:indice -->` → índice bien formado                                  |
| Vault gigante (20k+ notas) congela el export                                                                                  | Solo listado de nombres (sin abrir archivos) + tope de escaneo; al exceder el tope se enlaza con lo escaneado y se sigue                                            | Vault sintético de 20k notas → conversión < 2 s                                                                      |
| El enlazado "toca la red sin querer" (telemetría de crate, resolución externa)                                                | Cero dependencias nuevas de red; todo es `std::fs` + walk; principio del repo                                                                                      | Prueba reina: export completo con wifi apagado                                                                       |
| Dos appends al inbox se pisan o duplican                                                                                      | Append-only al final del archivo con entrada timestampeada; sin reescritura del cuerpo; una sola operación de escritura por envío                                  | Test: dos envíos seguidos → dos entradas íntegras en orden                                                           |
| Descarga corrupta/incompleta del GGUF                                                                                         | El layout HF nombra blobs por SHA-256 y `hf download` verifica; el script de setup re-comprueba tamaño+hash contra `catalog.json` antes de dar el modelo por listo | Alterar 1 byte del blob → el script lo rechaza y pide re-descarga                                                    |
| El benchmark mide los ajustes personales, no el motor (ya pasó: el diccionario personal contaminó la primera congelada)       | Sandbox portable heredado de `run.ts` (ajustes de fábrica) + idioma pinneado `es`                                                                                  | Corrida desde sandbox limpio; WER idéntico al re-correr (decode greedy determinista)                                 |
| Comparación injusta entre modelos (quants distintos, caché fría asimétrica, un modelo ausente que se omite en silencio)       | Mismo quant Q8_0 los 4; `--repeat 3` y `best_ms` para todos; `load_ms` aparte; el script ABORTA si falta cualquier modelo o audio (fail-closed)                    | Quitar un modelo de la caché → el script aborta con mensaje; tabla publica quant y condiciones                       |
| Números no reproducibles o con pinta de cherry-picking                                                                        | Script + JSON crudo committeados; decisión escrita gane o pierda; condiciones de máquina documentadas                                                              | Tercero re-corre los dos comandos y obtiene el mismo WER y tiempos comparables                                       |
| Un cambio de recomendación del catálogo se cuela a release sin decisión (catalog.json va compilado en el binario)             | La propuesta va en commit separado marcado "pendiente del corte"; regla del repo: el corte lo decide Alejandro                                                     | Revisar que ningún build/release se corte con ese commit sin OK explícito                                            |

## Blueprint (el ciclo de cultivo)

> Solo FASES. Las subtareas se generan al entrar a cada fase (bucle agéntico).

### Fase 1: Enlaces `[[...]]` al exportar

**Objetivo**: comando `link_obsidian_mentions` (escaneo de nombres sin salir
del vault + matcher con límites de palabra) integrado al diálogo de vista
previa, con su interruptor en Ajustes.
**Validación**: tests unitarios del matcher (substring, longitud, tildes,
zonas excluidas, symlinks) + export real a vault de pruebas con enlaces que
resuelven en Obsidian.

### Fase 2: Nota índice MOC

**Objetivo**: `Escriba.md` con bloque gestionado, actualizado en cada export
con el nombre final de archivo; interruptor en Ajustes.
**Validación**: test de supervivencia de ediciones fuera del bloque +
idempotencia (re-export no duplica) + export real.

### Fase 3: Bandeja de entrada diaria

**Objetivo**: `append_to_obsidian_inbox` + ajuste (default off) + acción por
entrada en el historial con su toast.
**Validación**: dos envíos → una nota diaria con dos entradas timestampeadas;
acción oculta con el ajuste apagado.

### Fase 4: Modelos del benchmark en la caché HF

**Objetivo**: script de setup que descarga Qwen3-ASR 0.6B/1.7B y parakeet-v3
(Q8_0) al layout HF estándar, verificando tamaño+hash contra catalog.json.
**Validación**: `escriba --list-models --json` reporta los 3 como instalados;
alterar 1 byte de un blob → el script lo rechaza.

### Fase 5: Script de benchmark y corrida completa

**Objetivo**: `tests/bateria-es/benchmark.ts` (4 modelos × 40 audios × repeat
3, sandbox portable, WER + best_ms + RTF + load_ms + RAM pico) y la corrida
real en el M4.
**Validación**: el script aborta si falta modelo/audio; re-correrlo da el
mismo WER; JSON crudo generado.

### Fase 6: Decisión documentada y propuesta de catálogo

**Objetivo**: `docs/benchmarks/qwen3-asr-es.md` con tablas, condiciones y
decisión escrita (gane o pierda) + entrada en CHANGELOG; si Qwen gana en algún
eje, commit separado con la propuesta de recomendación, pendiente del corte.
**Validación**: el documento permite a un tercero reproducir los números con
dos comandos; ningún release se corta sin OK de Alejandro.

### Fase 7: Validación Final

- [ ] `cargo build` + `tsc` + `bun run lint` + `bun run check:translations` pasan
- [ ] `bun run tauri dev` y ejercitar el flujo real: Sesión → export con
      enlaces → índice → inbox desde historial (no solo compilar)
- [ ] Prueba reina: export a Obsidian completo con wifi apagado
- [ ] La batería de regresión (`tests/bateria-es/run.ts`) sigue en verde
      (esperado.tsv intacto)
- [ ] Criterios de éxito cumplidos
- [ ] Premortem re-verificado con evidencia
- [ ] Blindajes capturados (`raiz blindar`)

## Estado de cierre (8-ago-2026)

Las 7 fases completas en 3 commits. Evidencia: 8 tests nuevos de Obsidian
(matcher, índice, symlinks) + 178 tests del backend en verde + batería de
regresión 40/40 con el binario final + benchmark corrido entero (4×40×3,
JSON crudo committeado) + tsc/lint/check:translations en verde. Veredicto del
benchmark: Qwen3-ASR NO desplaza recomendaciones (ver docs/benchmarks/); no
hay cambio de catálogo, así que no hay commit pendiente del corte por ese
lado. QA manual pendiente para Alejandro: export real a su vault (enlaces +
índice + inbox en Obsidian de verdad), y la prueba reina con wifi apagado.
Sin corte de release: lo decide Alejandro.

## Aprendizajes (Self-Annealing)

### 2026-08-08: catalog.json es {models: [...]}, no un array

- **Error**: el script de descarga asumió un array raíz y `catalogo.find`
  reventó.
- **Fix**: aceptar ambas formas; verificado contra el archivo real.
- **Aplicar en**: todo consumidor nuevo de catalog.json.

### 2026-08-08: los dos JSON del CLI tienen formas distintas

- **Error**: `--transcribe-file --json` emite UNA línea compacta;
  `--list-models --json` emite pretty multilínea. El benchmark parseaba
  "la última línea" para ambos y explotó con `]`.
- **Fix**: parsear el stdout completo en list-models (los logs van por
  stderr).
- **Aplicar en**: cualquier consumidor del CLI; documentado aquí para no
  redescubrirlo.

### 2026-08-08: el WER global puede mentir por FORMATO, no por precisión

- **Error conceptual cazado a tiempo**: Qwen3-ASR "ganaba" el WER global
  solo porque turbo y Parakeet convierten numerales a cifras y la
  verdad-terreno es el texto hablado: la categoría NUM medía divergencia de
  formato, no reconocimiento. Una lectura ingenua habría cambiado la
  recomendación del catálogo EN LA DIRECCIÓN EQUIVOCADA.
- **Fix**: lectura por categorías con la trampa explicada en el documento;
  la decisión se tomó excluyendo NUM.
- **Aplicar en**: todo benchmark futuro de STT: separar SIEMPRE errores de
  reconocimiento de divergencias de normalización/formato antes de decidir.


## Gotchas

- [ ] `src/bindings.ts` es autogenerado por tauri-specta SOLO en builds de
      depuración; los comandos nuevos aparecen ahí (no editar a mano fuera de eso)
- [ ] Strings en JSX prohibidos por ESLint: todo por i18next; claves a los 21
      locales con `scripts/add-i18n-keys.ts`
- [ ] Settings nuevos: default + `#[serde(default = ...)]` para instalaciones
      existentes (patrón `obsidian_notes_folder`)
- [ ] El índice debe enlazar el nombre FINAL de archivo que devolvió
      `unique_path` (con sufijo si hubo colisión), no el título
- [ ] `esperado.tsv` es regresión del pipeline, NO verdad-terreno del
      benchmark; la verdad-terreno es la columna `texto` de `casos.tsv`
- [ ] Los modelos viven en la caché HF compartida (`~/.cache/huggingface/hub`),
      no en app-data; id de modelo para el CLI: `org/repo/archivo.gguf`
- [ ] `catalog.json` va `include_str!` en el binario: cambiarlo afecta al
      release → commit separado, corte lo decide Alejandro
- [ ] La revalidación del vault va EN CADA operación de escritura, no solo al
      guardar el ajuste (el comentario en `export_to_obsidian` explica por qué)

## Anti-Patrones

- NO leer contenidos de las notas del vault: el matcher usa solo NOMBRES de
  archivo (privacidad y velocidad; los aliases del front matter quedan fuera
  de esta versión a propósito)
- NO seguir symlinks en el escaneo del vault
- NO agregar crates que linkeen ggml (conflicto de símbolos con transcribe-cpp)
- NO llamadas de red en el camino feliz (la descarga de modelos es setup
  explícito, como el gestor de modelos)
- NO editar `src/bindings.ts` a mano (se regenera en debug)
- NO strings hardcodeados en JSX (i18next + 21 locales)
- NO settings nuevos sin default + merge para instalaciones existentes
- NO unwrap() en producción
- NO tocar `esperado.tsv` ni el CLI desde el benchmark
- NO omitir un modelo en silencio si falta: el benchmark aborta (fail-closed)

_PRP pendiente aprobación. No se ha modificado código._
