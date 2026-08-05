use natural::phonetics::soundex;
use once_cell::sync::Lazy;
use regex::Regex;
use strsim::levenshtein;

/// Builds an n-gram string by cleaning and concatenating words
///
/// Strips punctuation from each word, lowercases, and joins without spaces.
/// This allows matching "Charge B" against "ChargeBee".
fn build_ngram(words: &[&str]) -> String {
    words
        .iter()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .collect::<Vec<_>>()
        .concat()
}

/// Finds the best matching custom word for a candidate string
///
/// Uses Levenshtein distance and Soundex phonetic matching to find
/// the best match above the given threshold.
///
/// # Arguments
/// * `candidate` - The cleaned/lowercased candidate string to match
/// * `custom_words` - Original custom words (for returning the replacement)
/// * `custom_words_nospace` - Custom words with spaces removed, lowercased (for comparison)
/// * `threshold` - Maximum similarity score to accept
///
/// # Returns
/// The best matching custom word and its score, if any match was found
fn find_best_match<'a>(
    candidate: &str,
    custom_words: &'a [String],
    custom_words_nospace: &[String],
    threshold: f64,
) -> Option<(&'a String, f64)> {
    if candidate.is_empty() || candidate.len() > 50 {
        return None;
    }

    let mut best_match: Option<&String> = None;
    let mut best_score = f64::MAX;

    for (i, custom_word_nospace) in custom_words_nospace.iter().enumerate() {
        // Skip if lengths are too different (optimization + prevents over-matching)
        // Use percentage-based check: max 25% length difference (prevents n-grams from
        // matching significantly shorter custom words, e.g., "openaigpt" vs "openai")
        let len_diff = (candidate.len() as i32 - custom_word_nospace.len() as i32).abs() as f64;
        let max_len = candidate.len().max(custom_word_nospace.len()) as f64;
        let max_allowed_diff = (max_len * 0.25).max(2.0); // At least 2 chars difference allowed
        if len_diff > max_allowed_diff {
            continue;
        }

        // Calculate Levenshtein distance (normalized by length)
        let levenshtein_dist = levenshtein(candidate, custom_word_nospace);
        let max_len = candidate.len().max(custom_word_nospace.len()) as f64;
        let levenshtein_score = if max_len > 0.0 {
            levenshtein_dist as f64 / max_len
        } else {
            1.0
        };

        // Calculate phonetic similarity using Soundex
        let phonetic_match = soundex(candidate, custom_word_nospace);

        // El impulso fonético solo vale para candidatos que YA están cerca por
        // escritura. Sin este tope rescataba cadenas 56% distintas: medido con
        // el diccionario real, "imperiales con su" (3 palabras) se convertía en
        // "Imperio Agéntico" y el verbo "escribir" en "Escriba". Un dictado
        // castellano corriente quedaba corrompido en silencio, que es peor que
        // no corregir un término raro.
        //
        // El corte sale de medir, no de intuición: los aciertos legítimos
        // ("imperio agentico" -> "Imperio Agéntico") tienen distancia ≤ 0,06;
        // los destrozos empiezan en 0,25. 0,20 los separa con holgura.
        // (Provocado por la medición pública de Diapasón, 30-jul-2026.)
        const MAX_LEV_PARA_IMPULSO_FONETICO: f64 = 0.20;
        let combined_score = if phonetic_match && levenshtein_score < MAX_LEV_PARA_IMPULSO_FONETICO
        {
            levenshtein_score * 0.3
        } else {
            levenshtein_score
        };

        // Accept if the score is good enough (configurable threshold)
        if combined_score < threshold && combined_score < best_score {
            best_match = Some(&custom_words[i]);
            best_score = combined_score;
        }
    }

    best_match.map(|m| (m, best_score))
}

