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
  linkSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const AQUI = dirname(fileURLToPath(import.meta.url));
const AUDIO_DIR = join(AQUI, "audio");
const CASOS_TSV = join(AQUI, "casos.tsv");
const ESPERADO_TSV = join(AQUI, "esperado.tsv");

// Pinneado: la batería mide el pipeline, no la elección de modelo.
const MODELO =
  process.env.BATERIA_MODELO ??
  "handy-computer/whisper-large-v3-turbo-gguf/whisper-large-v3-turbo-Q8_0.gguf";

const BIN_FUENTE =
  process.env.ESCRIBA_BIN ??
  join(AQUI, "..", "..", "src-tauri", "target", "debug", "escriba");

// La batería corre en MODO PORTABLE dentro de un sandbox propio: hardlink del
// binario + marcador `portable` → ajustes de FÁBRICA en sandbox/Data. Sin esto
// el arnés heredaba los ajustes del usuario (la primera congelada salió con el
// diccionario personal convirtiendo "escrita" en "Escriba") y las salidas no
// eran comparables entre máquinas. Los modelos viven en la caché compartida de
// HuggingFace, así que el sandbox no los re-descarga.
const SANDBOX = join(AQUI, "sandbox");
const BIN = join(SANDBOX, "escriba");

function prepararSandbox() {
  mkdirSync(SANDBOX, { recursive: true });
  const marcador = join(SANDBOX, "portable");
  if (!existsSync(marcador)) writeFileSync(marcador, "Handy Portable Mode");
  // Idioma PINNEADO a "es": con "auto", un audio dudoso hace que la detección
  // de idioma caiga al fallback de temperatura y la salida deja de ser
  // determinista (EMO-04 congeló una alucinación en ruso y a la corrida
  // siguiente dio "planet"). La batería mide el pipeline español, no el
  // detector de idioma.
  const store = join(SANDBOX, "Data", "settings_store.json");
  mkdirSync(dirname(store), { recursive: true });
  let datos: { settings?: Record<string, unknown> } = {};
  if (existsSync(store)) {
    try {
      datos = JSON.parse(readFileSync(store, "utf8"));
    } catch {
      datos = {};
    }
  }
  if (datos.settings?.selected_language !== "es") {
    datos.settings = { ...(datos.settings ?? {}), selected_language: "es" };
    writeFileSync(store, JSON.stringify(datos));
  }
  if (existsSync(BIN)) {
    // Refrescar el hardlink si el binario fuente cambió (otro inode o más nuevo).
    const a = statSync(BIN);
    const b = statSync(BIN_FUENTE);
    if (a.ino === b.ino) return;
    rmSync(BIN);
  }
  linkSync(BIN_FUENTE, BIN);
}

type Caso = { id: string; voz: string; formato: string; texto: string };

function leerCasos(): Caso[] {
  const filas = readFileSync(CASOS_TSV, "utf8").trim().split("\n").slice(1);
  return filas.map((f) => {
    const [id, voz, formato, texto] = f.split("\t");
    return { id, voz, formato, texto };
  });
}

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

function rutaAudio(c: Caso): string {
  return join(AUDIO_DIR, `${c.id}.${c.formato}`);
}

/** Sintetiza el audio de un caso con `say` (macOS) y lo convierte al formato. */
function generarAudio(c: Caso) {
  mkdirSync(AUDIO_DIR, { recursive: true });
  const destino = rutaAudio(c);
  if (existsSync(destino)) return;
  const aiff = join(AUDIO_DIR, `${c.id}.tmp.aiff`);
  const say = spawnSync("say", ["-v", c.voz, "-o", aiff, c.texto]);
  if (say.status !== 0) {
    throw new Error(`say falló para ${c.id} (voz "${c.voz}"): ${say.stderr}`);
  }
  const args =
    c.formato === "wav"
      ? ["-f", "WAVE", "-d", "LEI16@16000", "-c", "1", aiff, destino]
      : ["-f", "m4af", "-d", "aac", aiff, destino];
  const af = spawnSync("afconvert", args);
  rmSync(aiff, { force: true });
  if (af.status !== 0) {
    throw new Error(`afconvert falló para ${c.id}: ${af.stderr}`);
  }
  console.log(`  audio generado: ${c.id}.${c.formato} (${c.voz})`);
}

function transcribir(c: Caso): string {
  const r = spawnSync(
    BIN,
    ["--transcribe-file", rutaAudio(c), "--model", MODELO, "--json"],
    {
      encoding: "utf8",
      timeout: 180_000,
    },
  );
  if (r.status !== 0) {
    throw new Error(
      `CLI falló para ${c.id} (exit ${r.status}): ${r.stderr?.slice(-300)}`,
    );
  }
  // La última línea de stdout es el JSON (los logs van a stderr).
  const linea = r.stdout.trim().split("\n").at(-1) ?? "";
  return (JSON.parse(linea).text as string).trim();
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

if (!existsSync(BIN_FUENTE)) {
  console.error(
    `No existe el binario: ${BIN_FUENTE} (compila con cargo build o exporta ESCRIBA_BIN)`,
  );
  process.exit(2);
}
prepararSandbox();

const esperado = leerEsperado();
const resultados: { id: string; salida: string; ok: boolean }[] = [];
let fallas = 0;

for (const c of casos) {
  if (!existsSync(rutaAudio(c))) {
    console.error(`Falta el audio de ${c.id}: corre primero con --gen`);
    process.exit(2);
  }
  const salida = transcribir(c);
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
