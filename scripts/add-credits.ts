/**
 * Créditos donde se usa la feature (11-ago-2026).
 *
 * A pedido de Max Cord, organización de los Juegos Imperiales, y extendido por
 * decisión de la dupla a TODOS los que aportaron una idea, no solo a quien lo
 * reclamó. La regla es la que Escriba ya aplicaba con Pedro Sánchez desde el
 * 15 de julio: "el crédito va donde se usa la feature, no escondido en un
 * changelog".
 *
 * Dos fórmulas distintas, a propósito:
 *  - Comunidad: "Idea de X" / "Pedido por X". Te lo regalaron.
 *  - Duplas de los Juegos Imperiales: "Crédito a X (Juegos Imperiales 2026)".
 *    Ellos no te dieron nada: tú tomaste la referencia. Decir "idea de" sería
 *    generoso de más y sonaría falso.
 *
 * Uso: bun scripts/add-credits.ts   (idempotente: no pisa lo ya traducido)
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

/** Sufijos de crédito, en línea, donde vive cada feature. */
const CREDITOS: Record<string, { en: string; es: string }> = {
  // --- Los tres modos visuales del 26-27 de julio ---
  "settings.general.appearance.visualModesCredit": {
    en: "Credit to Takhygraphe (Juegos Imperiales 2026): calm mode, high contrast and colorblind assistance follow what their team showed during the contest.",
    es: "Crédito a Takhygraphe (Juegos Imperiales 2026): el Modo Calma, el alto contraste y la asistencia para daltonismo siguen lo que mostró su dupla durante el concurso.",
  },
  // --- Obsidian enlazable ---
  "obsidian.credit": {
    en: "Credit to Takhygraphe (Juegos Imperiales 2026): the index note and linkable mentions follow what their team did better than us during the contest.",
    es: "Crédito a Takhygraphe (Juegos Imperiales 2026): la nota índice y las menciones enlazables siguen lo que su dupla hizo mejor que nosotros durante el concurso.",
  },
  // --- Español profundo ---
  "settings.general.spanishCredit": {
    en: "Credit to Abrax (Juegos Imperiales 2026): they showed that dictation in Spanish deserved its own linguistic work.",
    es: "Crédito a Abrax (Juegos Imperiales 2026): su dupla demostró que el dictado en español merecía trabajo lingüístico propio.",
  },
  // --- Numerales, pedido por la comunidad ---
  "settings.general.spokenNumerals.credit": {
    en: "Asked for by Juan Francisco Ceccarelli (community, 5 Aug 2026), for dictating data into spreadsheets.",
    es: "Pedido por Juan Francisco Ceccarelli (comunidad, 5-ago-2026), para dictar datos en planillas.",
  },
  // --- Plumín ayuda por voz ---
  "pluminHelp.credit": {
    en: "Credit to Fuwa (Juegos Imperiales 2026): theirs was the idea of in-app help that answers only about the app.",
    es: "Crédito a Fuwa (Juegos Imperiales 2026): suya fue la idea de una ayuda dentro de la app que solo responde sobre la app.",
  },
  // --- Modelos: la recomendación cambió por un reporte real ---
  "modelSelector.credit": {
    en: "Recommendation changed after Antonio Bocanet's report (community, 9 Aug 2026): we measured it and he was right.",
    es: "La recomendación cambió tras el reporte de Antonio Bocanet (comunidad, 9-ago-2026): lo medimos y tenía razón.",
  },
  // --- Diccionario personal: el bug que nos encontró Diapasón ---
  "settings.general.customWords.credit": {
    en: "Credit to Diapasón (Juegos Imperiales 2026): their public measurement found that this feature was eating Spanish words. Fixed, with a test so it cannot come back.",
    es: "Crédito a Diapasón (Juegos Imperiales 2026): su medición pública detectó que esta función se comía palabras en castellano. Arreglado, con un test para que no vuelva.",
  },

  // --- Pantalla de Gracias en Acerca de ---
  "settings.about.thanks.title": { en: "Thanks", es: "Gracias" },
  "settings.about.thanks.intro": {
    en: "Escriba is what it is because people told us what was missing. Each one is credited where their idea lives, and all of them are here.",
    es: "Escriba es lo que es porque hubo gente que nos dijo qué faltaba. Cada uno está acreditado donde vive su idea, y todos están aquí.",
  },
  "settings.about.thanks.communityTitle": { en: "Community", es: "Comunidad" },
  "settings.about.thanks.pedro": {
    en: "Pedro Sánchez: the whole visual inclusion push (Appearance), the Apple Intelligence fallback and Plumín reading the mood of a session.",
    es: "Pedro Sánchez: toda la tanda de inclusión visual (Apariencia), el respaldo de Apple Intelligence y que Plumín perciba el ánimo de la sesión.",
  },
  "settings.about.thanks.john": {
    en: "John Walter: the Meeting Interpreter, translating a call in both directions.",
    es: "John Walter: el Intérprete de reuniones, que traduce una llamada en las dos direcciones.",
  },
  "settings.about.thanks.juanfran": {
    en: "Juan Francisco Ceccarelli: spoken numbers as digits, for dictating into spreadsheets.",
    es: "Juan Francisco Ceccarelli: los numerales hablados a cifras, para dictar en planillas.",
  },
  "settings.about.thanks.antonio": {
    en: "Antonio Bocanet: he reported that the recommended model was mangling short Spanish words. We measured it, he was right, and the recommendation changed for everyone.",
    es: "Antonio Bocanet: reportó que el modelo recomendado destrozaba palabras cortas en español. Lo medimos, tenía razón, y la recomendación cambió para todos.",
  },
  "settings.about.thanks.alexa": {
    en: "Alexa Sánchez: she found that on Windows the install said the AI engine was ready when the system had blocked it. Now it says what really happened.",
    es: "Alexa Sánchez: descubrió que en Windows la instalación decía que el motor de IA estaba listo cuando el sistema lo había bloqueado. Ahora dice lo que de verdad pasa.",
  },
  "settings.about.thanks.flor": {
    en: "Flor Vallejo: the other half of the pair. The visual identity, the video and the landing page are hers.",
    es: "Flor Vallejo: la otra mitad de la dupla. La identidad visual, el video y la landing son suyos.",
  },
  "settings.about.thanks.rivalsTitle": {
    en: "Juegos Imperiales 2026 teams",
    es: "Duplas de los Juegos Imperiales 2026",
  },
  "settings.about.thanks.rivalsIntro": {
    en: "We competed against seven projects built on the same base. Several did something better than us, and it is only fair to say so by name. Ideas, never code: everything here was rewritten from scratch.",
    es: "Competimos contra siete proyectos construidos sobre la misma base. Varios hicieron algo mejor que nosotros y corresponde decirlo con nombre. Ideas, nunca código: todo esto se reescribió desde cero.",
  },
  "settings.about.thanks.takhygraphe": {
    en: "Takhygraphe: the visual accessibility modes, and an Obsidian export with an index note and linkable mentions that was better than ours.",
    es: "Takhygraphe: los modos visuales de accesibilidad, y un export a Obsidian con nota índice y menciones enlazables que era mejor que el nuestro.",
  },
  "settings.about.thanks.abrax": {
    en: "Abrax: they showed that dictation in Spanish deserved real linguistic work, which became accents, dictated emojis and numbers.",
    es: "Abrax: demostraron que el dictado en español merecía trabajo lingüístico de verdad, que aquí se volvió tildes, emojis dictados y numerales.",
  },
  "settings.about.thanks.dictum": {
    en: "Dictum: the headless command line with reproducible benchmarks, which is why our own measurements are now public.",
    es: "Dictum: la línea de comandos headless con benchmarks reproducibles, gracias a la cual hoy publicamos nuestras propias mediciones.",
  },
  "settings.about.thanks.fuwa": {
    en: "Fuwa: in-app help that answers only about the app, which here became Ask Plumín.",
    es: "Fuwa: la ayuda dentro de la app que solo responde sobre la app, que aquí se volvió Pregúntale a Plumín.",
  },
  "settings.about.thanks.diapason": {
    en: "Diapasón: they published a measurement that found a bug of ours, the personal dictionary eating Spanish words. Being corrected in public is also a gift.",
    es: "Diapasón: publicaron una medición que encontró un fallo nuestro, el diccionario personal comiéndose palabras en castellano. Que te corrijan en público también se agradece.",
  },
  "settings.about.thanks.upstream": {
    en: "And CJ Pais, whose Handy is the base all of this is built on.",
    es: "Y CJ Pais, cuyo Handy es la base sobre la que está construido todo esto.",
  },
};

function poner(
  obj: Record<string, unknown>,
  ruta: string[],
  valor: string,
): boolean {
  const [cabeza, ...resto] = ruta;
  if (resto.length === 0) {
    if (cabeza in obj) return false;
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
  let n = 0;
  for (const [clave, textos] of Object.entries(CREDITOS)) {
    const valor = locale === "es" ? textos.es : textos.en;
    if (poner(datos, clave.split("."), valor)) n++;
  }
  if (n > 0) {
    writeFileSync(ruta, JSON.stringify(datos, null, 2) + "\n");
    console.log(`${locale}: +${n}`);
  }
}
