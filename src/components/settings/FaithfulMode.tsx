import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { ToggleSwitch } from "../ui/ToggleSwitch";

interface FaithfulModeProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Conserva el texto del motor en el dictado normal. Es deliberadamente un
 * ajuste del atajo principal: los atajos explícitos de Escritura Inteligente,
 * traducción y edición siguen transformando porque ahí existe intención clara.
 */
export const FaithfulMode: React.FC<FaithfulModeProps> = React.memo(
  ({ descriptionMode = "inline", grouped = true }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const enabled = getSetting("faithful_mode_enabled") ?? false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("faithful_mode_enabled", value)}
        isUpdating={isUpdating("faithful_mode_enabled")}
        label={t("settings.general.faithfulMode.label")}
        description={t("settings.general.faithfulMode.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);
