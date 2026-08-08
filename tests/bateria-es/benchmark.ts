/**
 * Benchmark Qwen3-ASR en español (PRP-007, Fase 5).
 *
 * 4 modelos × 40 audios de la batería × repeat 3, por el CLI real en el
 * sandbox portable: mide lo que vive el usuario (pipeline completo, con las
 * mismas correcciones para los 4 motores). Verdad-terreno: la columna
 * `texto` de casos.tsv. `esperado.tsv` NO se usa (es regresión, no verdad).
 *
 * Métricas: WER global y por categoría, best_ms, RTF, load_ms y RAM pico
 * (una corrida representativa por modelo con /usr/bin/time -l).
 *
 * Normalización del WER (congelada aquí, cambiarla exige tocar este bloque):
 * NFC, minúsculas, sin puntuación, espacios colapsados, TILDES CONSERVADAS
 * (son el eje español del benchmark).
 *
 * Fail-closed: si falta un modelo o un audio, aborta. Jamás se omite un
 * contendiente en silencio.
 *
 * Uso: bun tests/bateria-es/benchmark.ts [--repeat 3] [--salida <dir>]
 */
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  AQUI,
  BIN,
  leerCasos,
  prepararSandbox,
  rutaAudio,
  transcribirCli,
} from "./comun";

const args = process.argv.slice(2);
const REPEAT = Number(args[args.indexOf("--repeat") + 1]) || 3;
const SALIDA_DIR =
  args.indexOf("--salida") >= 0
    ? args[args.indexOf("--salida") + 1]
    : join(AQUI, "..", "..", "docs", "benchmarks");

const MODELOS: { clave: string; id: string }[] = [
  {
    clave: "whisper-large-v3-turbo Q8_0",
    id: "handy-computer/whisper-large-v3-turbo-gguf/whisper-large-v3-turbo-Q8_0.gguf",
  },
  {
    clave: "parakeet-tdt-0.6b-v3 Q8_0",
    id: "handy-computer/parakeet-tdt-0.6b-v3-gguf/parakeet-tdt-0.6b-v3-Q8_0.gguf",
  },
  {
    clave: "Qwen3-ASR-0.6B Q8_0",
    id: "handy-computer/Qwen3-ASR-0.6B-gguf/Qwen3-ASR-0.6B-Q8_0.gguf",
  },
  {
    clave: "Qwen3-ASR-1.7B Q8_0",
    id: "handy-computer/Qwen3-ASR-1.7B-gguf/Qwen3-ASR-1.7B-Q8_0.gguf",
  },
];

/** Normalización congelada del WER (ver cabecera). */
function normalizar(s: string): string {
  return s
    .normalize("NFC")
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s]/gu, " ")
    .replace(/\s+/g, " ")
    .trim();
}

/** Distancia de Levenshtein sobre PALABRAS. */
function distanciaPalabras(ref: string[], hip: string[]): number {
  const n = ref.length;
  const m = hip.length;
  let prev = Array.from({ length: m + 1 }, (_, j) => j);
  for (let i = 1; i <= n; i++) {
    const fila = [i, ...Array(m).fill(0)];
    for (let j = 1; j <= m; j++) {
      const costo = ref[i - 1] === hip[j - 1] ? 0 : 1;
      fila[j] = Math.min(fila[j - 1] + 1, prev[j] + 1, prev[j - 1] + costo);
    }
    prev = fila;
  }
  return prev[m];
}

// ---------- fail-closed: todo presente antes de medir nada ----------
prepararSandbox();
const casos = leerCasos();
for (const c of casos) {
  if (!existsSync(rutaAudio(c))) {
    console.error(`ABORTO: falta el audio de ${c.id} (corre run.ts --gen)`);
    process.exit(2);
  }
}
{
  const r = spawnSync(BIN, ["--list-models", "--json"], { encoding: "utf8" });
  const instalados = new Set(
    (
      JSON.parse(r.stdout.trim().split("\n").at(-1) ?? "[]") as {
        id: string;
        is_downloaded: boolean;
      }[]
    )
      .filter((m) => m.is_downloaded)
      .map((m) => m.id),
  );
  for (const m of MODELOS) {
    if (!instalados.has(m.id)) {
      console.error(
        `ABORTO: el modelo ${m.id} no está instalado (corre descargar-benchmark.ts)`,
      );
      process.exit(2);
    }
  }
}

