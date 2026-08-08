import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Dialog } from "../ui/Dialog";
import { Button } from "../ui/Button";
import { Textarea } from "../ui/Textarea";
import { useObsidianStore } from "../../stores/obsidianStore";
import { sendToObsidian } from "@/lib/obsidian";
import { commands } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";

/**
 * Revisar antes de que la nota toque el vault.
 *
 * El vault de alguien es su segundo cerebro, no una bandeja de salida: una
 * exportación que aterriza sin verse obliga a entrar a Obsidian a arreglar el
 * título o a borrar la nota. Aquí se ve y se edita lo que se va a escribir, y
 * nada llega al disco hasta confirmar.
 *
 * Se monta UNA vez en App.tsx; Sesiones y el Estudio lo abren por el store.
 */
export const ObsidianPreviewDialog: React.FC = () => {
  const { t } = useTranslation();
  const { open, title, content, setTitle, setContent, close } =
    useObsidianStore();
  const [saving, setSaving] = useState(false);
  const titleRef = useRef<HTMLInputElement>(null);
  const { getSetting } = useSettings();

  // Enlaces [[...]] (PRP-007): al abrirse el diálogo, las menciones que
  // coinciden con notas del vault llegan ya convertidas, y el usuario las ve
  // y edita ANTES de que nada toque el disco. Una sola vez por apertura: el
  // texto editado a mano no se vuelve a procesar.
  const linkMentions = (getSetting("obsidian_link_mentions") ??
    true) as boolean;
  const [linkedCount, setLinkedCount] = useState<number | null>(null);
  const linkedForOpen = useRef(false);
  useEffect(() => {
    if (!open) {
      linkedForOpen.current = false;
      setLinkedCount(null);
      return;
    }
    if (linkedForOpen.current || !linkMentions) return;
    linkedForOpen.current = true;
    commands
      .linkObsidianMentions(useObsidianStore.getState().content)
      .then((r) => {
        if (r.status === "ok" && r.data.links > 0) {
          setContent(r.data.content);
          setLinkedCount(r.data.links);
        }
      })
      .catch(() => {
        // Sin enlaces no hay drama: el export sigue igual que siempre.
      });
  }, [open, linkMentions, setContent]);

  const save = async () => {
    setSaving(true);
    try {
      // La lógica del vault (elegirlo la primera vez, volver a pedirlo si la
      // carpeta desapareció) sigue viviendo en sendToObsidian, detrás de esta
      // confirmación. Solo se cierra si de verdad se guardó: si cancelaron el
      // selector de carpeta, el diálogo sigue ahí con el texto editado.
      if (await sendToObsidian(title, content, t)) close();
    } finally {
      setSaving(false);
    }
  };

  const canSave = title.trim().length > 0 && content.trim().length > 0;

  return (
    <Dialog
      open={open}
      title={t("obsidian.previewTitle")}
      description={t("obsidian.previewDescription")}
      closeLabel={t("common.close")}
      onOpenChange={(next) => {
        if (!next && !saving) close();
      }}
      initialFocusRef={titleRef}
      footer={
        <div className="flex flex-wrap justify-end gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={close}
            disabled={saving}
          >
            {t("common.cancel")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={save}
            disabled={saving || !canSave}
          >
            {saving ? t("obsidian.saving") : t("obsidian.saveToVault")}
          </Button>
        </div>
      }
    >
      <div className="flex flex-col gap-3">
        <div className="flex flex-col gap-1">
          {/* `htmlFor`/`id` de verdad: el título es un campo con nombre, no un
              texto suelto encima de una caja. */}
          <label
            htmlFor="obsidian-note-title"
            className="text-xs font-medium text-mid-gray"
          >
            {t("obsidian.noteTitle")}
          </label>
          <input
            ref={titleRef}
            id="obsidian-note-title"
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            className="w-full rounded-control border border-line bg-background px-3 py-2 text-sm text-text focus:outline-none focus:ring-1 focus:ring-logo-primary"
          />
          <p className="text-3xs text-mid-gray">
            {t("obsidian.noteTitleHint")}
          </p>
        </div>

        <div className="flex flex-col gap-1">
          <label
            htmlFor="obsidian-note-body"
            className="text-xs font-medium text-mid-gray"
          >
            {t("obsidian.noteBody")}
          </label>
          <Textarea
            id="obsidian-note-body"
            value={content}
            onChange={(e) => setContent(e.target.value)}
            rows={12}
            className="font-mono text-2xs"
          />
          {linkedCount !== null && (
            <p className="text-3xs text-mid-gray">
              {t("obsidian.linkedHint", { count: linkedCount })}
            </p>
          )}
        </div>
      </div>
    </Dialog>
  );
};

ObsidianPreviewDialog.displayName = "ObsidianPreviewDialog";