/// Applies custom word corrections to transcribed text using fuzzy matching
///
/// This function corrects words in the input text by finding the best matches
/// from a list of custom words using a combination of:
/// - Levenshtein distance for string similarity
/// - Soundex phonetic matching for pronunciation similarity
/// - N-gram matching for multi-word speech artifacts (e.g., "Charge B" -> "ChargeBee")
///
/// # Arguments
/// * `text` - The input text to correct
/// * `custom_words` - List of custom words to match against
/// * `threshold` - Maximum similarity score to accept (0.0 = exact match, 1.0 = any match)
///
/// # Returns
/// The corrected text with custom words applied
pub fn apply_custom_words(text: &str, custom_words: &[String], threshold: f64) -> String {
    if custom_words.is_empty() {
        return text.to_string();
    }

    // Pre-compute lowercase versions to avoid repeated allocations
    let custom_words_lower: Vec<String> = custom_words.iter().map(|w| w.to_lowercase()).collect();

    // Pre-compute versions with spaces removed for n-gram comparison
    let custom_words_nospace: Vec<String> = custom_words_lower
        .iter()
        .map(|w| w.replace(' ', ""))
        .collect();

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut result = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let mut matched = false;

        // Try n-grams from longest (3) to shortest (1) - greedy matching
        for n in (1..=3).rev() {
            if i + n > words.len() {
                continue;
            }

            let ngram_words = &words[i..i + n];
            let ngram = build_ngram(ngram_words);

            if let Some((replacement, _score)) =
                find_best_match(&ngram, custom_words, &custom_words_nospace, threshold)
            {
                // Extract punctuation from first and last words of the n-gram
                let (prefix, _) = extract_punctuation(ngram_words[0]);
                let (_, suffix) = extract_punctuation(ngram_words[n - 1]);

                // Preserve case from first word
                let corrected = preserve_case_pattern(ngram_words[0], replacement);

                result.push(format!("{}{}{}", prefix, corrected, suffix));
                i += n;
                matched = true;
                break;
            }
        }

        if !matched {
            result.push(words[i].to_string());
            i += 1;
        }
    }

    result.join(" ")
}

/// Preserves the case pattern of the original word when applying a replacement
pub(crate) fn preserve_case_pattern(original: &str, replacement: &str) -> String {
    if original.chars().all(|c| c.is_uppercase()) {
        replacement.to_uppercase()
    } else if original.chars().next().is_some_and(|c| c.is_uppercase()) {
        let mut chars: Vec<char> = replacement.chars().collect();
        if let Some(first_char) = chars.get_mut(0) {
            *first_char = first_char.to_uppercase().next().unwrap_or(*first_char);
        }
        chars.into_iter().collect()
    } else {
        replacement.to_string()
    }
}

/// Extracts punctuation prefix and suffix from a word
///
/// Los índices son de BYTES, no de caracteres: la versión anterior contaba
/// caracteres y rebanaba bytes, así que un prefijo multibyte ("¿cómo",
/// "¡ándale!") hacía panic al caer el corte a mitad del signo — justo en el
/// camino caliente del dictado cuando el diccionario personal empataba.
fn extract_punctuation(word: &str) -> (&str, &str) {
    let prefix_end = word
        .char_indices()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(i, _)| i)
        .unwrap_or(word.len());
    let suffix_start = word
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphanumeric())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(prefix_end);

    (&word[..prefix_end], &word[suffix_start..])
}

