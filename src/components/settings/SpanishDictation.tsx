import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface SpanishDictationProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Español profundo (PRP-006): interruptores de las correcciones deterministas
 * del dictado en español. La restauración de tildes NO tiene interruptor
 * (solo toca pares inequívocos y corre siempre); los emojis dictados nacen
 * apagados por decisión de producto: un emoji inesperado en un correo
 * profesional cuesta más que encender esto una vez.
 */
export const SpanishDictation: React.FC<SpanishDictationProps> = React.memo(
  ({ descriptionMode = "inline", grouped = true }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const emojis = getSetting("dictated_emojis_enabled") ?? false;
    const numerales = getSetting("spoken_numerals_enabled") ?? false;
    const planilla = getSetting("numerals_spreadsheet_auto") ?? false;

    return (
      <>
        <ToggleSwitch
          checked={emojis}
          onChange={(v) => updateSetting("dictated_emojis_enabled", v)}
          isUpdating={isUpdating("dictated_emojis_enabled")}
          label={t("settings.general.dictatedEmojis.label")}
          description={t("settings.general.dictatedEmojis.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
        <ToggleSwitch
          checked={numerales}
          onChange={(v) => updateSetting("spoken_numerals_enabled", v)}
          isUpdating={isUpdating("spoken_numerals_enabled")}
          label={t("settings.general.spokenNumerals.label")}
          description={t("settings.general.spokenNumerals.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
        <ToggleSwitch
          checked={planilla}
          onChange={(v) => updateSetting("numerals_spreadsheet_auto", v)}
          isUpdating={isUpdating("numerals_spreadsheet_auto")}
          label={t("settings.general.numeralsSpreadsheet.label")}
          description={t("settings.general.numeralsSpreadsheet.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
      </>
    );
  },
);
