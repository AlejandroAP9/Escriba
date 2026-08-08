/**
 * Setup del benchmark Qwen3-ASR (PRP-007, Fase 4): descarga los GGUF que
 * faltan a la caché HF estándar (~/.cache/huggingface/hub) con el MISMO
 * layout que produce huggingface_hub (blobs/<sha256> + snapshots/<rev>/ +
 * refs/main), verificado contra el turbo ya instalado.
 *
 * Es un paso de setup EXPLÍCITO con red, como cualquier descarga del gestor
 * de modelos; el benchmark en sí corre offline. Fail-closed: cualquier
 * descarga incompleta o con tamaño distinto al declarado en catalog.json
 * aborta con mensaje claro.
 *
 * Uso: bun tests/bateria-es/descargar-benchmark.ts
 */
import { createHash } from "node:crypto";
import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { homedir } from "node:os";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const AQUI = dirname(fileURLToPath(import.meta.url));
const HUB = join(homedir(), ".cache", "huggingface", "hub");
const CATALOGO = join(
  AQUI,
  "..",
  "..",
  "src-tauri",
  "src",
  "catalog",
  "catalog.json",
);

/** Los contendientes del benchmark, todos Q8_0 (mismo quant que el turbo
 *  instalado: comparación pareja). */
const OBJETIVOS = [
  {
    repo: "handy-computer/Qwen3-ASR-0.6B-gguf",
    archivo: "Qwen3-ASR-0.6B-Q8_0.gguf",
  },
  {
    repo: "handy-computer/Qwen3-ASR-1.7B-gguf",
    archivo: "Qwen3-ASR-1.7B-Q8_0.gguf",
  },
  {
    repo: "handy-computer/parakeet-tdt-0.6b-v3-gguf",
    archivo: "parakeet-tdt-0.6b-v3-Q8_0.gguf",
  },
];

type EntradaCatalogo = {
  id: string;
  files?: { filename: string; size_bytes: number }[];
};
const catalogoCrudo = JSON.parse(readFileSync(CATALOGO, "utf8"));
const catalogo: EntradaCatalogo[] = Array.isArray(catalogoCrudo)
  ? catalogoCrudo
  : (catalogoCrudo.models ?? []);

function tamanoDeclarado(repo: string, archivo: string): number {
  const entrada = catalogo.find((m) => m.id === repo);
  const f = entrada?.files?.find((f) => f.filename === archivo);
  if (!f) {
    console.error(`ABORTO: ${repo}/${archivo} no está en catalog.json`);
    process.exit(2);
  }
  return f.size_bytes;
}

async function revision(repo: string): Promise<string> {
  const r = await fetch(`https://huggingface.co/api/models/${repo}`);
  if (!r.ok) {
    console.error(
      `ABORTO: no se pudo resolver la revisión de ${repo} (HTTP ${r.status})`,
    );
    process.exit(2);
  }
  const sha = ((await r.json()) as { sha?: string }).sha;
  if (!sha) {
    console.error(`ABORTO: la API de HF no devolvió revisión para ${repo}`);
    process.exit(2);
  }
  return sha;
}

async function descargar(repo: string, archivo: string) {
  const esperado = tamanoDeclarado(repo, archivo);
  const dirModelo = join(HUB, `models--${repo.replaceAll("/", "--")}`);
  const rev = await revision(repo);
  const rutaFinal = join(dirModelo, "snapshots", rev, archivo);
  if (existsSync(rutaFinal) && statSync(rutaFinal).size === esperado) {
    console.log(`ya instalado: ${archivo}`);
    return;
  }

  console.log(`descargando ${archivo} (${(esperado / 1e6).toFixed(0)} MB)…`);
  const url = `https://huggingface.co/${repo}/resolve/main/${archivo}`;
  const r = await fetch(url);
  if (!r.ok || !r.body) {
    console.error(`ABORTO: descarga de ${archivo} falló (HTTP ${r.status})`);
    process.exit(2);
  }
  mkdirSync(join(dirModelo, "blobs"), { recursive: true });
  const tmp = join(dirModelo, "blobs", `${archivo}.incompleto`);
  const hash = createHash("sha256");
  const out = Bun.file(tmp).writer();
  let bajado = 0;
  for await (const chunk of r.body) {
    hash.update(chunk);
    out.write(chunk);
    bajado += chunk.length;
  }
  await out.end();

  // Fail-closed: el tamaño DEBE calzar con el declarado en catalog.json.
  if (bajado !== esperado) {
    rmSync(tmp, { force: true });
    console.error(
      `ABORTO: ${archivo} bajó ${bajado} bytes y el catálogo declara ${esperado}. Re-corre el script.`,
    );
    process.exit(2);
  }
  const sha256 = hash.digest("hex");
  const blob = join(dirModelo, "blobs", sha256);
  renameSync(tmp, blob);
  mkdirSync(join(dirModelo, "snapshots", rev), { recursive: true });
  // Symlink relativo, igual que huggingface_hub.
  if (!existsSync(rutaFinal)) {
    symlinkSync(join("..", "..", "blobs", sha256), rutaFinal);
  }
  mkdirSync(join(dirModelo, "refs"), { recursive: true });
  writeFileSync(join(dirModelo, "refs", "main"), rev);
  console.log(`listo: ${archivo} (sha256 ${sha256.slice(0, 12)}…)`);
}

for (const o of OBJETIVOS) {
  await descargar(o.repo, o.archivo);
}
console.log("Setup del benchmark completo.");