/// Returns filler words appropriate for the given language code.
///
/// Some words like "um" and "ha" are real words in certain languages
/// (e.g., Portuguese "um" = "a/an", Spanish "ha" = "has"), so we only
/// include them as fillers for languages where they are truly fillers.
fn get_filler_words_for_language(lang: &str) -> &'static [&'static str] {
    let base_lang = lang.split(&['-', '_'][..]).next().unwrap_or(lang);

    match base_lang {
        "en" => &[
            "uh", "um", "uhm", "umm", "uhh", "uhhh", "ah", "hmm", "hm", "mmm", "mm", "mh", "eh",
            "ehh", "ha",
        ],
        "es" => &["ehm", "mmm", "hmm", "hm"],
        "pt" => &["ahm", "hmm", "mmm", "hm"],
        "fr" => &["euh", "hmm", "hm", "mmm"],
        "de" => &["äh", "ähm", "hmm", "hm", "mmm"],
        "it" => &["ehm", "hmm", "mmm", "hm"],
        "cs" => &["ehm", "hmm", "mmm", "hm"],
        "pl" => &["hmm", "mmm", "hm"],
        "tr" => &["hmm", "mmm", "hm"],
        "ru" => &["хм", "ммм", "hmm", "mmm"],
        "uk" => &["хм", "ммм", "hmm", "mmm"],
        "ar" => &["hmm", "mmm"],
        "ja" => &["hmm", "mmm"],
        "ko" => &["hmm", "mmm"],
        "vi" => &["hmm", "mmm", "hm"],
        "zh" => &["hmm", "mmm"],
        // Conservative universal fallback (no "um", "eh", "ha")
        _ => &[
            "uh", "uhm", "umm", "uhh", "uhhh", "ah", "hmm", "hm", "mmm", "mm", "mh", "ehh",
        ],
    }
}

static MULTI_SPACE_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"\s{2,}").unwrap());

/// Interrogativas con tilde. En español la tilde ES la marca interrogativa
/// ("como" conjunción vs "cómo" pregunta), así que anclan sin ambigüedad: no
/// existen con esa grafía en portugués, catalán ni gallego, y por eso estas
/// reglas no necesitan saber el idioma del dictado — se auto-limitan a texto
/// español. "por qué" se captura extendiendo "qué" hacia atrás.
const INTERROGATIVE_WORDS: &str =
    "qué|cómo|cuándo|dónde|adónde|quién|quiénes|cuál|cuáles|cuánto|cuánta|cuántos|cuántas";

/// "cómo ¿estás?" → el "¿" tiene delante una interrogativa con tilde: Whisper
/// decidió tarde que era pregunta y plantó el signo donde subió la entonación.
/// Se mueve el signo delante de la interrogativa (y de su "por" si es
/// "por qué").
static MISPLACED_OPENING_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(
        r"(?i)\b((?:por\s+)?(?:{INTERROGATIVE_WORDS})\s+)¿"
    ))
    .unwrap()
});

/// Normaliza los signos de apertura de interrogación del español.
///
/// El modelo comete dos fallos con "¿", y los dos se corrigen con la misma
/// ancla conservadora (una interrogativa CON tilde):
///
/// 1. **Mal colocado** — "Hola, cómo ¿estás?": el signo se mueve delante de la
///    interrogativa.
/// 2. **Ausente** — "Hola, cómo estás?": la oración termina en "?" sin "¿" en
///    ninguna parte; se inserta delante de la interrogativa que abre la última
///    cláusula (tras coma/punto y coma/dos puntos, o el inicio de la oración).
///
/// Lo que NO hace, a propósito: tocar oraciones sin ancla ("Vienes mañana?"),
/// porque insertar a ciegas rompería más de lo que arregla, ni oraciones que
/// ya traen un "¿" ("Vienes, ¿no?" queda tal cual).
pub fn fix_spanish_question_marks(text: &str) -> String {
    // Paso 1: mover los mal colocados.
    let text = MISPLACED_OPENING_PATTERN.replace_all(text, "¿$1");

    // Paso 2: insertar los ausentes, oración por oración (offsets de bytes).
    let mut result = String::with_capacity(text.len() + 8);
    let mut sentence_start = 0usize;
    let mut i = 0usize;
    while i < text.len() {
        let ch = text[i..].chars().next().unwrap();
        let ch_len = ch.len_utf8();
        if ch == '?' || ch == '!' || ch == '.' || ch == '\n' {
            let sentence = &text[sentence_start..i + ch_len];
            if ch == '?' && !sentence.contains('¿') {
                result.push_str(&insert_opening_mark(sentence));
            } else {
                result.push_str(sentence);
            }
            i += ch_len;
            // El espacio entre oraciones pertenece a la siguiente.
            sentence_start = i;
        } else {
            i += ch_len;
        }
    }
    result.push_str(&text[sentence_start..]);
    result
}

