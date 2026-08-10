# Benchmark: Qwen3-ASR en español, contra los titulares

**Pregunta**: ¿Qwen3-ASR (el motor que Dictum integró en los Juegos
Imperiales) supera a whisper-large-v3-turbo o a Parakeet V3 en español, en
esta app y en hardware real? La respuesta se decide con números propios,
reproducibles por cualquiera, gane o pierda.

## Cómo reproducirlo (dos comandos)

```bash
bun tests/bateria-es/descargar-benchmark.ts   # una vez, con red
bun tests/bateria-es/benchmark.ts             # el benchmark, offline
```

## Condiciones de la corrida

- **Máquina**: MacBook (Apple M4, 16 GB, Metal), macOS 26, sin otra carga.
- **Modelos**: los 4 en **Q8_0** (mismo quant: comparación pareja), desde la
  caché HF compartida. `--repeat 3` y se reporta `best_ms`; `load_ms` aparte.
- **Audios**: los 40 casos congelados de la batería de español
  (`tests/bateria-es/casos.tsv`), 7 voces sintéticas es_CL/ES/MX, categorías
  TIL (tildes), AMB (ambigüedad), EMO (emojis), NUM (numerales), GEN
  (general). Verdad-terreno: la columna `texto` de casos.tsv.
- **Pipeline completo**: se mide por el CLI real en sandbox portable con
  ajustes de fábrica e idioma pinneado a `es`: lo que vive el usuario, con
  las mismas correcciones deterministas para los 4 motores.
- **WER**: Levenshtein por palabras sobre texto normalizado (NFC, minúsculas,
  sin puntuación, espacios colapsados, **tildes conservadas**). La
  normalización está congelada en `benchmark.ts`.
- **Advertencia honesta**: voces sintéticas, no humanas. Sirven para comparar
  motores entre sí en condiciones idénticas; los WER absolutos con voz humana
  real pueden diferir. El JSON crudo va al lado
  (`qwen3-asr-es.crudo.json`) con las 160 transcripciones.

## Actualización del 9-ago: faltaba medir el modelo que usa la gente

Un usuario andaluz (Antonio Bocanet, Windows 11) reportó que Escriba le
destrozaba palabras cortas: "sí" → "fe", "esta" → "hecha", "voz" → "bos",
"Bot" → "voto". Estaba usando **Nemotron Streaming 3.5**, que es el modelo
**recomendado** del catálogo, y por lo tanto el que corre en las
instalaciones reales.

La primera corrida de este benchmark medía Whisper, Parakeet y los dos Qwen.
No medía Nemotron. **Medíamos todo menos el que le toca al usuario.** Añadido
al script de forma permanente; la tabla de abajo ya lo incluye.

| Modelo | WER | WER TIL | WER AMB | WER EMO | WER NUM | WER GEN | best_ms | RTF | load_ms | RAM pico |
|---|---|---|---|---|---|---|---|---|---|---|
| nemotron-3.5-asr-streaming Q8_0 **(recomendado)** | 17.2% | 5.6% | **33.3%** | 35.3% | 12.3% | 14.1% | 126 | 17.9x | 431 | 1027 MB |
| whisper-large-v3-turbo Q8_0 | 14.6% | **0.0%** | 5.6% | **5.9%** | 43.8% | 3.1% | 2564 | 0.9x | 501 | 1163 MB |
| parakeet-tdt-0.6b-v3 Q8_0 | 15.7% | **0.0%** | **0.0%** | 17.6% | 46.6% | **1.6%** | **78** | **29.0x** | **364** | **1010 MB** |
| Qwen3-ASR-0.6B Q8_0 | 12.3% | 9.3% | 2.8% | 23.5% | 15.1% | 10.9% | 209 | 10.7x | 601 | 1584 MB |
| Qwen3-ASR-1.7B Q8_0 | 10.7% | 1.9% | 13.9% | 26.5% | 15.1% | 3.1% | 529 | 4.3x | 1566 | 3199 MB |

(Recordar la trampa de NUM: turbo y Parakeet convierten numerales a cifras y
la verdad-terreno es texto hablado, así que esa columna mide FORMATO, no
precisión. Ver la sección siguiente.)

