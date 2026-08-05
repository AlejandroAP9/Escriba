/**
 * Generador OFFLINE de la tabla de emojis dictados (PRP-006, Fase 4).
 *
 * Fuente: anotaciones en español de CLDR (common/annotations/es.xml),
 * licencia Unicode-3.0 (declarada en THIRD_PARTY_NOTICES.md). Se usan SOLO
 * los nombres canónicos (type="tts"), no las palabras clave sueltas: "cara"
 * o "feliz" a secas serían un campo minado de falsos positivos.
 *
 * La tabla NO reproduce CLDR verbatim: se filtran los signos ASCII, los
 * modificadores de tono de piel y las banderas, y se agregan alias dictables
 * curados a mano ("cara feliz", "pulgar arriba") que no vienen de CLDR pero
 * son como la gente dicta de verdad.
 *
 * Uso:    bun scripts/gen-emojis.ts <ruta-a-es.xml>
 * Salida: src-tauri/resources/es/emojis.tsv (committeado y auditable)
 */
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const AQUI = dirname(fileURLToPath(import.meta.url));
const SALIDA = join(AQUI, "..", "src-tauri", "resources", "es", "emojis.tsv");

const ruta = process.argv[2];
if (!ruta) {
  console.error("uso: bun scripts/gen-emojis.ts <ruta a es.xml de CLDR>");
  process.exit(2);
}
const xmlCrudo = readFileSync(ruta);
const xml = xmlCrudo.toString("utf8");

/** Normaliza como se compara en runtime: minúsculas y sin tildes (el dictado
 *  puede llegar sin ellas). */
const normalizar = (s: string) =>
  s
    .toLowerCase()
    .replaceAll("á", "a")
    .replaceAll("é", "e")
    .replaceAll("í", "i")
    .replaceAll("ó", "o")
    .replaceAll("ú", "u")
    .replaceAll("ü", "u")
    .replace(/\s+/g, " ")
    .trim();

/** ¿El "emoji" es de verdad un emoji dictable? Fuera signos ASCII/teclado,
 *  tonos de piel y banderas de país (pares de indicadores regionales). */
function esDictable(cp: string) {
  const puntos = [...cp].map((c) => c.codePointAt(0)!);
  if (puntos.every((p) => p < 0x2000)) return false; // ASCII y signos
  if (puntos.some((p) => p >= 0x1f3fb && p <= 0x1f3ff)) return false; // tonos
  if (puntos.every((p) => p >= 0x1f1e6 && p <= 0x1f1ff)) return false; // banderas
  return true;
}

const tabla = new Map<string, string>();
const re = /<annotation cp="([^"]+)" type="tts">([^<]+)<\/annotation>/g;
let m: RegExpExecArray | null;
let leidos = 0;
while ((m = re.exec(xml))) {
  leidos++;
  const [, cp, nombre] = m;
  if (!esDictable(cp)) continue;
  const clave = normalizar(nombre);
  if (!clave || clave.length < 3) continue;
  // Primera aparición gana; un nombre no puede apuntar a dos emojis.
  if (!tabla.has(clave)) tabla.set(clave, cp);
}

// Alias curados: cómo dicta la gente, no cómo nombra Unicode. Cada alias se
// revisa a mano; ante la duda, NO se agrega (premortem de falsos positivos).
const ALIAS: [string, string][] = [
  ["cara feliz", "🙂"],
  ["carita feliz", "🙂"],
  ["cara triste", "🙁"],
  ["carita triste", "🙁"],
  ["risa", "😂"],
  ["carcajada", "🤣"],
  ["guiño", "😉"],
  ["corazon", "❤️"],
  ["corazon rojo", "❤️"],
  ["pulgar arriba", "👍"],
  ["me gusta", "👍"],
  ["pulgar abajo", "👎"],
  ["aplausos", "👏"],
  ["manos arriba", "🙌"],
  ["fuego", "🔥"],
  ["cohete", "🚀"],
  ["fiesta", "🎉"],
  ["estrella", "⭐"],
  ["listo", "✅"],
  ["check", "✅"],
  ["equis roja", "❌"],
  ["ojos", "👀"],
  ["pensando", "🤔"],
  ["cafe", "☕"],
  ["pluma", "🪶"],
];
for (const [nombre, emoji] of ALIAS) {
  tabla.set(normalizar(nombre), emoji);
}

const filas = [...tabla.entries()].sort((a, b) =>
  a[0].localeCompare(b[0], "es"),
);
mkdirSync(dirname(SALIDA), { recursive: true });
const sha = createHash("sha256").update(xmlCrudo).digest("hex");
const cabecera = [
  "# emojis.tsv — emojis dictados en español (generado, NO editar a mano)",
  "# Generador: scripts/gen-emojis.ts | nombres canónicos tts de CLDR es +",
  "# alias curados. Claves normalizadas: minúsculas y sin tildes.",
  "# Fuente: CLDR common/annotations/es.xml, licencia Unicode-3.0",
  `# es.xml sha256=${sha}`,
  `# entradas=${filas.length}`,
].join("\n");
writeFileSync(
  SALIDA,
  cabecera + "\n" + filas.map(([n, e]) => `${n}\t${e}`).join("\n") + "\n",
);

console.error(
  `anotaciones tts leídas=${leidos} entradas=${tabla.size} → ${SALIDA}`,
);
let malo = false;
for (const debe of [
  "cara feliz",
  "pulgar arriba",
  "corazon rojo",
  "aplausos",
]) {
  const v = tabla.get(debe);
  console.error(`  ${debe} → ${v ?? "(AUSENTE)"}`);
  if (!v) malo = true;
}
// Nombres de un token demasiado genéricos que NO deben ser claves: si algún
// día CLDR los introduce, hay que revisarlos a mano.
for (const noDebe of ["cara", "feliz", "mano", "punto", "signo"]) {
  if (tabla.has(noDebe)) {
    console.error(
      `  ¡REVISAR! clave genérica "${noDebe}" → ${tabla.get(noDebe)}`,
    );
    malo = true;
  }
}
if (malo) {
  console.error("Centinelas fallidos: NO commitear sin revisar.");
  process.exit(1);
}
