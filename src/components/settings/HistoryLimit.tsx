import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";

interface HistoryLimitProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const HistoryLimit: React.FC<HistoryLimitProps> = ({
  descriptionMode = "inline",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const historyLimit = (getSetting("history_limit") ?? 5) as number;

  // Estado local para no persistir en cada tecla: bajar el límite ejecuta un
  // borrado IRREVERSIBLE de grabaciones (issue Handy #1281). Solo commiteamos
  // al salir del campo o con Enter, ya con el valor final.
  const [draft, setDraft] = useState(String(historyLimit));
  useEffect(() => {
    setDraft(String(historyLimit));
  }, [historyLimit]);

  const commit = () => {
    const value = parseInt(draft, 10);
    if (isNaN(value) || value < 0) {
      setDraft(String(historyLimit)); // revertir entrada inválida
      return;
    }
    const clamped = Math.min(value, 1000);
    if (clamped !== historyLimit) {
      updateSetting("history_limit", clamped);
    }
    setDraft(String(clamped));
  };

  return (
    <SettingContainer
      title={t("settings.debug.historyLimit.title")}
      description={t("settings.debug.historyLimit.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="horizontal"
    >
      <div className="flex items-center space-x-2">
        <Input
          type="number"
          min="0"
          max="1000"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
          }}
          disabled={isUpdating("history_limit")}
          className="w-20"
        />
        <span className="text-sm text-text">
          {t("settings.debug.historyLimit.entries")}
        </span>
      </div>
    </SettingContainer>
  );
};
