import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { SettingContainer } from "../ui/SettingContainer";
import { Button } from "../ui/Button";

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
    const [busy, setBusy] = useState(false);

    const refresh = useCallback(async () => {
      const r = await commands.getObsidianVault();
      setVault(r.status === "ok" ? r.data : "");
    }, []);

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
        </div>
      </SettingContainer>
    );
  },
);

ObsidianVault.displayName = "ObsidianVault";