/// Inserta "¿" en una oración que termina en "?" y no lo trae, si la última
/// cláusula empieza con una interrogativa con tilde. Si no hay ancla, la
/// oración vuelve intacta.
fn insert_opening_mark(sentence: &str) -> String {
    // Inicio de la última cláusula: tras la última coma/;/: o el inicio.
    let clause_start = sentence.rfind([',', ';', ':']).map(|p| p + 1).unwrap_or(0);
    // Salta espacios al comienzo de la cláusula (offset de bytes).
    let word_start = sentence[clause_start..]
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map(|(i, _)| clause_start + i)
        .unwrap_or(sentence.len());

    // ¿La cláusula abre con interrogativa (con "por qué" incluido)?
    static CLAUSE_ANCHOR: Lazy<Regex> =
        Lazy::new(|| Regex::new(&format!(r"(?i)^(?:por\s+)?(?:{INTERROGATIVE_WORDS})\b")).unwrap());
    if CLAUSE_ANCHOR.is_match(&sentence[word_start..]) {
        let mut fixed = String::with_capacity(sentence.len() + 2);
        fixed.push_str(&sentence[..word_start]);
        fixed.push('¿');
        fixed.push_str(&sentence[word_start..]);
        fixed
    } else {
        sentence.to_string()
    }
}

/// Collapses repeated words (3+ repetitions) to a single instance.
/// E.g., "wh wh wh wh" -> "wh", "I I I I" -> "I"
fn collapse_stutters(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return text.to_string();
    }

    let mut result: Vec<&str> = Vec::new();
    let mut i = 0;

    while i < words.len() {
        let word = words[i];
        let word_lower = word.to_lowercase();

        if word_lower.chars().all(|c| c.is_alphabetic()) {
            // Count consecutive repetitions (case-insensitive)
            let mut count = 1;
            while i + count < words.len() && words[i + count].to_lowercase() == word_lower {
                count += 1;
            }

            // If 3+ repetitions, collapse to single instance
            if count >= 3 {
                result.push(word);
                i += count;
            } else {
                result.push(word);
                i += 1;
            }
        } else {
            result.push(word);
            i += 1;
        }
    }

    result.join(" ")
}

/// Filters transcription output by removing filler words and stutter artifacts.
///
/// This function cleans up raw transcription text by:
/// 1. Removing filler words based on the app language (or custom list)
/// 2. Collapsing repeated word stutters (e.g., "wh wh wh" -> "wh")
/// 3. Cleaning up excess whitespace
///
/// # Arguments
/// * `text` - The raw transcription text to filter
/// * `lang` - The app language code (e.g., "en", "pt-BR") used to select filler words
/// * `custom_filler_words` - Optional user-provided filler word list. `Some(vec)` overrides
///   language defaults; `Some(empty vec)` disables filtering; `None` uses language defaults.
///
/// # Returns
/// The filtered text with filler words and stutters removed
pub fn filter_transcription_output(
    text: &str,
    lang: &str,
    custom_filler_words: &Option<Vec<String>>,
) -> String {
    let mut filtered = text.to_string();

    // Build filler patterns from custom list or language defaults
    let patterns: Vec<Regex> = match custom_filler_words {
        Some(words) => words
            .iter()
            .filter_map(|word| Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word))).ok())
            .collect(),
        None => get_filler_words_for_language(lang)
            .iter()
            .map(|word| Regex::new(&format!(r"(?i)\b{}\b[,.]?", regex::escape(word))).unwrap())
            .collect(),
    };

    // Remove filler words
    for pattern in &patterns {
        filtered = pattern.replace_all(&filtered, "").to_string();
    }

    // Collapse repeated 1-2 letter words (stutter artifacts like "wh wh wh wh")
    filtered = collapse_stutters(&filtered);

    // Clean up multiple spaces to single space
    filtered = MULTI_SPACE_PATTERN.replace_all(&filtered, " ").to_string();

    // Trim leading/trailing whitespace
    filtered.trim().to_string()
}