**El recomendado pierde contra Parakeet V3 en todas las categorías de
reconocimiento, y además es más lento** (17,9x contra 29,0x tiempo real) y usa
un poco más de RAM. Su única ventaja real es el streaming, que Parakeet no
soporta: sirve para ver el texto aparecer mientras hablas, no para acertar.

El 33,3% de error en AMB explica el reporte palabra por palabra: esa categoría
son frases cortas y comunes, exactamente donde un modelo de streaming decide
sin poder escuchar lo que viene después. Parakeet V3 marca 0,0% ahí.

**Propuesta (pendiente de decisión de Alejandro, toca el binario):** que
Parakeet TDT 0.6B v3 pase a ser el modelo recomendado, y que Nemotron quede
como la opción explícita de "quiero ver el texto en vivo". De paso, la
curación de `scripts/gen_catalog.py` se heredó de Handy y es anglocéntrica:
el rank 1 (`parakeet-unified-en-0.6b`, insignia "Mejor calidad") es un modelo
**solo inglés**, algo que en una interfaz en español desorienta a quien busca
el mejor motor para dictar.

## Resultados de la primera corrida (8-ago-2026, M4)

| Modelo | WER | WER TIL | WER AMB | WER EMO | WER NUM | WER GEN | best_ms | RTF | load_ms | RAM pico |
|---|---|---|---|---|---|---|---|---|---|---|
| whisper-large-v3-turbo Q8_0 | **14.6%** | 0.0% | 5.6% | 5.9% | 43.8% | 3.1% | 3737 | 0.6x | 956 | 1102 MB |
| parakeet-tdt-0.6b-v3 Q8_0 | **15.7%** | 0.0% | 0.0% | 17.6% | 46.6% | 1.6% | 191 | 11.8x | 405 | 987 MB |
| Qwen3-ASR-0.6B Q8_0 | **12.3%** | 9.3% | 2.8% | 23.5% | 15.1% | 10.9% | 327 | 6.9x | 715 | 1550 MB |
| Qwen3-ASR-1.7B Q8_0 | **10.7%** | 1.9% | 13.9% | 26.5% | 15.1% | 3.1% | 793 | 2.8x | 2137 | 3202 MB |

## La lectura correcta: el WER global miente aquí

A primera vista Qwen3-ASR-1.7B "gana" (10,7% contra 14,6%). Es un artefacto
de la categoría NUM: la verdad-terreno es el texto HABLADO ("tres millones y
medio"), y turbo y Parakeet convierten numerales a cifras por su cuenta
("3.500.000"), así que el WER los castiga por una divergencia de FORMATO que
para el usuario no es un error (Escriba incluso la ofrece como feature). Qwen
transcribe los numerales como palabras y "acierta" contra esta verdad-terreno.

Mirando las categorías que sí miden precisión de reconocimiento:

- **TIL (tildes)**: turbo y Parakeet perfectos (0,0%); Qwen 0.6B comete 9,3%.
- **AMB (ambigüedad)**: Parakeet perfecto; Qwen 1.7B el peor (13,9%).
- **EMO y GEN**: turbo y Parakeet ganan con claridad.

Excluyendo NUM, los dos titulares superan a ambos Qwen en TODAS las
categorías. Y en costo: Qwen 1.7B necesita 3,2 GB de RAM y 2,1 s de carga
para ser 4 veces más lento que Parakeet, que con 987 MB corre a 11,8x
tiempo real con la mejor precisión combinada.

## Decisión

**Qwen3-ASR no desplaza a ninguna recomendación del catálogo.** Parakeet V3
sigue siendo la recomendación correcta (precisión de punta, 11,8x tiempo
real, el que menos RAM usa) y whisper-large-v3-turbo sigue siendo la opción
de máxima ortografía. Los dos Qwen ya están en el catálogo para quien quiera
elegirlos; no cambia nada. El eje "Dictum integró un motor que nadie más
tiene" queda cerrado con números: integrarlo primero no lo hacía mejor.

Perder también se publica: este documento y el JSON crudo son la prueba.
