/**
 * Escriba: traducciones al español de las descripciones del catálogo de
 * modelos. Mapeadas por el TEXTO en inglés (estable entre quants del mismo
 * modelo) y no por id (que incluye el archivo de quant y es frágil).
 * Cubre las 28 descripciones únicas del catalog.json actual; una descripción
 * nueva simplemente se muestra en inglés hasta agregarla aquí.
 */
export const MODEL_DESCRIPTIONS_ES: Record<string, string> = {
  "Fast, accurate live English transcription":
    "Transcripción en vivo rápida y precisa, SOLO inglés",
  "Live multilingual transcription across 28 languages":
    "Transcripción multilingüe en vivo, 28 idiomas (incluye español)",
  // Descripciones nuevas de la curación del 9-ago (ver gen_catalog.py): el
  // recomendado dice que es bueno EN ESPAÑOL, y Nemotron dice lo que de
  // verdad aporta, que es ver el texto en vivo, no acertar más.
  "Fast and accurate in Spanish and 24 other European languages":
    "Rápido y preciso en español y otros 24 idiomas europeos",
  "See your words appear as you speak, in 28 languages":
    "Mira tus palabras aparecer mientras hablas, en 28 idiomas",
  "Tiny and instant, runs well on any hardware":
    "Diminuto e instantáneo, corre bien en cualquier equipo",
  "Highest accuracy, 14 languages, slower":
    "La mayor precisión, 14 idiomas, más lento",
  "Broadest language, but may run a bit slow":
    "El de más idiomas (100), puede ir algo lento",
  "Live multilingual, excellent on powerful machines":
    "Multilingüe en vivo, excelente en equipos potentes",
  "Fast and accurate. Supports 25 European languages":
    "Rápido y preciso. Soporta 25 idiomas europeos",
  "English only. The best model for English speakers":
    "SOLO inglés. El mejor modelo para angloparlantes",
  "Excellent multilingual model": "Excelente modelo multilingüe",
  "A tiny multilingual model": "Un modelo multilingüe diminuto",
  "100-language speech-to-text with translation, auto language detection, segment-level timestamps.":
    "Voz a texto en 100 idiomas con traducción, detección automática de idioma y marcas de tiempo.",
  "4-language speech-to-text with translation.":
    "Voz a texto en 4 idiomas con traducción.",
  "25-language speech-to-text with translation.":
    "Voz a texto en 25 idiomas con traducción.",
  "English speech-to-text.": "Voz a texto SOLO en inglés.",
  "3-language speech-to-text.": "Voz a texto en 3 idiomas.",
  "Russian speech-to-text with token-level timestamps.":
    "Voz a texto en ruso con marcas de tiempo por token.",
  "5-language speech-to-text.": "Voz a texto en 5 idiomas.",
  "6-language speech-to-text with translation.":
    "Voz a texto en 6 idiomas con traducción.",
  "5-language speech-to-text with word-level timestamps.":
    "Voz a texto en 5 idiomas con marcas de tiempo por palabra.",
  "English speech-to-text with token-level timestamps.":
    "Voz a texto SOLO en inglés con marcas de tiempo por token.",
  "English speech-to-text with streaming.":
    "Voz a texto SOLO en inglés, con streaming.",
  "English speech-to-text with streaming, token-level timestamps.":
    "Voz a texto SOLO en inglés, streaming y marcas de tiempo por token.",
  "30-language speech-to-text with auto language detection.":
    "Voz a texto en 30 idiomas con detección automática de idioma.",
  "5-language speech-to-text with auto language detection.":
    "Voz a texto en 5 idiomas con detección automática de idioma.",
  "8-language speech-to-text with translation, auto language detection.":
    "Voz a texto en 8 idiomas con traducción y detección automática.",
  "99-language speech-to-text with translation, auto language detection, segment-level timestamps.":
    "Voz a texto en 99 idiomas con traducción, detección automática y marcas de tiempo.",
  "English speech-to-text with segment-level timestamps.":
    "Voz a texto SOLO en inglés con marcas de tiempo por segmento.",
  "Optimized for Taiwanese Mandarin. Code-switching support.":
    "Optimizado para mandarín taiwanés, con soporte de cambio de idioma.",
};
