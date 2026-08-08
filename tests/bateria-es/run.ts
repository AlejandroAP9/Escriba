/**
 * Batería de español difícil (PRP-006, Fase 2).
 *
 * Corre los audios congelados por el CLI de Escriba contra el motor REAL y
 * compara la salida con `esperado.tsv`. Es el arnés de regresión del pipeline
 * completo (decode → motor → correcciones): si una corrección nueva cambia
 * cualquier salida, la batería FALLA y el cambio se acepta conscientemente
 * con `--update`.
 *
 * Modos:
 *   bun tests/bateria-es/run.ts            # correr y comparar (exit 1 si difiere)
 *   bun tests/bateria-es/run.ts --gen      # sintetizar audios que falten (say, macOS)
 *   bun tests/bateria-es/run.ts --update   # congelar las salidas actuales
 *   bun tests/bateria-es/run.ts --solo NUM # correr solo los casos cuyo id contenga NUM
 *
 * El modelo va PINNEADO: mismo audio + mismo modelo + decode greedy = salida
 * determinista. Los audios son voces sintéticas de macOS (7 voces es_CL/ES/MX)
 * como v1 congelada; se pueden sumar grabaciones humanas como casos nuevos sin
 * tocar el runner.
 */
import { spawnSync } from "node:child_process";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import {
  AQUI,
  AUDIO_DIR,
  leerCasos,
  prepararSandbox,
  rutaAudio,
  transcribirCli,
  type Caso,
} from "./comun";

const ESPERADO_TSV = join(AQUI, "esperado.tsv");

// Pinneado: la batería mide el pipeline, no la elección de modelo.
const MODELO =
  process.env.BATERIA_MODELO ??
  "handy-computer/whisper-large-v3-turbo-gguf/whisper-large-v3-turbo-Q8_0.gguf";

function leerEsperado(): Map<string, string> {
  const mapa = new Map<string, string>();
  if (!existsSync(ESPERADO_TSV)) return mapa;
  for (const f of readFileSync(ESPERADO_TSV, "utf8")
    .trim()
    .split("\n")
    .slice(1)) {
    const tab = f.indexOf("\t");
    if (tab > 0) mapa.set(f.slice(0, tab), f.slice(tab + 1));
  }
  return mapa;
}

const args = process.argv.slice(2);
const modoGen = args.includes("--gen");
const modoUpdate = args.includes("--update");
const soloIdx = args.indexOf("--solo");
const filtro = soloIdx >= 0 ? args[soloIdx + 1] : null;

let casos = leerCasos();
if (filtro) casos = casos.filter((c) => c.id.includes(filtro));

if (modoGen) {
  for (const c of casos) generarAudio(c);
  console.log(`Audios listos: ${casos.length} casos.`);
  process.exit(0);
}

// prepararSandbox aborta con mensaje claro si falta el binario.
prepararSandbox();

const esperado = leerEsperado();
const resultados: { id: string; salida: string; ok: boolean }[] = [];
let fallas = 0;

for (const c of casos) {
  if (!existsSync(rutaAudio(c))) {
    console.error(`Falta el audio de ${c.id}: corre primero con --gen`);
    process.exit(2);
  }
  const salida = transcribirCli(rutaAudio(c), MODELO, 1).text.trim();
  if (modoUpdate) {
    resultados.push({ id: c.id, salida, ok: true });
    console.log(`  ${c.id}: "${salida}"`);
  } else {
    const quiere = esperado.get(c.id);
    const ok = quiere !== undefined && quiere === salida;
    if (!ok) {
      fallas++;
      console.log(`✗ ${c.id}`);
      console.log(
        `    esperado: ${quiere === undefined ? "(sin congelar)" : JSON.stringify(quiere)}`,
      );
      console.log(`    obtenido: ${JSON.stringify(salida)}`);
    } else {
      console.log(`✓ ${c.id}`);
    }
    resultados.push({ id: c.id, salida, ok });
  }
}

if (modoUpdate) {
  // Congelar: se preservan las filas de casos no corridos (filtro --solo).
  const todas = leerEsperado();
  for (const r of resultados) todas.set(r.id, r.salida);
  const filas = [
    "id\ttexto",
    ...[...todas.entries()].map(([id, t]) => `${id}\t${t}`),
  ];
  writeFileSync(ESPERADO_TSV, filas.join("\n") + "\n");
  console.log(`Congelados ${resultados.length} casos en esperado.tsv`);
} else {
  console.log(`\n${resultados.length - fallas}/${resultados.length} casos OK`);
  if (fallas > 0) process.exit(1);
}
