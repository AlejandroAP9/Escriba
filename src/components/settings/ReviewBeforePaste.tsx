import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { ToggleSwitch } from "../ui/ToggleSwitch";

/**
 * Revisar antes de pegar: el dictado se muestra en el overlay (Pegar /
 * Descartar / corregir dictando) en vez de escribirse directo. Apagado por
 * defecto: el flujo rápido de Escriba es sagrado; esto es para correos
 * delicados y documentos formales.
 */
export const ReviewBeforePaste: React.FC<{
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}> = ({ descriptionMode = "tooltip", grouped = false }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = (getSetting("review_before_paste") ?? false) as boolean;

  return (
    <ToggleSwitch
      checked={enabled}
      onChange={(v) => updateSetting("review_before_paste", v)}
      label={t("settings.general.reviewBeforePaste.label")}
      description={t("settings.general.reviewBeforePaste.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      isUpdating={isUpdating("review_before_paste")}
    />
  );
};
