//! Español profundo (PRP-006): restauración determinista de tildes.
//!
//! El mapa viene de `scripts/gen-tildes.ts` (fuente RLA-ES vía LibreOffice,
//! MPL 1.1, declarada en THIRD_PARTY_NOTICES.md) y contiene SOLO pares cuya
//! forma sin tilde NO es una palabra española válida y cuya restauración es
//! única: "rapido"→"rápido" y "pidio"→"pidió" entran; "llego" (yo llego),
//! "esta", "practico" o "medico" (yo medico) quedan fuera POR CONSTRUCCIÓN,
//! no por lista negra. Lo ambiguo pertenece al LLM de post-proceso, que ya lo
//! cubre cuando está activo; esta capa prefiere no tocar antes que devorar
//! castellano (incidente 813a0275).

use std::io::Read;
use std::sync::OnceLock;

use super::text::preserve_case_pattern;

/// Pares (sin_tilde → con_tilde) ordenados por clave, para búsqueda binaria.
/// ~158k pares; residentes solo tras el primer dictado en español.
static TILDES: OnceLock<Vec<(Box<str>, Box<str>)>> = OnceLock::new();

fn mapa_tildes() -> &'static [(Box<str>, Box<str>)] {
    TILDES.get_or_init(|| {
        let gz: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/es/tildes.tsv.gz"
        ));
        let mut texto = String::new();
        // El TSV va embebido en el binario: si no se puede leer es un bug de
        // build, no de runtime. Se degrada a mapa vacío; jamás panic.
        if flate2::read::GzDecoder::new(gz)
            .read_to_string(&mut texto)
            .is_err()
        {
            log::error!("tildes.tsv.gz embebido ilegible; restauración de tildes inactiva");
            return Vec::new();
        }
        let mut pares: Vec<(Box<str>, Box<str>)> = texto
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .filter_map(|l| l.split_once('\t'))
            .map(|(a, b)| (a.into(), b.trim().into()))
            .collect();
        // El generador ya ordena, pero el invariante de la búsqueda binaria
        // se garantiza aquí, no en un script externo.
        pares.sort();
        pares
    })
}

fn buscar(token_minusculas: &str) -> Option<&'static str> {
    let mapa = mapa_tildes();
    mapa.binary_search_by(|(k, _)| k.as_ref().cmp(token_minusculas))
        .ok()
        .map(|i| mapa[i].1.as_ref())
}

/// Vocales acentuadas y signos que solo existen en español: si aparecen, el
/// texto ya viene con ortografía española y la restauración es segura.
fn tiene_marcas_espanolas(texto: &str) -> bool {
    texto.chars().any(|c| "áéíóúñüÁÉÍÓÚÑ¿¡".contains(c))
}

/// Palabras función distintivas del español, en su forma SIN tilde (el caso
/// que nos ocupa es justamente texto que llegó sin tildes). Deliberadamente
/// NO incluye palabras que también existen en inglés ("con", "sin", "no").
const EVIDENCIA_ES: &[&str] = &[
    "que", "para", "pero", "como", "los", "las", "del", "una", "este", "esta", "estas", "estos",
    "porque", "donde", "cuando", "tambien", "muy", "hasta", "desde", "entre", "ahora", "todo",
    "toda", "cada",
];

/// ¿El texto parece español? Marcas ortográficas españolas, o al menos dos
/// palabras función distintivas DISTINTAS. Con `selected_language = "auto"`
/// el idioma detectado por el motor no llega hasta aquí, así que la evidencia
/// se toma del propio texto.
pub fn parece_espanol(texto: &str) -> bool {
    if tiene_marcas_espanolas(texto) {
        return true;
    }
    let mut vistas: Vec<&str> = Vec::new();
    for token in texto.split(|c: char| !c.is_alphabetic()) {
        if token.is_empty() {
            continue;
        }
        let min = token.to_lowercase();
        if EVIDENCIA_ES.contains(&min.as_str()) && !vistas.contains(&min.as_str()) {
            // `min` vive poco: guardamos la referencia estática de la lista.
            if let Some(fija) = EVIDENCIA_ES.iter().find(|w| **w == min) {
                vistas.push(fija);
            }
            if vistas.len() >= 2 {
                return true;
            }
        }
    }
    false
}

