/**
 * Inyector de claves i18n nuevas en los 21 idiomas (herramienta de desarrollo).
 *
 * Regla del repo: toda string nace en en/translation.json y DEBE existir en
 * los 21 locales o `bun run check:translations` falla. Los idiomas sin
 * traducción humana reciben el texto en inglés (estado documentado en el
 * README: "la estructura está, muchas cadenas muestran el texto en inglés").
 *
 * Uso: editar NUEVAS abajo y correr `bun scripts/add-i18n-keys.ts`.
 * Idempotente: una clave ya presente en un locale no se pisa.
 */
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const LOCALES = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "src",
  "i18n",
  "locales",
);

/** clave → { en, es } (los demás locales reciben el texto en inglés). */
const NUEVAS: Record<string, { en: string; es: string }> = {
  "recovery.retranscribe": {
    en: "Recover re-transcribing the audio",
    es: "Recuperar re-transcribiendo el audio",
  },
  "recovery.retranscribing": {
    en: "Transcribing the recorded audio…",
    es: "Transcribiendo el audio grabado…",
  },
};


function poner(
  obj: Record<string, unknown>,
  ruta: string[],
  valor: string,
): boolean {
  const [cabeza, ...resto] = ruta;
  if (resto.length === 0) {
    if (cabeza in obj) return false; // no pisar traducciones existentes
    obj[cabeza] = valor;
    return true;
  }
  if (typeof obj[cabeza] !== "object" || obj[cabeza] === null) obj[cabeza] = {};
  return poner(obj[cabeza] as Record<string, unknown>, resto, valor);
}

for (const locale of readdirSync(LOCALES)) {
  const ruta = join(LOCALES, locale, "translation.json");
  let datos: Record<string, unknown>;
  try {
    datos = JSON.parse(readFileSync(ruta, "utf8"));
  } catch {
    continue;
  }
  let cambiadas = 0;
  for (const [clave, textos] of Object.entries(NUEVAS)) {
    const valor = locale === "es" ? textos.es : textos.en;
    if (poner(datos, clave.split("."), valor)) cambiadas++;
  }
  if (cambiadas > 0) {
    writeFileSync(ruta, JSON.stringify(datos, null, 2) + "\n");
    console.log(`${locale}: +${cambiadas}`);
  }
}