#[cfg(test)]
mod correccion_castellano {
    use super::apply_custom_words;

    /// Fija el comportamiento que provocó la medición pública de Diapasón
    /// (avance 2, 30-jul-2026): la corrección por distancia + fonética "se come
    /// palabras enteras" en castellano. Se midió con el diccionario real de
    /// Alejandro y el umbral de fábrica, y era cierto también aquí.
    ///
    /// El daño venía del impulso fonético, que rescataba cadenas hasta 56%
    /// distintas. Ahora solo aplica a candidatos ya cercanos por escritura.
    #[test]
    fn no_devora_castellano_corriente_y_sigue_corrigiendo() {
        let dicc = vec![
            "Escriba".to_string(),
            "Imperio Agéntico".to_string(),
            "Claude".to_string(),
        ];
        let intactos = [
            // Antes: "los juegos Imperio Agéntico jurado" (3 palabras devoradas).
            "los juegos imperiales con su jurado",
            // Antes: "vamos a Escriba la propuesta" (verbo destrozado).
            "vamos a escribir la propuesta",
            "hay que escribirle a la apoderada",
            "la clase de ayer estuvo buena",
            "el club de lectura es el jueves",
        ];
        for t in intactos {
            assert_eq!(apply_custom_words(t, &dicc, 0.18), t, "no debía tocarse");
        }

        // Y lo que SÍ debe corregir sigue corrigiéndose: el término real,
        // dictado sin tilde, que es justo para lo que existe el diccionario.
        assert_eq!(
            apply_custom_words("esto es para imperio agentico", &dicc, 0.18),
            "esto es para Imperio Agéntico"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_custom_words_exact_match() {
        let text = "hello world";
        let custom_words = vec!["Hello".to_string(), "World".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "Hello World");
    }

    #[test]
    fn test_apply_custom_words_fuzzy_match() {
        let text = "helo wrold";
        let custom_words = vec!["hello".to_string(), "world".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_preserve_case_pattern() {
        assert_eq!(preserve_case_pattern("HELLO", "world"), "WORLD");
        assert_eq!(preserve_case_pattern("Hello", "world"), "World");
        assert_eq!(preserve_case_pattern("hello", "WORLD"), "WORLD");
    }

    #[test]
    fn test_extract_punctuation() {
        assert_eq!(extract_punctuation("hello"), ("", ""));
        assert_eq!(extract_punctuation("!hello?"), ("!", "?"));
        assert_eq!(extract_punctuation("...hello..."), ("...", "..."));
    }

    #[test]
    fn test_empty_custom_words() {
        let text = "hello world";
        let custom_words = vec![];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_filter_filler_words() {
        let text = "So uhm I was thinking uh about this";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "So I was thinking about this");
    }

    #[test]
    fn test_filter_filler_words_case_insensitive() {
        let text = "UHM this is UH a test";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "this is a test");
    }

    #[test]
    fn test_filter_filler_words_with_punctuation() {
        let text = "Well, uhm, I think, uh. that's right";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Well, I think, that's right");
    }

    #[test]
    fn test_filter_cleans_whitespace() {
        let text = "Hello    world   test";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Hello world test");
    }

    #[test]
    fn test_filter_trims() {
        let text = "  Hello world  ";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_filter_combined() {
        let text = "  Uhm, so I was, uh, thinking about this  ";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "so I was, thinking about this");
    }

    #[test]
    fn test_filter_preserves_valid_text() {
        let text = "This is a completely normal sentence.";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "This is a completely normal sentence.");
    }

    #[test]
    fn test_filter_stutter_collapse() {
        let text = "w wh wh wh wh wh wh wh wh wh why";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "w wh why");
    }

    #[test]
    fn test_filter_stutter_short_words() {
        let text = "I I I I think so so so so";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "I think so");
    }

