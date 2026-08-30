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
  "recovery.title": {
    en: "A session was left unfinished",
    es: "Una sesión quedó a medias",
  },
  "recovery.subtitle": {
    en: "Escriba kept everything up to the last moment, encrypted on this device. What do you want to do with it?",
    es: "Escriba guardó todo hasta el último momento, cifrado en este equipo. ¿Qué quieres hacer con ella?",
  },
  "recovery.summary": {
    en: "{{turns}} turns · {{duration}}",
    es: "{{turns}} turnos · {{duration}}",
  },
  "recovery.hasDoc": {
    en: "Includes the final document",
    es: "Incluye el acta final",
  },
  "recovery.brokenTail": {
    en: "The crash cut the very last line; everything before it was saved.",
    es: "El corte rompió la última línea; todo lo anterior quedó a salvo.",
  },
  "recovery.recover": {
    en: "Recover session",
    es: "Recuperar la sesión",
  },
  "recovery.export": {
    en: "Export the document",
    es: "Exportar el acta",
  },
  "recovery.discard": {
    en: "Discard",
    es: "Descartar",
  },
  "recovery.discardTitle": {
    en: "Discard session",
    es: "Descartar sesión",
  },
  "recovery.discardConfirm": {
    en: "Delete this session? Its journal and any recovered document will be permanently removed.",
    es: "¿Eliminar esta sesión? Su registro y el acta recuperada se borran para siempre.",
  },
  "recovery.recovered": {
    en: "Session recovered",
    es: "Sesión recuperada",
  },
  "recovery.error": {
    en: "Could not recover the session",
    es: "No se pudo recuperar la sesión",
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
