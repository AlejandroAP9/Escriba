/**
 * Piezas compartidas entre la batería (run.ts) y el benchmark (benchmark.ts):
 * lectura de casos, sandbox portable (ajustes de fábrica + idioma pinneado a
 * "es") e invocación del CLI. Una sola implementación para que el arnés y el
 * benchmark midan EXACTAMENTE el mismo pipeline.
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

export const AQUI = dirname(fileURLToPath(import.meta.url));
export const AUDIO_DIR = join(AQUI, "audio");
export const CASOS_TSV = join(AQUI, "casos.tsv");

const BIN_FUENTE =
  process.env.ESCRIBA_BIN ??
  join(AQUI, "..", "..", "src-tauri", "target", "debug", "escriba");

const SANDBOX = join(AQUI, "sandbox");
export const BIN = join(SANDBOX, "escriba");

export type Caso = { id: string; voz: string; formato: string; texto: string };

export function leerCasos(): Caso[] {
  const filas = readFileSync(CASOS_TSV, "utf8").trim().split("\n").slice(1);
  return filas.map((f) => {
    const [id, voz, formato, texto] = f.split("\t");
    return { id, voz, formato, texto };
  });
}

export function rutaAudio(c: Caso): string {
  return join(AUDIO_DIR, `${c.id}.${c.formato}`);
}

/** Sandbox portable: ajustes de fábrica siempre, idioma pinneado a "es"
 *  (la lección de la primera congelada: el diccionario personal contaminaba
 *  las salidas, y el auto-detect era no determinista). */
export function prepararSandbox() {
  if (!existsSync(BIN_FUENTE)) {
    console.error(
      `No existe el binario: ${BIN_FUENTE} (compila con cargo build)`,
    );
    process.exit(2);
  }
  mkdirSync(SANDBOX, { recursive: true });
  const marcador = join(SANDBOX, "portable");
  if (!existsSync(marcador)) writeFileSync(marcador, "Handy Portable Mode");
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
    const a = statSync(BIN);
    const b = statSync(BIN_FUENTE);
    if (a.ino === b.ino) return;
    rmSync(BIN);
  }
  linkSync(BIN_FUENTE, BIN);
}

export type SalidaCli = {
  text: string;
  audio_secs: number;
  load_ms: number;
  best_ms: number;
  transcribe_ms: number[];
  rtf: number;
};

/** Corre el CLI sobre un audio con un modelo dado y parsea el JSON. */
export function transcribirCli(
  audio: string,
  modelo: string,
  repeat: number,
): SalidaCli {
  const r = spawnSync(
    BIN,
    [
      "--transcribe-file",
      audio,
      "--model",
      modelo,
      "--json",
      "--repeat",
      String(repeat),
    ],
    { encoding: "utf8", timeout: 600_000 },
  );
  if (r.status !== 0) {
    throw new Error(
      `CLI falló (${modelo} · ${audio}): ${r.stderr?.slice(-300)}`,
    );
  }
  const linea = r.stdout.trim().split("\n").at(-1) ?? "";
  return JSON.parse(linea) as SalidaCli;
}
