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
  "settings.general.readSelectionVoice.title": {
    en: "Reading voice",
    es: "Voz de lectura",
  },
  "settings.general.readSelectionVoice.description": {
    en: "Choose the system voice or Escriba's included local voice for Read Selection.",
    es: "Elige la voz del sistema o la voz local incluida de Escriba para Leer selección.",
  },
  "conversation.systemTranslate.voiceEngineHint": {
    en: "Voice engine for the meeting interpreter",
    es: "Motor de voz del intérprete de reuniones",
  },
  "pluminHelp.open": {
    en: "Ask Plumín",
    es: "Pregúntale a Plumín",
  },
  "pluminHelp.title": {
    en: "Ask Plumín",
    es: "Pregúntale a Plumín",
  },
  "pluminHelp.description": {
    en: "Local, focused help about Escriba and your current setup.",
    es: "Ayuda local y acotada a Escriba y a tu configuración actual.",
  },
  "pluminHelp.intro": {
    en: "Tell me what is getting in your way. I can explain it and take you to the right section.",
    es: "Cuéntame qué te está frenando. Puedo explicártelo y llevarte a la sección correcta.",
  },
  "pluminHelp.questionLabel": {
    en: "What do you need help with?",
    es: "¿Con qué necesitas ayuda?",
  },
  "pluminHelp.placeholder": {
    en: "For example: Why isn't it writing? How do I change the shortcut? Does my voice leave my computer?",
    es: "Por ejemplo: ¿Por qué no escribe? ¿Cómo cambio el atajo? ¿Mi voz sale de mi computador?",
  },
  "pluminHelp.dictate": {
    en: "Dictate your question",
    es: "Dictar la pregunta",
  },
  "pluminHelp.ask": {
    en: "Ask",
    es: "Preguntar",
  },
  "pluminHelp.thinking": {
    en: "Looking it up…",
    es: "Buscando…",
  },
  "pluminHelp.localAnswer": {
    en: "Answered by the local engine from Escriba's built-in guide.",
    es: "Respondido por el motor local desde la guía interna de Escriba.",
  },
  "pluminHelp.guideFallback": {
    en: "Verified answer from Escriba's built-in guide.",
    es: "Respuesta verificada de la guía incorporada de Escriba.",
  },
  "pluminHelp.openSection": {
    en: "Open the recommended section",
    es: "Abrir la sección recomendada",
  },
  "pluminHelp.speak": {
    en: "Read aloud",
    es: "Leer en voz alta",
  },
  "pluminHelp.stopSpeaking": {
    en: "Stop reading",
    es: "Detener lectura",
  },
  "pluminHelp.answers.troubleshooting": {
    en: "Check that a transcription model is selected, the correct microphone is active, microphone and Accessibility permissions are granted, and the shortcut is not used by another app. Open General to review those controls.",
    es: "Comprueba que haya un modelo seleccionado, que el micrófono correcto esté activo, que estén concedidos los permisos de Micrófono y Accesibilidad y que otra app no use el mismo atajo. Abre General para revisar esos controles.",
  },
  "pluminHelp.answers.shortcut": {
    en: "Open General and select the shortcut you want to change. Enter a new combination that is not already reserved by the system or another app.",
    es: "Abre General y selecciona el atajo que quieres cambiar. Introduce una combinación que no esté reservada por el sistema ni por otra app.",
  },
  "pluminHelp.answers.privacy": {
    en: "Transcription and the local writing engine run on your computer. Cloud providers are optional and only run if you configure them; saved history is encrypted and saving audio is off by default.",
    es: "La transcripción y el motor local de escritura funcionan en tu computador. Los proveedores cloud son opcionales y solo se usan si los configuras; el historial se cifra y guardar audio viene apagado.",
  },
  "pluminHelp.answers.microphone": {
    en: "Choose the microphone in General. If it is missing, refresh the device list and verify the operating system's microphone permission.",
    es: "Elige el micrófono en General. Si no aparece, actualiza la lista de dispositivos y comprueba el permiso de micrófono del sistema.",
  },
  "pluminHelp.answers.models": {
    en: "Models lets you download, select and remove transcription engines. Parakeet V3 is the balanced recommendation; Whisper large-v3-turbo prioritizes spelling quality.",
    es: "Modelos permite descargar, seleccionar y borrar motores de transcripción. Parakeet V3 es la recomendación equilibrada; Whisper large-v3-turbo prioriza la ortografía.",
  },
  "pluminHelp.answers.history": {
    en: "History keeps your dictation text so you can search, copy, save or retry it. Saving recordings is optional and off by default; when enabled, text and audio are encrypted at rest.",
    es: "Historial conserva el texto para buscarlo, copiarlo, guardarlo o reintentar. Guardar grabaciones es opcional y viene apagado; al activarlo, texto y audio quedan cifrados.",
  },
  "pluminHelp.answers.studio": {
    en: "Use Studio to transcribe an audio or video file without the microphone. You can review its segments and export TXT, JSON, SRT or VTT.",
    es: "Usa Estudio para transcribir un archivo de audio o video sin micrófono. Puedes revisar sus segmentos y exportar TXT, JSON, SRT o VTT.",
  },
  "pluminHelp.answers.translator": {
    en: "Translator handles turns between two languages and keeps brief context. Interpreter is for a live room, while Sessions can turn a longer conversation into a document.",
    es: "Traductor gestiona turnos entre dos idiomas y conserva contexto breve. Intérprete sirve para una sala en vivo; Sesiones puede convertir una conversación larga en documento.",
  },
  "pluminHelp.answers.writing": {
    en: "Smart Writing can correct, translate or change the tone of a dictation with the local engine. Configure its templates and shortcut in Smart Writing.",
    es: "Escritura inteligente puede corregir, traducir o cambiar el tono de un dictado con el motor local. Configura sus plantillas y su atajo en Escritura inteligente.",
  },
  "pluminHelp.answers.obsidian": {
    en: "Choose your vault in General. Escriba can export reviewed notes, maintain an index, link mentions and append dictations to a daily inbox.",
    es: "Elige tu vault en General. Escriba puede exportar notas revisadas, mantener un índice, enlazar menciones y añadir dictados a un inbox diario.",
  },
  "pluminHelp.answers.updates": {
    en: "Finding an update never installs it automatically: you must explicitly select “Update available”. Portable installations show the manual update path instead.",
    es: "Encontrar una actualización nunca la instala sola: debes pulsar explícitamente «Actualización disponible». Las instalaciones portables muestran la vía manual.",
  },
  "pluminHelp.answers.overview": {
    en: "I can help with dictation and permissions, shortcuts, models, privacy, History, Studio, Translator, Smart Writing, Obsidian and updates. Ask about one of those and I will take you to the right place.",
    es: "Puedo ayudarte con dictado y permisos, atajos, modelos, privacidad, Historial, Estudio, Traductor, Escritura inteligente, Obsidian y actualizaciones. Pregunta por uno y te llevaré al lugar correcto.",
  },
  "modelSelector.why.balanced": {
    en: "Best all-round",
    es: "Mejor equilibrio",
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
