import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type ModelInfo } from "@/bindings";

interface RetranscribeMenuProps {
  /** Called with the chosen model id, or null to use the current default model. */
  onRetranscribe: (modelId: string | null) => void;
  disabled?: boolean;
  /** Model id used for the current transcription, to mark it in the list. */
  currentModelId?: string | null;
  /** Etiqueta corta del placeholder (p.ej. "Re-transcribir"); por defecto la larga. */
  label?: string;
}

/**
 * Compact model picker to re-run a transcription with a DIFFERENT model.
 * "Same audio, more accuracy": lists only downloaded models so the user can
 * compare results without leaving the screen. Local, no re-upload.
 */
export const RetranscribeMenu: React.FC<RetranscribeMenuProps> = ({
  onRetranscribe,
  disabled = false,
  currentModelId,
  label,
}) => {
  const { t } = useTranslation();
  const [models, setModels] = useState<ModelInfo[]>([]);

  useEffect(() => {
    commands.getAvailableModels().then((result) => {
      if (result.status === "ok") {
        setModels(result.data.filter((m) => m.is_downloaded));
      }
    });
  }, []);

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const value = e.target.value;
    e.target.value = "";
    if (!value) return;
    onRetranscribe(value === "__default__" ? null : value);
  };

  return (
    <label className="inline-flex items-center gap-1.5 text-sm text-mid-gray">
      <select
        value=""
        onChange={handleChange}
        disabled={disabled}
        title={t("retranscribe.title")}
        className="px-2 py-1 rounded-lg border border-mid-gray/30 bg-background text-text text-xs disabled:opacity-50"
      >
        <option value="">{label ?? t("retranscribe.action")}</option>
        {models.map((m) => (
          <option key={m.id} value={m.id}>
            {m.name}
            {currentModelId === m.id ? ` ${t("retranscribe.currentTag")}` : ""}
          </option>
        ))}
      </select>
    </label>
  );
};