/// ¿Corresponde aplicar las correcciones de español a este dictado?
/// "es" explícito siempre; "auto" solo con evidencia en el propio texto.
pub fn aplica_espanol(selected_language: &str, texto: &str) -> bool {
    match selected_language {
        "es" => true,
        "auto" => parece_espanol(texto),
        _ => false,
    }
}

/// Las funciones con un disparador inequívoco ("emoji <nombre>" y numerales
/// españoles estrictos) también se pueden ejecutar con idioma automático. A
/// diferencia de la restauración general de tildes, sus parsers exactos no
/// modifican texto de otros idiomas cuando no reconocen la construcción.
pub fn permite_funciones_espanolas(selected_language: &str) -> bool {
    matches!(selected_language, "es" | "auto")
}

/// Tabla de emojis dictados (nombre normalizado → emoji), ordenada por clave.
/// Viene de CLDR es (nombres canónicos tts) + alias curados; ~1.900 entradas,
/// ~40 KB embebidos. Ver scripts/gen-emojis.ts.
static EMOJIS: OnceLock<Vec<(Box<str>, Box<str>)>> = OnceLock::new();

fn tabla_emojis() -> &'static [(Box<str>, Box<str>)] {
    EMOJIS.get_or_init(|| {
        let tsv = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/resources/es/emojis.tsv"
        ));
        let mut pares: Vec<(Box<str>, Box<str>)> = tsv
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .filter_map(|l| l.split_once('\t'))
            .map(|(a, b)| (a.into(), b.trim().into()))
            .collect();
        pares.sort();
        pares
    })
}

fn buscar_emoji(nombre_normalizado: &str) -> Option<&'static str> {
    let tabla = tabla_emojis();
    tabla
        .binary_search_by(|(k, _)| k.as_ref().cmp(nombre_normalizado))
        .ok()
        .map(|i| tabla[i].1.as_ref())
}