// ---------- la corrida ----------
type PorAudio = {
  caso: string;
  categoria: string;
  wer: number;
  errores: number;
  palabras_ref: number;
  best_ms: number;
  load_ms: number;
  audio_secs: number;
  texto: string;
};
type PorModelo = {
  modelo: string;
  id: string;
  ram_pico_mb: number | null;
  audios: PorAudio[];
};

const resultados: PorModelo[] = [];
for (const modelo of MODELOS) {
  console.error(`\n=== ${modelo.clave} ===`);
  const audios: PorAudio[] = [];
  for (const c of casos) {
    const salida = transcribirCli(rutaAudio(c), modelo.id, REPEAT);
    const ref = normalizar(c.texto).split(" ").filter(Boolean);
    const hip = normalizar(salida.text).split(" ").filter(Boolean);
    const errores = distanciaPalabras(ref, hip);
    audios.push({
      caso: c.id,
      categoria: c.id.split("-")[0],
      wer: ref.length ? errores / ref.length : 0,
      errores,
      palabras_ref: ref.length,
      best_ms: salida.best_ms,
      load_ms: salida.load_ms,
      audio_secs: salida.audio_secs,
      texto: salida.text,
    });
    console.error(
      `  ${c.id}: wer=${(audios.at(-1)!.wer * 100).toFixed(0)}% best=${salida.best_ms}ms`,
    );
  }
  // RAM pico: una corrida representativa (GEN-07, la frase más larga).
  let ram: number | null = null;
  const rep = casos.find((c) => c.id === "GEN-07") ?? casos[0];
  const t = spawnSync(
    "/usr/bin/time",
    [
      "-l",
      BIN,
      "--transcribe-file",
      rutaAudio(rep),
      "--model",
      modelo.id,
      "--json",
    ],
    { encoding: "utf8", timeout: 600_000 },
  );
  const m = t.stderr?.match(/(\d+)\s+maximum resident set size/);
  if (m) ram = Math.round(Number(m[1]) / (1024 * 1024));
  resultados.push({
    modelo: modelo.clave,
    id: modelo.id,
    ram_pico_mb: ram,
    audios,
  });
}

// ---------- agregados y salida ----------
function agrega(audios: PorAudio[]) {
  const errores = audios.reduce((s, a) => s + a.errores, 0);
  const palabras = audios.reduce((s, a) => s + a.palabras_ref, 0);
  const audioSecs = audios.reduce((s, a) => s + a.audio_secs, 0);
  const bestSecs = audios.reduce((s, a) => s + a.best_ms, 0) / 1000;
  return {
    wer: palabras ? errores / palabras : 0,
    best_ms_prom: audios.reduce((s, a) => s + a.best_ms, 0) / audios.length,
    rtf: bestSecs > 0 ? audioSecs / bestSecs : 0,
    load_ms_prom: audios.reduce((s, a) => s + a.load_ms, 0) / audios.length,
  };
}

const CATS = ["TIL", "AMB", "EMO", "NUM", "GEN"];
let md = `| Modelo | WER | ${CATS.map((c) => `WER ${c}`).join(" | ")} | best_ms | RTF | load_ms | RAM pico |\n`;
md += `|---|---|${CATS.map(() => "---").join("|")}|---|---|---|---|\n`;
for (const r of resultados) {
  const g = agrega(r.audios);
  const porCat = CATS.map((cat) => {
    const a = agrega(r.audios.filter((x) => x.categoria === cat));
    return `${(a.wer * 100).toFixed(1)}%`;
  });
  md += `| ${r.modelo} | **${(g.wer * 100).toFixed(1)}%** | ${porCat.join(" | ")} | ${g.best_ms_prom.toFixed(0)} | ${g.rtf.toFixed(1)}x | ${g.load_ms_prom.toFixed(0)} | ${r.ram_pico_mb ?? "?"} MB |\n`;
}

mkdirSync(SALIDA_DIR, { recursive: true });
writeFileSync(
  join(SALIDA_DIR, "qwen3-asr-es.crudo.json"),
  JSON.stringify({ repeat: REPEAT, resultados }, null, 1),
);
console.log(md);
console.error(`\nJSON crudo: ${join(SALIDA_DIR, "qwen3-asr-es.crudo.json")}`);
