import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";
import { Button } from "../ui/Button";
import { ToggleSwitch } from "../ui/ToggleSwitch";

interface ObsidianVaultProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Carpeta del vault de Obsidian.
 *
 * El backend ya tenía `get_obsidian_vault` desde que se añadió la exportación,
 * pero ninguna pantalla lo llamaba: la única forma de configurar el vault era
 * pulsar "Enviar a Obsidian" y toparse con un selector de carpetas sin
 * contexto, y después no había manera de ver cuál habías elegido, cambiarlo ni
 * olvidarlo. Esta pantalla cierra ese hueco.
 */
export const ObsidianVault: React.FC<ObsidianVaultProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const [vault, setVault] = useState<string | null>(null);
    const [folder, setFolder] = useState("");
    const [busy, setBusy] = useState(false);
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const refresh = useCallback(async () => {
      const r = await commands.getObsidianVault();
      setVault(r.status === "ok" ? r.data : "");
    }, []);

    // El valor persistido manda: el backend lo guarda ya saneado, así que si
    // escribiste algo que no vale, al volver ves lo que de verdad se usa.
    const stored = (getSetting("obsidian_notes_folder") ?? "") as string;
    useEffect(() => {
      setFolder(stored);
    }, [stored]);

    const saveFolder = async () => {
      if (folder === stored) return;
      await commands.setObsidianNotesFolder(folder);
    };

    useEffect(() => {
      refresh();
    }, [refresh]);

    const choose = async () => {
      const folder = await openDialog({
        directory: true,
        multiple: false,
        title: t("obsidian.pickVault"),
      });
      if (typeof folder !== "string") return; // cancelado
      setBusy(true);
      const saved = await commands.setObsidianVault(folder);
      setBusy(false);
      if (saved.status === "error") {
        toast.error(saved.error);
        return;
      }
      await refresh();
      toast.success(t("obsidian.vaultSaved"));
    };

    const forget = async () => {
      setBusy(true);
      // Cadena vacía es el "olvidar" que el backend ya entiende.
      await commands.setObsidianVault("");
      setBusy(false);
      await refresh();
    };

    const configured = !!vault;

    return (
      <SettingContainer
        title={t("obsidian.vaultTitle")}
        description={t("obsidian.vaultDescription")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="stacked"
      >
        <div className="flex flex-col gap-2">
          {/* La ruta puede ser larga: se parte en vez de ensanchar la fila. */}
          <p
            className={`min-w-0 break-all font-mono text-2xs ${
              configured ? "text-text" : "text-mid-gray"
            }`}
          >
            {configured ? vault : t("obsidian.vaultNotSet")}
          </p>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="secondary"
              size="sm"
              onClick={choose}
              disabled={busy}
            >
              {configured
                ? t("obsidian.vaultChange")
                : t("obsidian.vaultChoose")}
            </Button>
            {configured && (
              <Button
                variant="danger-ghost"
                size="sm"
                onClick={forget}
                disabled={busy}
              >
                {t("obsidian.vaultForget")}
              </Button>
            )}
          </div>

          {/* La subcarpeta solo tiene sentido con un vault elegido. Se crea
              sola al exportar, así que aquí no hay nada que hacer más que
              ponerle nombre. */}
          {configured && (
            <div className="mt-1 flex flex-col gap-1">
              <label
                htmlFor="obsidian-notes-folder"
                className="text-xs font-medium text-mid-gray"
              >
                {t("obsidian.folderLabel")}
              </label>
              <input
                id="obsidian-notes-folder"
                type="text"
                value={folder}
                placeholder={t("obsidian.folderPlaceholder")}
                onChange={(e) => setFolder(e.target.value)}
                // Se guarda al salir del campo, no en cada tecla: el backend
                // sanea el nombre y devolverlo mientras escribes movería el
                // cursor a mitad de palabra.
                onBlur={saveFolder}
                className="w-full rounded-control border border-line bg-background px-3 py-2 font-mono text-2xs text-text focus:outline-none focus:ring-1 focus:ring-logo-primary"
              />
              <p className="text-3xs text-mid-gray">
                {folder.trim()
                  ? t("obsidian.folderHint", { folder: folder.trim() })
                  : t("obsidian.folderHintRoot")}
              </p>
            </div>
          )}

          {/* Obsidian enlazable (PRP-007): enlaces e índice pasan por la
              vista previa; el inbox es la vía rápida y nace apagado. */}
          {configured && (
            <div className="mt-2 flex flex-col gap-2">
              <ToggleSwitch
                checked={
                  (getSetting("obsidian_link_mentions") ?? true) as boolean
                }
                onChange={(v) => updateSetting("obsidian_link_mentions", v)}
                isUpdating={isUpdating("obsidian_link_mentions")}
                label={t("obsidian.linkMentions.label")}
                description={t("obsidian.linkMentions.description")}
                descriptionMode="inline"
                grouped
              />
              <ToggleSwitch
                checked={(getSetting("obsidian_index_note") ?? true) as boolean}
                onChange={(v) => updateSetting("obsidian_index_note", v)}
                isUpdating={isUpdating("obsidian_index_note")}
                label={t("obsidian.indexNote.label")}
                description={t("obsidian.indexNote.description")}
                descriptionMode="inline"
                grouped
              />
              <ToggleSwitch
                checked={
                  (getSetting("obsidian_daily_inbox") ?? false) as boolean
                }
                onChange={(v) => updateSetting("obsidian_daily_inbox", v)}
                isUpdating={isUpdating("obsidian_daily_inbox")}
                label={t("obsidian.dailyInbox.label")}
                description={t("obsidian.dailyInbox.description")}
                descriptionMode="inline"
                grouped
              />
              {/* El índice y las menciones enlazables siguen lo que
                  Takhygraphe hizo mejor que nosotros durante el concurso: el
                  crédito va aquí, donde se usa. */}
              <p className="pt-1 text-3xs leading-relaxed text-mid-gray">
                {t("obsidian.credit")}
              </p>
            </div>
          )}
        </div>
      </SettingContainer>
    );
  },
);

ObsidianVault.displayName = "ObsidianVault";
