/**
 * Generador OFFLINE del mapa de restauración de tildes (PRP-006, Fase 3).
 *
 * NO corre en build ni en runtime: lo corre un desarrollador y el TSV
 * resultante se commitea y se audita fila a fila en el diff.
 *
 * Fuente: diccionario RLA-ES (es_ES.dic + es_ES.aff), el mismo de LibreOffice.
 *   https://github.com/LibreOffice/dictionaries/tree/master/es
 *   Licencia: tri-licencia GPLv3 / LGPLv3 / MPL 1.1 — se usa bajo MPL 1.1
 *   (declarada en THIRD_PARTY_NOTICES.md). El diccionario NO se redistribuye:
 *   solo se distribuye el mapa derivado (pares sin-tilde → con-tilde).
 *
 * Regla central (premortem "el mapa devora castellano", incidente 813a0275):
 * un par `sin → con` entra SOLO si la forma sin tilde NO es una palabra
 * española válida. Así "rapido→rápido" y "pidio→pidió" entran, pero
 * "llego" (yo llego), "esta", "si", "mas", "aun", "practico", "hacia",
 * "medico" (yo medico) quedan excluidos por construcción, no por lista.
 * La lista EXCLUIDAS de abajo es un cinturón extra, no la defensa principal.
 *
 * Uso:
 *   bun scripts/gen-tildes.ts <dir-con-es_ES.dic-y-aff>
 * Salida:
 *   src-tauri/resources/es/tildes.tsv (+ estadísticas por stderr)
 */
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const AQUI = dirname(fileURLToPath(import.meta.url));
// Dos artefactos, una fuente: el TSV plano queda en scripts/data para que el
// diff sea auditable fila a fila; lo que se EMPAQUETA es el .gz (4,1MB → ~1MB,
// flate2 ya es dependencia del backend).
const SALIDA_AUDIT = join(AQUI, "data", "tildes-es.tsv");
const SALIDA_GZ = join(
  AQUI,
  "..",
  "src-tauri",
  "resources",
  "es",
  "tildes.tsv.gz",
);

const dir = process.argv[2];
if (!dir) {
  console.error(
    "uso: bun scripts/gen-tildes.ts <dir con es_ES.dic y es_ES.aff>",
  );
  process.exit(2);
}
const dicRuta = join(dir, "es_ES.dic");
const affRuta = join(dir, "es_ES.aff");
const dicCrudo = readFileSync(dicRuta);
const affCrudo = readFileSync(affRuta);
const sha = (b: Buffer) => createHash("sha256").update(b).digest("hex");

// ---------- parseo del .aff (solo SFX/PFX; es lo que usa RLA-ES) ----------

type Regla = {
  strip: string; // lo que se quita ("0" = nada)
  add: string; // lo que se agrega (puede traer /FLAGS de continuación)
  addFlags: string; // flags de continuación del sufijo agregado
  cond: RegExp; // condición sobre la palabra original
};
type Clase = { cross: boolean; reglas: Regla[] };

const sfx = new Map<string, Clase>();
const pfx = new Map<string, Clase>();

for (const linea of affCrudo.toString("utf8").split("\n")) {
  const p = linea.trim().split(/\s+/);
  if ((p[0] === "SFX" || p[0] === "PFX") && p.length >= 4) {
    const mapa = p[0] === "SFX" ? sfx : pfx;
    // Cabecera: EXACTAMENTE 4 campos, cross Y/N y conteo numérico
    // ("SFX E Y 73"). Una regla con add="0" ("SFX E r 0 [ae]r") tiene 5
    // campos: confundirla con cabecera reseteaba la clase entera y por eso
    // "llego" no se generaba (bug real de la primera corrida).
    if (
      p.length === 4 &&
      (p[2] === "Y" || p[2] === "N") &&
      /^\d+$/.test(p[3])
    ) {
      mapa.set(p[1], { cross: p[2] === "Y", reglas: [] });
    } else {
      // regla: SFX F r ción/S ar
      const clase = mapa.get(p[1]);
      if (!clase) continue;
      const [addRaw, flags = ""] = p[3].split("/");
      const condCruda = p[4] ?? ".";
      const cond =
        p[0] === "SFX"
          ? new RegExp(condCruda + "$")
          : new RegExp("^" + condCruda);
      clase.reglas.push({
        strip: p[2] === "0" ? "" : p[2],
        add: addRaw === "0" ? "" : addRaw,
        addFlags: flags,
        cond,
      });
    }
  }
}

// ---------- expansión (profundidad 2: sufijo + continuación tipo plural) ----------

function aplicarSfx(
  palabra: string,
  flags: string,
  salida: Set<string>,
  profundidad: number,
) {
  for (const f of flags) {
    const clase = sfx.get(f);
    if (!clase) continue;
    for (const r of clase.reglas) {
      if (!r.cond.test(palabra)) continue;
      if (r.strip && !palabra.endsWith(r.strip)) continue;
      const forma = palabra.slice(0, palabra.length - r.strip.length) + r.add;
      salida.add(forma);
      if (profundidad > 0 && r.addFlags)
        aplicarSfx(forma, r.addFlags, salida, profundidad - 1);
    }
  }
}

