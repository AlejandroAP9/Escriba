import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import type { TFunction } from "i18next";
import { commands } from "@/bindings";

/**
 * Enviar a Obsidian, compartido por Sesiones y el Estudio. Flujo perezoso: la
 * primera vez (o si la carpeta ya no existe) abre el selector para elegir el
 * vault, lo guarda y reintenta. El backend escribe la nota Markdown. Muestra
 * el resultado con un toast; no lanza excepciones al llamador.
 */
export async function sendToObsidian(
  title: string,
  content: string,
  t: TFunction,
): Promise<void> {
  const write = async (): Promise<
    { ok: true; path: string } | { ok: false; code: string }
  > => {
    const r = await commands.exportToObsidian(title, content);
    if (r.status === "ok") return { ok: true, path: r.data };
    return { ok: false, code: r.error };
  };

  let result = await write();

  // Falta configurar el vault (o la carpeta se movió): pedir la carpeta.
  if (
    !result.ok &&
    (result.code === "SIN_VAULT" || result.code === "VAULT_NO_EXISTE")
  ) {
    // Avisar ANTES de abrir el selector. Sin esto, pulsabas "Enviar a
    // Obsidian" y te aparecía un explorador de carpetas sin explicación:
    // parecía un paso de cada envío cuando es la configuración de una sola
    // vez. El toast dura lo justo para leerse mientras se abre el diálogo.
    toast.info(
      result.code === "SIN_VAULT"
        ? t("obsidian.firstTimeHint")
        : t("obsidian.vaultMissingHint"),
    );
    const folder = await openDialog({
      directory: true,
      multiple: false,
      title: t("obsidian.pickVault"),
    });
    if (typeof folder !== "string") return; // el usuario canceló
    const saved = await commands.setObsidianVault(folder);
    if (saved.status === "error") {
      toast.error(saved.error);
      return;
    }
    result = await write();
  }

  if (result.ok) {
    toast.success(t("obsidian.sent"));
  } else {
    toast.error(t("obsidian.failed"));
  }
}
