import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface NoiseSuppressionProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Supresión de ruido de fondo (RNNoise) sobre el audio del micrófono antes de
 * transcribir: limpia ventilador, teclado, tráfico, etc. Todo local.
 */
export const NoiseSuppression: React.FC<NoiseSuppressionProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const enabled = getSetting("noise_suppression") || false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("noise_suppression", value)}
        isUpdating={isUpdating("noise_suppression")}
        label={t("settings.sound.noiseSuppression.label")}
        description={t("settings.sound.noiseSuppression.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);

NoiseSuppression.displayName = "NoiseSuppression";
