import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettings } from "../../hooks/useSettings";
import { SessionAudioRetention } from "@/bindings";

interface SessionRetentionSelectorProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Retención del AUDIO de sesiones (PRP-009, Fase 5). Calcado del selector de
 * retención del historial: mismo patrón, misma UI, otro ciclo de vida.
 */
export const SessionRetentionSelector: React.FC<SessionRetentionSelectorProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const selected = getSetting("session_audio_retention") || "on_document";

    const options = [
      {
        value: "on_document",
        label: t("settings.general.sessionRetention.onDocument"),
      },
      { value: "days_7", label: t("settings.general.sessionRetention.days7") },
      {
        value: "days_30",
        label: t("settings.general.sessionRetention.days30"),
      },
      {
        value: "forever",
        label: t("settings.general.sessionRetention.forever"),
      },
    ];

    return (
      <SettingContainer
        title={t("settings.general.sessionRetention.title")}
        description={t("settings.general.sessionRetention.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Dropdown
          options={options}
          selectedValue={selected}
          onSelect={(value) =>
            updateSetting(
              "session_audio_retention",
              value as SessionAudioRetention,
            )
          }
          placeholder={t("settings.general.sessionRetention.title")}
          disabled={isUpdating("session_audio_retention")}
        />
      </SettingContainer>
    );
  });

SessionRetentionSelector.displayName = "SessionRetentionSelector";
