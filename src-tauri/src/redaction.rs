//! Redacción de datos sensibles antes de escribirlos en el historial.
//!
//! En una app cuya propuesta es "habla y aparece el texto", que el usuario diga
//! algo sensible en voz alta no es un caso raro: es cuestión de tiempo. Antes,
//! una tarjeta de crédito dictada llegaba literal a `transcription_text` en
//! SQLite y se quedaba ahí, en claro, hasta que el usuario lo notara.
//!
//! **Qué NO hace esto.** Solo toca lo que se PERSISTE. El texto que se pega en
//! la app de destino sale intacto: si el usuario dictó su tarjeta para meterla
//! en un formulario, tiene que llegar entera. Redactar la salida rompería la
//! función principal del programa.
//!
//! **Alcance deliberadamente estrecho.** Solo patrones con estructura fuerte y
//! verificable, no heurísticas de lenguaje. Detectar "contraseñas" o "datos de
//! salud" por palabras clave daría falsos positivos constantes y acabaría
//! censurando dictados normales, que es peor que no hacer nada: el usuario
//! dejaría de confiar en su propio historial.

/// Marca que sustituye a un dato detectado.
const MASK: &str = "[dato sensible]";

/// Longitudes de tarjeta que acepta el algoritmo de Luhn en la práctica.
const CARD_MIN_DIGITS: usize = 13;
const CARD_MAX_DIGITS: usize = 19;

/// Validación de Luhn, el dígito de control que llevan las tarjetas de pago.
///
/// Es lo que evita que esto redacte cualquier número largo: un número de
/// teléfono, un código de pedido o una cifra dictada no pasan Luhn salvo por
/// casualidad (~1 de cada 10).
fn passes_luhn(digits: &[u8]) -> bool {
    if digits.len() < CARD_MIN_DIGITS || digits.len() > CARD_MAX_DIGITS {
        return false;
    }
    let mut sum = 0u32;
    let mut double = false;
    for &d in digits.iter().rev() {
        let mut v = d as u32;
        if double {
            v *= 2;
            if v > 9 {
                v -= 9;
            }
        }
        sum += v;
        double = !double;
    }
    sum % 10 == 0
}

/// Sustituye por [`MASK`] las secuencias que validan como tarjeta de pago.
///
/// Acepta los separadores que el dictado suele producir (espacios y guiones)
/// porque un número dictado casi nunca sale pegado.
pub fn redact_for_storage(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            out.push(chars[i]);
            i += 1;
            continue;
        }

        // Consumir la secuencia completa de dígitos y separadores internos.
        let start = i;
        let mut digits: Vec<u8> = Vec::new();
        let mut end = i;
        while end < chars.len() {
            let c = chars[end];
            if c.is_ascii_digit() {
                digits.push(c as u8 - b'0');
                end += 1;
            } else if (c == ' ' || c == '-')
                && end + 1 < chars.len()
                && chars[end + 1].is_ascii_digit()
                && !digits.is_empty()
            {
                // Separador solo si viene otro dígito detrás: así "1234 y" corta.
                end += 1;
            } else {
                break;
            }
        }

        if passes_luhn(&digits) {
            out.push_str(MASK);
        } else {
            out.extend(&chars[start..end]);
        }
        i = end;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_a_valid_card_number() {
        // Número de prueba de Visa, válido por Luhn, público en la documentación
        // de cualquier pasarela de pago.
        let text = "mi tarjeta es 4242 4242 4242 4242 gracias";
        let out = redact_for_storage(text);
        assert!(out.contains(MASK), "{}", out);
        assert!(!out.contains("4242"), "{}", out);
        assert!(out.starts_with("mi tarjeta es "));
        assert!(out.ends_with(" gracias"));
    }

    #[test]
    fn redacts_card_without_separators() {
        let out = redact_for_storage("4242424242424242");
        assert_eq!(out, MASK);
    }

    #[test]
    fn leaves_ordinary_numbers_alone() {
        // Lo importante del filtro no es lo que detecta, sino lo que NO toca:
        // años, cantidades, teléfonos y códigos son dictado normal.
        for text in [
            "quedamos en 2026",
            "son 1500 pesos",
            "llámame al 9 8765 4321",
            "el pedido 100200300400 llegó",
            "artículo 47 de la ley",
        ] {
            assert_eq!(redact_for_storage(text), text, "no debería tocar: {}", text);
        }
    }

    #[test]
    fn preserves_text_around_the_match() {
        let out = redact_for_storage("antes 4242424242424242 después");
        assert_eq!(out, format!("antes {} después", MASK));
    }

    #[test]
    fn empty_and_plain_text_pass_through() {
        assert_eq!(redact_for_storage(""), "");
        assert_eq!(redact_for_storage("hola qué tal"), "hola qué tal");
    }
}