/// Normaliza igual que el generador: minúsculas y sin tildes (el dictado
/// puede llegar sin ellas).
fn normalizar_token(token: &str) -> String {
    token
        .to_lowercase()
        .replace('á', "a")
        .replace('é', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ú', "u")
        .replace('ü', "u")
}

/// Convierte "emoji <nombre>" en el emoji: "emoji cara feliz" → 🙂.
///
/// El token "emoji" es el disparador obligatorio; el nombre se compara por
/// tokens EXACTOS contra la tabla, probando primero el nombre más largo
/// (hasta 4 tokens) y solo a través de espacios: una coma corta el nombre,
/// así "pon un emoji, por favor" queda intacto (blindaje matcher-includes).
/// Sin coincidencia, no se toca nada: "me mandó un emoji precioso" sobrevive.
pub fn apply_dictated_emojis(texto: &str) -> String {
    // Spans de tokens alfabéticos (inicio, fin) sobre el texto original.
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut inicio: Option<usize> = None;
    for (i, c) in texto.char_indices() {
        if c.is_alphabetic() {
            if inicio.is_none() {
                inicio = Some(i);
            }
        } else if let Some(s) = inicio.take() {
            tokens.push((s, i));
        }
    }
    if let Some(s) = inicio {
        tokens.push((s, texto.len()));
    }

    let mut salida = String::with_capacity(texto.len());
    let mut cursor = 0; // byte hasta donde ya copiamos
    let mut idx = 0; // índice de token
    while idx < tokens.len() {
        let (ti, tf) = tokens[idx];
        let token = &texto[ti..tf];
        if normalizar_token(token) != "emoji" {
            idx += 1;
            continue;
        }
        // Candidatos: hasta 4 tokens siguientes, el más largo primero, y solo
        // si los separadores entre ellos son puro espacio en blanco.
        let mut reemplazo: Option<(&'static str, usize, usize)> = None; // (emoji, fin_bytes, tokens_consumidos)
        let max = (tokens.len() - idx - 1).min(4);
        'largo: for n in (1..=max).rev() {
            let mut nombre = String::new();
            let mut fin_prev = tf;
            for k in 1..=n {
                let (ni, nf) = tokens[idx + k];
                if !texto[fin_prev..ni].chars().all(char::is_whitespace) {
                    continue 'largo; // una coma u otro signo corta el nombre
                }
                if k > 1 {
                    nombre.push(' ');
                }
                nombre.push_str(&normalizar_token(&texto[ni..nf]));
                fin_prev = nf;
            }
            if let Some(emoji) = buscar_emoji(&nombre) {
                reemplazo = Some((emoji, fin_prev, n));
                break;
            }
        }
        match reemplazo {
            Some((emoji, fin, consumidos)) => {
                salida.push_str(&texto[cursor..ti]);
                salida.push_str(emoji);
                cursor = fin;
                idx += consumidos + 1;
            }
            None => {
                idx += 1;
            }
        }
    }
    salida.push_str(&texto[cursor..]);
    salida
}

/// Restaura tildes token a token. Reconstruye el texto completo por
/// segmentos (nada de regex que consuma el resto: blindaje regex-lookahead);
/// para toda entrada sin coincidencias la salida es byte a byte idéntica.
pub fn restore_tildes(texto: &str) -> String {
    let mut salida = String::with_capacity(texto.len() + 16);
    let mut resto = texto;
    while !resto.is_empty() {
        // Segmento no alfabético (separadores) tal cual.
        let fin_sep = resto
            .char_indices()
            .find(|(_, c)| c.is_alphabetic())
            .map(|(i, _)| i)
            .unwrap_or(resto.len());
        salida.push_str(&resto[..fin_sep]);
        resto = &resto[fin_sep..];
        if resto.is_empty() {
            break;
        }
        // Segmento alfabético: candidato a restauración.
        let fin_tok = resto
            .char_indices()
            .find(|(_, c)| !c.is_alphabetic())
            .map(|(i, _)| i)
            .unwrap_or(resto.len());
        let token = &resto[..fin_tok];
        let min = token.to_lowercase();
        // Si el token ya trae tilde, el motor hizo su trabajo: no se toca.
        if tiene_marcas_espanolas(&min) {
            salida.push_str(token);
        } else {
            match buscar(&min) {
                Some(con_tilde) => salida.push_str(&preserve_case_pattern(token, con_tilde)),
                None => salida.push_str(token),
            }
        }
        resto = &resto[fin_tok..];
    }
    salida
}

// ---------------------------------------------------------------------------
// Numerales hablados a cifras (PRP-006, Fase 5; petición de la comunidad).
// ---------------------------------------------------------------------------

/// ¿Cuánta agresividad al convertir numerales?
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ModoNumerales {
    /// Solo secuencias largas con evidencia numérica fuerte (multiplicador,
    /// decena compuesta o decimal con "coma"). "uno de los problemas", "hora
    /// y media" y "la una de la tarde" quedan intactos.
    Estricto,
    /// Planilla al frente: también numerales sueltos ("cinco" → 5).
    Planilla,
}

fn valor_unidad(t: &str) -> Option<u64> {
    Some(match t {
        "cero" => 0,
        "un" | "uno" | "una" => 1,
        "dos" => 2,
        "tres" => 3,
        "cuatro" => 4,
        "cinco" => 5,
        "seis" => 6,
        "siete" => 7,
        "ocho" => 8,
        "nueve" => 9,
        "diez" => 10,
        "once" => 11,
        "doce" => 12,
        "trece" => 13,
        "catorce" => 14,
        "quince" => 15,
        "dieciseis" => 16,
        "diecisiete" => 17,
        "dieciocho" => 18,
        "diecinueve" => 19,
        "veinte" => 20,
        "veintiun" | "veintiuno" | "veintiuna" => 21,
        "veintidos" => 22,
        "veintitres" => 23,
        "veinticuatro" => 24,
        "veinticinco" => 25,
        "veintiseis" => 26,
        "veintisiete" => 27,
        "veintiocho" => 28,
        "veintinueve" => 29,
        _ => return None,
    })
}

fn valor_decena(t: &str) -> Option<u64> {
    Some(match t {
        "treinta" => 30,
        "cuarenta" => 40,
        "cincuenta" => 50,
        "sesenta" => 60,
        "setenta" => 70,
        "ochenta" => 80,
        "noventa" => 90,
        _ => return None,
    })
}

fn valor_centena(t: &str) -> Option<u64> {
    Some(match t {
        "cien" | "ciento" => 100,
        "doscientos" | "doscientas" => 200,
        "trescientos" | "trescientas" => 300,
        "cuatrocientos" | "cuatrocientas" => 400,
        "quinientos" | "quinientas" => 500,
        "seiscientos" | "seiscientas" => 600,
        "setecientos" | "setecientas" => 700,
        "ochocientos" | "ochocientas" => 800,
        "novecientos" | "novecientas" => 900,
        _ => return None,
    })
}

fn es_token_numeral(t: &str) -> bool {
    valor_unidad(t).is_some()
        || valor_decena(t).is_some()
        || valor_centena(t).is_some()
        || matches!(
            t,
            "mil" | "millon" | "millones" | "y" | "coma" | "medio" | "media"
        )
}

/// Resultado de parsear una secuencia de tokens numerales.
struct NumeroParseado {
    valor: u64,
    /// Décimas tras "y medio" (500 sobre mil / 500.000 sobre millón).
    decimales: Option<String>,
    /// Evidencia fuerte: multiplicador, centena, decena compuesta o "coma".
    evidencia: bool,
    tokens: usize,
}

/// Parsea un grupo 1-999 desde `toks[i..]`. Devuelve (valor, consumidos, evidencia).
fn parsear_grupo(toks: &[&str], i: usize) -> Option<(u64, usize, bool)> {
    let mut valor = 0u64;
    let mut j = i;
    let mut evidencia = false;
    if let Some(c) = toks.get(j).and_then(|t| valor_centena(t)) {
        valor += c;
        j += 1;
        evidencia = true;
    }
    if let Some(d) = toks.get(j).and_then(|t| valor_decena(t)) {
        valor += d;
        j += 1;
        // "cuarenta y dos": la 'y' es obligatoria entre decena y unidad.
        if toks.get(j) == Some(&"y") {
            if let Some(u) = toks.get(j + 1).and_then(|t| valor_unidad(t)) {
                if u >= 1 && u <= 9 {
                    valor += u;
                    j += 2;
                    evidencia = true; // decena compuesta: evidencia fuerte
                }
            }
        }
    } else if let Some(u) = toks.get(j).and_then(|t| valor_unidad(t)) {
        // Unidad o especial (once, veintidós...) tras la centena o sola.
        if j == i || valor > 0 {
            valor += u;
            j += 1;
        }
    }
    if j == i {
        None
    } else {
        Some((valor, j - i, evidencia))
    }
}

/// Parsea una secuencia completa de numerales en español.
/// Gramática: [grupo "millon(es)"] [(grupo)? "mil"] [grupo] ["y medio"] |
/// grupo ["coma" unidades...]. El grupo final SOLO se acepta después de un
/// multiplicador: sin esa regla, "dos tres" parsearía como 5.
fn parsear_numero(toks: &[&str]) -> Option<NumeroParseado> {
    let mut valor = 0u64;
    let mut j = 0;
    let mut evidencia = false;
    let mut decimales: Option<String> = None;
    let mut hubo_multiplicador = false;

    // Millones: grupo + "millon(es)".
    if let Some((v, n, _e)) = parsear_grupo(toks, j) {
        if matches!(toks.get(j + n), Some(&"millon") | Some(&"millones")) {
            valor += v * 1_000_000;
            j += n + 1;
            evidencia = true;
            hubo_multiplicador = true;
            // "tres millones y medio" → +500.000
            if toks.get(j) == Some(&"y")
                && matches!(toks.get(j + 1), Some(&"medio") | Some(&"media"))
            {
                valor += 500_000;
                j += 2;
                return Some(NumeroParseado {
                    valor,
                    decimales,
                    evidencia,
                    tokens: j,
                });
            }
        }
    }
    // Miles: "mil" solo, o grupo + "mil".
    if toks.get(j) == Some(&"mil") {
        // "mil" solitario como secuencia completa es idiomático ("mil
        // gracias"): no cuenta como evidencia por sí solo.
        valor += 1000;
        j += 1;
        hubo_multiplicador = true;
        evidencia = toks.len() > 1;
    } else if let Some((v, n, _e)) = parsear_grupo(toks, j) {
        if toks.get(j + n) == Some(&"mil") {
            valor += v * 1000;
            j += n + 1;
            evidencia = true;
            hubo_multiplicador = true;
        }
    }
    // Grupo final (0-999): solo tras un multiplicador, o como ÚNICO grupo.
    if j == 0 {
        if let Some((v, n, e)) = parsear_grupo(toks, j) {
            valor += v;
            j += n;
            evidencia |= e;
        }
    } else if hubo_multiplicador {
        if let Some((v, n, e)) = parsear_grupo(toks, j) {
            valor += v;
            j += n;
            evidencia |= e;
        }
    }
    if j == 0 {
        return None;
    }
    // Decimales: "coma" + unidades una a una ("cuarenta y dos coma cinco").
    if toks.get(j) == Some(&"coma") {
        let mut digitos = String::new();
        let mut k = j + 1;
        while let Some(u) = toks.get(k).and_then(|t| valor_unidad(t)) {
            if u > 9 {
                break; // tras la coma se dictan dígitos, no "coma cuarenta"
            }
            digitos.push(char::from(b'0' + u as u8));
            k += 1;
        }
        if !digitos.is_empty() {
            decimales = Some(digitos);
            evidencia = true;
            j = k;
        }
    }
    Some(NumeroParseado {
        valor,
        decimales,
        evidencia,
        tokens: j,
    })
}

/// Formatea al estilo es-CL: miles con punto, decimales con coma.
fn formatear_numero(n: &NumeroParseado) -> String {
    let mut entero = n.valor.to_string();
    if n.valor >= 10_000 {
        // Separador de miles solo desde 10.000: "mil novecientos noventa y
        // ocho" se dicta como año tanto como cantidad y "1998" es la forma
        // neutra que sirve para ambos.
        let bytes: Vec<char> = entero.chars().collect();
        let mut con_puntos = String::new();
        for (idx, c) in bytes.iter().enumerate() {
            if idx > 0 && (bytes.len() - idx) % 3 == 0 {
                con_puntos.push('.');
            }
            con_puntos.push(*c);
        }
        entero = con_puntos;
    }
    match &n.decimales {
        Some(d) => format!("{},{}", entero, d),
        None => entero,
    }
}

/// El ASR a veces ya convierte la cantidad base pero deja el multiplicador en
/// palabras ("3 millones y medio"). El parser alfabético de abajo no puede
/// ver ese 3, así que resolvemos únicamente esta construcción inequívoca antes
/// de procesar los numerales completamente hablados.
fn mixed_millions_to_digits(texto: &str) -> String {
    static MIXED_MILLIONS: OnceLock<regex::Regex> = OnceLock::new();
    let pattern = MIXED_MILLIONS.get_or_init(|| {
        regex::Regex::new(
            r"(?iu)\b(?P<num>[0-9]+(?:\.[0-9]{3})*)\s+mill[oó]n(?:es)?(?P<half>\s+y\s+medi[oa])?\b",
        )
        .expect("regex estática de millones mixtos")
    });

    pattern
        .replace_all(texto, |caps: &regex::Captures<'_>| {
            let original = caps.get(0).map_or("", |m| m.as_str());
            let Some(base) = caps
                .name("num")
                .and_then(|m| m.as_str().replace('.', "").parse::<u64>().ok())
            else {
                return original.to_string();
            };
            let Some(mut valor) = base.checked_mul(1_000_000) else {
                return original.to_string();
            };
            if caps.name("half").is_some() {
                let Some(con_medio) = valor.checked_add(500_000) else {
                    return original.to_string();
                };
                valor = con_medio;
            }
            formatear_numero(&NumeroParseado {
                valor,
                decimales: None,
                evidencia: true,
                tokens: 0,
            })
        })
        .into_owned()
}

