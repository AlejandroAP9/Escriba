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
  "settings.general.spanishTitle": {
    en: "Spanish dictation",
    es: "Dictado en español",
  },
  "settings.general.dictatedEmojis.label": {
    en: "Dictated emojis",
    es: "Emojis dictados",
  },
  "settings.general.dictatedEmojis.description": {
    en: 'Say "emoji" plus its Spanish name and the symbol appears: "emoji cara feliz" becomes 🙂. Names come from Unicode CLDR plus natural aliases.',
    es: 'Di "emoji" y su nombre, y aparece el símbolo: "emoji cara feliz" se vuelve 🙂. Los nombres salen de Unicode CLDR más alias naturales ("pulgar arriba", "me gusta").',
  },
  "settings.general.spokenNumerals.label": {
    en: "Spoken numbers as digits",
    es: "Numerales hablados a cifras",
  },
  "settings.general.spokenNumerals.description": {
    en: 'Long number phrases become digits: "tres millones y medio" turns into 3.500.000. Only unambiguous sequences convert; "uno de los problemas" stays untouched.',
    es: 'Las cantidades largas se vuelven cifras: "tres millones y medio" se escribe 3.500.000. Solo convierte secuencias inequívocas; "uno de los problemas" queda intacto.',
  },
  "settings.general.numeralsSpreadsheet.label": {
    en: "Aggressive numbers in spreadsheets",
    es: "Números agresivos en planillas",
  },
  "settings.general.numeralsSpreadsheet.description": {
    en: 'With Excel, Numbers or LibreOffice Calc in front, even single numbers become digits: say "cinco" and 5 is typed. Google Sheets in a browser can\'t be detected.',
    es: 'Con Excel, Numbers o LibreOffice Calc al frente, hasta un número suelto se vuelve cifra: dictas "cinco" y se escribe 5. Google Sheets en el navegador no se puede detectar.',
  },
  "obsidian.linkedHint": {
    en: "{{count}} mention(s) linked to existing notes. Edit freely: nothing is written until you save.",
    es: "{{count}} mención(es) enlazadas a notas existentes. Edita con libertad: nada se escribe hasta que guardes.",
  },
  "obsidian.linkMentions.label": {
    en: "Link mentions to existing notes",
    es: "Enlazar menciones a notas existentes",
  },
  "obsidian.linkMentions.description": {
    en: "When exporting, names that match notes in your vault become [[links]]. You always review them in the preview before saving. Only note names are read, never their contents.",
    es: "Al exportar, los nombres que coinciden con notas de tu vault se vuelven [[enlaces]]. Siempre los revisas en la vista previa antes de guardar. Solo se leen nombres de notas, jamás su contenido.",
  },
  "obsidian.indexNote.label": {
    en: "Keep an index note",
    es: "Mantener una nota índice",
  },
  "obsidian.indexNote.description": {
    en: "Escriba maintains Escriba.md with links to every exported note. It only rewrites its own block: anything you write outside it survives.",
    es: "Escriba mantiene Escriba.md con enlaces a cada nota exportada. Solo reescribe su propio bloque: lo que escribas fuera de él sobrevive.",
  },
  "obsidian.dailyInbox.label": {
    en: "Daily inbox",
    es: "Bandeja de entrada diaria",
  },
  "obsidian.dailyInbox.description": {
    en: 'Adds a "Send to inbox" action to history entries: the dictation lands in a daily Inbox note with its time, no dialog. The reviewed path is still the normal export.',
    es: 'Agrega la acción "Enviar al inbox" en el historial: el dictado aterriza en una nota Inbox diaria con su hora, sin diálogo. La vía con revisión sigue siendo el export normal.',
  },
  "obsidian.inboxSaved": {
    en: "Sent to today's inbox in your vault.",
    es: "Enviado al inbox de hoy en tu vault.",
  },
  "obsidian.inboxNoVault": {
    en: "Choose your Obsidian vault first (Settings, Obsidian section).",
    es: "Primero elige tu vault de Obsidian (Ajustes, sección Obsidian).",
  },
  "obsidian.inboxFailed": {
    en: "Could not write to the inbox note.",
    es: "No se pudo escribir en la nota de inbox.",
  },
  "settings.history.sendToInbox": {
    en: "Send to daily inbox",
    es: "Enviar al inbox diario",
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