    #[test]
    fn test_filter_stutter_longer_words() {
        let text = "Check data doc doc doc doc documentation.";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "Check data doc documentation.");
    }

    #[test]
    fn test_filter_stutter_mixed_case() {
        let text = "No NO no NO no";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "No");
    }

    #[test]
    fn test_filter_stutter_preserves_two_repetitions() {
        let text = "no no is fine";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "no no is fine");
    }

    #[test]
    fn test_filter_english_removes_um() {
        let text = "um I think um this is good";
        let result = filter_transcription_output(text, "en", &None);
        assert_eq!(result, "I think this is good");
    }

    #[test]
    fn test_filter_portuguese_preserves_um() {
        // "um" means "a/an" in Portuguese
        let text = "um gato bonito";
        let result = filter_transcription_output(text, "pt", &None);
        assert_eq!(result, "um gato bonito");
    }

    #[test]
    fn test_filter_spanish_preserves_ha() {
        // "ha" means "has" in Spanish
        let text = "ha sido un buen día";
        let result = filter_transcription_output(text, "es", &None);
        assert_eq!(result, "ha sido un buen día");
    }

    #[test]
    fn test_filter_language_code_with_region() {
        // "pt-BR" should normalize to "pt"
        let text = "um gato bonito";
        let result = filter_transcription_output(text, "pt-BR", &None);
        assert_eq!(result, "um gato bonito");
    }

    #[test]
    fn test_filter_custom_filler_words_override() {
        let custom = Some(vec!["okay".to_string(), "right".to_string()]);
        let text = "okay so I think right this works";
        let result = filter_transcription_output(text, "en", &custom);
        assert_eq!(result, "so I think this works");
    }

    #[test]
    fn test_filter_custom_filler_words_empty_disables() {
        let custom = Some(vec![]);
        let text = "So uhm I was thinking uh about this";
        let result = filter_transcription_output(text, "en", &custom);
        // No filler words removed since custom list is empty
        assert_eq!(result, "So uhm I was thinking uh about this");
    }

    #[test]
    fn test_filter_unknown_language_uses_fallback() {
        let text = "uh I think uhm this works";
        let result = filter_transcription_output(text, "xx", &None);
        assert_eq!(result, "I think this works");
    }

    #[test]
    fn test_filter_fallback_does_not_remove_um() {
        // Fallback (unknown language) should not remove "um" since it's a real word in some languages
        let text = "um I think this works";
        let result = filter_transcription_output(text, "xx", &None);
        assert_eq!(result, "um I think this works");
    }

    #[test]
    fn test_apply_custom_words_ngram_two_words() {
        let text = "il cui nome è Charge B, che permette";
        let custom_words = vec!["ChargeBee".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("ChargeBee,"));
        assert!(!result.contains("Charge B"));
    }

    #[test]
    fn test_apply_custom_words_ngram_three_words() {
        let text = "use Chat G P T for this";
        let custom_words = vec!["ChatGPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("ChatGPT"));
    }

    #[test]
    fn test_apply_custom_words_prefers_longer_ngram() {
        let text = "Open AI GPT model";
        let custom_words = vec!["OpenAI".to_string(), "GPT".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "OpenAI GPT model");
    }

    #[test]
    fn test_apply_custom_words_ngram_preserves_case() {
        let text = "CHARGE B is great";
        let custom_words = vec!["ChargeBee".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("CHARGEBEE"));
    }

    #[test]
    fn test_apply_custom_words_ngram_with_spaces_in_custom() {
        // Custom word with space should also match against split words
        let text = "using Mac Book Pro";
        let custom_words = vec!["MacBook Pro".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert!(result.contains("MacBook"));
    }

    #[test]
    fn test_extract_punctuation_multibyte_no_panic() {
        // La versión anterior contaba caracteres y rebanaba bytes: "¿" y "¡"
        // (2 bytes) hacían panic a mitad del signo.
        assert_eq!(extract_punctuation("¿cómo"), ("¿", ""));
        assert_eq!(extract_punctuation("¡ándale!"), ("¡", "!"));
        assert_eq!(extract_punctuation("«hola»"), ("«", "»"));
    }

    #[test]
    fn test_apply_custom_words_multibyte_punctuation_prefix() {
        // Camino real del panic: diccionario personal empatando una palabra
        // que llega del modelo con signo de apertura pegado.
        let text = "veamos ¡andale! amigo";
        let custom_words = vec!["ándale".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        assert_eq!(result, "veamos ¡ándale! amigo");
    }

    #[test]
    fn test_fix_misplaced_opening_mark() {
        // El caso de la captura del 26-jul: Whisper planta el ¿ donde subió
        // la entonación, no donde empieza la pregunta.
        assert_eq!(
            fix_spanish_question_marks("Hola, cómo ¿estás?"),
            "Hola, ¿cómo estás?"
        );
    }

    #[test]
    fn test_fix_misplaced_opening_mark_por_que() {
        assert_eq!(
            fix_spanish_question_marks("pero por qué ¿dices eso?"),
            "pero ¿por qué dices eso?"
        );
    }

    #[test]
    fn test_fix_missing_opening_mark_after_comma() {
        assert_eq!(
            fix_spanish_question_marks("Hola, cómo estás?"),
            "Hola, ¿cómo estás?"
        );
    }

    #[test]
    fn test_fix_missing_opening_mark_sentence_start() {
        assert_eq!(fix_spanish_question_marks("Cómo estás?"), "¿Cómo estás?");
    }

    #[test]
    fn test_fix_leaves_correct_spanish_alone() {
        assert_eq!(fix_spanish_question_marks("¿Cómo estás?"), "¿Cómo estás?");
        // La coletilla ya trae su ¿: no se toca.
        assert_eq!(
            fix_spanish_question_marks("Vienes mañana, ¿no?"),
            "Vienes mañana, ¿no?"
        );
    }

    #[test]
    fn test_fix_no_anchor_no_guess() {
        // Sin interrogativa con tilde no se inserta a ciegas.
        assert_eq!(
            fix_spanish_question_marks("Vienes mañana?"),
            "Vienes mañana?"
        );
        assert_eq!(fix_spanish_question_marks("How are you?"), "How are you?");
    }

    #[test]
    fn test_fix_multiple_sentences() {
        assert_eq!(
            fix_spanish_question_marks("Hola. Cómo estás? Bien, gracias."),
            "Hola. ¿Cómo estás? Bien, gracias."
        );
    }

    #[test]
    fn test_fix_statement_with_como_untouched() {
        // "cómo" indirecto sin "?" al final: nada que hacer.
        assert_eq!(
            fix_spanish_question_marks("No sé cómo estás."),
            "No sé cómo estás."
        );
    }

    #[test]
    fn test_apply_custom_words_trailing_number_not_doubled() {
        // Verify that trailing non-alpha chars (like numbers) aren't double-counted
        // between build_ngram stripping them and extract_punctuation capturing them
        let text = "use GPT4 for this";
        let custom_words = vec!["GPT-4".to_string()];
        let result = apply_custom_words(text, &custom_words, 0.5);
        // Should NOT produce "GPT-44" (double-counting the trailing 4)
        assert!(
            !result.contains("GPT-44"),
            "got double-counted result: {}",
            result
        );
    }
}