/// Convierte numerales hablados a cifras. En `Estricto` solo secuencias con
/// evidencia fuerte; en `Planilla` también numerales sueltos. Reconstruye el
/// texto por segmentos: para entradas sin conversión, salida idéntica.
pub fn spoken_numbers_to_digits(texto: &str, modo: ModoNumerales) -> String {
    let texto_preparado = mixed_millions_to_digits(texto);
    let texto = texto_preparado.as_str();

    // Spans de tokens alfabéticos.
    let mut tokens: Vec<(usize, usize)> = Vec::new();
    let mut inicio: Option<usize> = None;
    for (i, c) in texto.char_indices() {
        if c.is_alphabetic() {
            if inicio.is_none() {
                inicio = Some(i);
            }
        } else if let Some(s) = inicio.take() {
            tokens.push((s, i));
        }
    }
    if let Some(s) = inicio {
        tokens.push((s, texto.len()));
    }

    let normalizados: Vec<String> = tokens
        .iter()
        .map(|(a, b)| normalizar_token(&texto[*a..*b]))
        .collect();

    let mut salida = String::with_capacity(texto.len());
    let mut cursor = 0;
    let mut idx = 0;
    while idx < tokens.len() {
        if !es_token_numeral(&normalizados[idx]) {
            idx += 1;
            continue;
        }
        // Corrida maximal de tokens numerales separados solo por espacios.
        let mut fin = idx;
        while fin + 1 < tokens.len()
            && es_token_numeral(&normalizados[fin + 1])
            && texto[tokens[fin].1..tokens[fin + 1].0]
                .chars()
                .all(char::is_whitespace)
        {
            fin += 1;
        }
        let refs: Vec<&str> = normalizados[idx..=fin].iter().map(|s| s.as_str()).collect();
        // "ciento" huérfano no es un número en español: el 100 solitario es
        // "cien"; "ciento" solo vive en compuestos ("por ciento", "ciento uno").
        if refs == ["ciento"] {
            idx = fin + 1;
            continue;
        }
        match parsear_numero(&refs) {
            // Solo si el parse consume la corrida COMPLETA: "hora y media"
            // nunca llega aquí ("hora" no es numeral) y "una hora" tampoco
            // (parse parcial de corridas mixtas se descarta).
            Some(num) if num.tokens == refs.len() => {
                let convertir = match modo {
                    ModoNumerales::Estricto => num.evidencia,
                    ModoNumerales::Planilla => true,
                };
                if convertir {
                    salida.push_str(&texto[cursor..tokens[idx].0]);
                    salida.push_str(&formatear_numero(&num));
                    cursor = tokens[fin].1;
                }
                idx = fin + 1;
            }
            _ => {
                idx = fin + 1;
            }
        }
    }
    salida.push_str(&texto[cursor..]);
    salida
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restaura_solo_lo_inequivoco() {
        // "medico" (yo medico) y "llego" (yo llego) son formas válidas y NO
        // se tocan; "rapido" y "pidio" no existen sin tilde y SÍ se restauran.
        assert_eq!(
            restore_tildes("el medico llego rapido y pidio quedarse"),
            "el medico llego rápido y pidió quedarse"
        );
    }

    #[test]
    fn no_devora_pares_ambiguos() {
        // Cada una de estas palabras existe sin tilde: quedan intactas.
        for frase in [
            "esta casa es grande",
            "si vienes avisame el lunes",
            "quiero mas cafe",
            "aun no llega nadie",
            "es un caso practico",
            "caminaba hacia el sur",
            "el publico aplaudio al final",
        ] {
            let salida = restore_tildes(frase);
            for (orig, res) in frase.split(' ').zip(salida.split(' ')) {
                if ["esta", "si", "mas", "aun", "practico", "hacia", "publico"].contains(&orig) {
                    assert_eq!(orig, res, "'{orig}' no debía cambiar en: {salida}");
                }
            }
        }
    }

    #[test]
    fn restaura_conservando_mayusculas_y_puntuacion() {
        assert_eq!(restore_tildes("Rapido, ven."), "Rápido, ven.");
        assert_eq!(restore_tildes("CANCION nueva"), "CANCIÓN nueva");
        assert_eq!(
            restore_tildes("¿Viste la reunion de ayer?"),
            "¿Viste la reunión de ayer?"
        );
    }

    #[test]
    fn texto_sin_coincidencias_queda_identico() {
        for texto in [
            "The quick brown fox jumps over the lazy dog.",
            "hola, todo bien... digo, todo tranquilo!",
            "",
            "   \n\t ",
            "123 456,78 -- ??",
        ] {
            assert_eq!(restore_tildes(texto), texto);
        }
    }

    #[test]
    fn restaura_adverbios_de_lugar() {
        // "aca" y "alla" no son palabras: acá y allá son restauraciones únicas.
        assert_eq!(
            restore_tildes("por aca todo bien, por alla no se"),
            "por acá todo bien, por allá no se"
        );
    }

    #[test]
    fn lo_ya_acentuado_no_se_toca() {
        assert_eq!(restore_tildes("la canción de ayer"), "la canción de ayer");
    }

    #[test]
    fn emoji_dictado_basico() {
        assert_eq!(apply_dictated_emojis("Emoji cara feliz."), "🙂.");
        assert_eq!(
            apply_dictated_emojis("Te mando un emoji pulgar arriba y chao"),
            "Te mando un 👍 y chao"
        );
        // El nombre más largo gana: "corazón rojo" antes que "corazón".
        assert_eq!(
            apply_dictated_emojis("emoji corazón rojo para ti"),
            "❤️ para ti"
        );
        assert_eq!(
            apply_dictated_emojis("emoji cara feliz, emoji pulgar arriba"),
            "🙂, 👍"
        );
    }

    #[test]
    fn emoji_sin_nombre_no_se_toca() {
        for frase in [
            "Me mandó un emoji precioso ayer.",
            "Ella puso un emoji en el chat del curso.",
            "pon un emoji, por favor",
            "el emoji",
        ] {
            assert_eq!(apply_dictated_emojis(frase), frase, "cambió: {frase}");
        }
    }

    #[test]
    fn emoji_normaliza_tildes_y_mayusculas() {
        // El dictado puede llegar sin tildes y con mayúscula inicial.
        assert_eq!(apply_dictated_emojis("EMOJI CORAZON ROJO"), "❤️");
        assert_eq!(apply_dictated_emojis("emoji corazon rojo"), "❤️");
    }

    #[test]
    fn numerales_estricto_convierte_con_evidencia() {
        use ModoNumerales::Estricto;
        let casos = [
            ("tres millones y medio", "3.500.000"),
            ("3 millones y medio", "3.500.000"),
            ("Tengo 3 millones y medio.", "Tengo 3.500.000."),
            ("3 millones", "3.000.000"),
            ("ciento un mil trescientos cincuenta y nueve", "101.359"),
            ("cuarenta y dos coma cinco", "42,5"),
            ("doscientos treinta y cuatro mil quinientos", "234.500"),
            ("mil novecientos noventa y ocho", "1998"),
            (
                "El presupuesto es de quinientos veinte mil pesos",
                "El presupuesto es de 520.000 pesos",
            ),
            (
                "setenta y cinco por ciento de asistencia",
                "75 por ciento de asistencia",
            ),
            ("cero coma cinco", "0,5"),
        ];
        for (entrada, esperado) in casos {
            assert_eq!(spoken_numbers_to_digits(entrada, Estricto), esperado);
        }
    }

    #[test]
    fn numerales_estricto_no_toca_trampas() {
        use ModoNumerales::Estricto;
        for frase in [
            "Uno de los problemas es el tiempo.",
            "Nos vemos en una hora y media.",
            "Es la una de la tarde.",
            "Primero lo primero: revisar las notas.",
            "Ninguno de los dos vino.",
            "mil gracias por todo",
            "dos y tres son cinco",
            "dos tres",
            "cinco",
            "una y media",
        ] {
            assert_eq!(
                spoken_numbers_to_digits(frase, Estricto),
                frase,
                "cambió: {frase}"
            );
        }
    }

    #[test]
    fn numerales_planilla_es_agresivo() {
        use ModoNumerales::Planilla;
        assert_eq!(spoken_numbers_to_digits("cinco", Planilla), "5");
        assert_eq!(
            spoken_numbers_to_digits("cuarenta y dos coma cinco", Planilla),
            "42,5"
        );
        assert_eq!(spoken_numbers_to_digits("veintiuno", Planilla), "21");
    }

    #[test]
    fn evidencia_de_espanol() {
        assert!(parece_espanol("mañana tengo una prueba"));
        assert!(parece_espanol(
            "la reunion quedo para el martes porque todos podian"
        ));
        assert!(!parece_espanol(
            "The quick brown fox jumps over the lazy dog"
        ));
        assert!(!parece_espanol("one two three four five"));
        assert!(aplica_espanol("es", "whatever"));
        assert!(!aplica_espanol("en", "que donde cuando"));
        assert!(permite_funciones_espanolas("auto"));
        assert!(permite_funciones_espanolas("es"));
        assert!(!permite_funciones_espanolas("en"));
        assert!(aplica_espanol(
            "auto",
            "no se donde queda, pero vamos cuando quieras"
        ));
    }
}
