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
  "settings.general.sessionsTitle": { en: "Sessions", es: "Sesiones" },
  "settings.general.sessionRecorder.label": {
    en: "Crash-proof session journal",
    es: "Registro de sesiones a prueba de fallos",
  },
  "settings.general.sessionRecorder.description": {
    en: "Every turn and document is saved encrypted the moment it happens. If Escriba dies mid-session, the next launch offers to recover everything.",
    es: "Cada turno y acta se guarda cifrado en el momento. Si Escriba muere a mitad de una sesión, al reabrir te ofrece recuperarlo todo.",
  },
  "settings.general.sessionRetention.title": {
    en: "Keep session audio",
    es: "Conservar el audio de las sesiones",
  },
  "settings.general.sessionRetention.description": {
    en: "The recorded tracks let you re-transcribe a recovered session. The text journal weighs KBs and has its own cycle.",
    es: "Las pistas grabadas permiten re-transcribir una sesión recuperada. El registro de texto pesa KB y lleva su propio ciclo.",
  },
  "settings.general.sessionRetention.onDocument": {
    en: "Until the document is confirmed",
    es: "Hasta confirmar el acta",
  },
  "settings.general.sessionRetention.days7": { en: "7 days", es: "7 días" },
  "settings.general.sessionRetention.days30": { en: "30 days", es: "30 días" },
  "settings.general.sessionRetention.forever": { en: "Always", es: "Siempre" },
  "settings.general.sessionRetention.graceHint": {
    en: "An interrupted session always gets at least 7 days to be recovered before cleanup.",
    es: "Una sesión interrumpida tiene siempre al menos 7 días para recuperarse antes de la limpieza.",
  },
  "settings.general.sessionsCredit": {
    en: "The crash-proof recorder follows the idea of reunion-local (flopez1977, MIT): journal the state, keep the audio.",
    es: "El grabador a prueba de fallos sigue la idea de reunion-local (flopez1977, MIT): journal del estado y conservar el audio.",
  },
  "conversation.diskLow": {
    en: "Low disk space ({{mb}} MB free): session audio may stop being recorded.",
    es: "Queda poco disco ({{mb}} MB libres): el audio de la sesión podría dejar de grabarse.",
  },
  "settings.about.thanks.reunionLocal": {
    en: "And reunion-local (flopez1977), whose idea that a meeting must survive a crash became the session recorder.",
    es: "Y reunion-local (flopez1977), cuya idea de que una reunión debe sobrevivir a un crash se volvió el grabador de sesiones.",
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