const VALIDAS = new Set<string>();
const lineasDic = dicCrudo.toString("utf8").split("\n").slice(1);
// Encoding verificado contra el propio .aff: `SET UTF-8`.
const esMinuscula = (w: string) => w === w.toLowerCase();

for (const linea of lineasDic) {
  const limpia = linea.trim();
  if (!limpia || limpia.startsWith("#")) continue;
  const [palabra, flags = ""] = limpia.split("/");
  if (!palabra) continue;
  VALIDAS.add(palabra.toLowerCase());
  if (flags) {
    const formas = new Set<string>();
    aplicarSfx(palabra, flags, formas, 1);
    // PFX con cross-product: prefijos sobre la base y sus formas sufijadas.
    for (const f of flags) {
      const clase = pfx.get(f);
      if (!clase) continue;
      for (const r of clase.reglas) {
        for (const base of [palabra, ...formas]) {
          if (!r.cond.test(base)) continue;
          if (r.strip && !base.startsWith(r.strip)) continue;
          formas.add(r.add + base.slice(r.strip.length));
        }
      }
    }
    for (const f of formas) VALIDAS.add(f.toLowerCase());
  }
}

// ---------- pares candidatos ----------

const TILDES = /[áéíóúü]/;
const quitar = (w: string) =>
  w
    .replaceAll("á", "a")
    .replaceAll("é", "e")
    .replaceAll("í", "i")
    .replaceAll("ó", "o")
    .replaceAll("ú", "u")
    .replaceAll("ü", "u");

// Cinturón extra sobre la defensa principal (forma sin tilde válida = fuera).
// En 3 letras viven restauraciones únicas y frecuentísimas ("dia"→"día",
// "ahi"→"ahí", "aca"→"acá"); los monosílabos diacríticos peligrosos (él, sí,
// qué, más) ya quedan fuera por la regla de validez, no por el largo.
const LARGO_MINIMO = 3;

const candidatos = new Map<string, Set<string>>();
for (const w of VALIDAS) {
  if (!TILDES.test(w)) continue;
  if (!esMinuscula(w)) continue; // nada de nombres propios
  if (w.length < LARGO_MINIMO) continue;
  const d = quitar(w);
  if (VALIDAS.has(d)) continue; // LA regla: la forma desnuda existe → fuera
  if (!candidatos.has(d)) candidatos.set(d, new Set());
  candidatos.get(d)!.add(w);
}

// Solo restauraciones con UNA única forma acentuada posible.
const pares: [string, string][] = [];
for (const [d, formas] of candidatos) {
  if (formas.size === 1) pares.push([d, [...formas][0]]);
}
pares.sort((a, b) => a[0].localeCompare(b[0], "es"));

mkdirSync(dirname(SALIDA_AUDIT), { recursive: true });
mkdirSync(dirname(SALIDA_GZ), { recursive: true });
const cabecera = [
  "# tildes-es.tsv — mapa de restauración de tildes (generado, NO editar a mano)",
  "# Generador: scripts/gen-tildes.ts  |  Regla: entra solo si la forma sin",
  "# tilde NO es palabra española válida y la restauración es única.",
  `# Fuente: RLA-ES via LibreOffice/dictionaries (MPL 1.1)`,
  `# es_ES.dic sha256=${sha(dicCrudo)}`,
  `# es_ES.aff sha256=${sha(affCrudo)}`,
  `# pares=${pares.length}`,
].join("\n");
const contenido =
  cabecera + "\n" + pares.map(([d, w]) => `${d}\t${w}`).join("\n") + "\n";
writeFileSync(SALIDA_AUDIT, contenido);
writeFileSync(SALIDA_GZ, Bun.gzipSync(Buffer.from(contenido)));

console.error(`válidas=${VALIDAS.size} pares=${pares.length}`);
console.error(`  audit: ${SALIDA_AUDIT}`);
console.error(`  bundle: ${SALIDA_GZ}`);
// Autochequeo con centinelas VERIFICADOS: si alguno falla, el generador
// cambió de conducta y hay que mirar el diff antes de commitear.
const mapa = new Map(pares);
let malo = false;
for (const debe of [
  "rapido",
  "pidio",
  "cancion",
  "reunion",
  "telefono",
  "corazon",
  "dia",
  "ahi",
  "aca",
]) {
  const v = mapa.get(debe);
  console.error(`  ${debe} → ${v ?? "(AUSENTE)"}`);
  if (!v) malo = true;
}
// "musica" y "publica" NO deben estar: musicar y publicar existen ("él
// musica", "él publica"). Es el filtro estricto haciendo su trabajo; esos
// casos quedan para el LLM contextual.
for (const noDebe of [
  "esta",
  "si",
  "mas",
  "aun",
  "practico",
  "hacia",
  "llego",
  "medico",
  "musica",
  "publica",
]) {
  if (mapa.has(noDebe)) {
    console.error(`  ¡PELIGRO! ${noDebe} → ${mapa.get(noDebe)}`);
    malo = true;
  }
}
if (malo) {
  console.error("Centinelas fallidos: NO commitear este TSV sin revisar.");
  process.exit(1);
}
