import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";
import { useOsType } from "../../hooks/useOsType";

interface PauseMediaOnDictateProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Pausa la reproducción de medios (Música/Spotify) mientras dictas y la reanuda
 * al terminar. La implementación actual usa AppleScript, así que solo se ofrece
 * en macOS.
 */
export const PauseMediaOnDictate: React.FC<PauseMediaOnDictateProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const os = useOsType();

    if (os !== "macos") return null;

    const enabled = getSetting("pause_media_on_dictate") || false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("pause_media_on_dictate", value)}
        isUpdating={isUpdating("pause_media_on_dictate")}
        label={t("settings.sound.pauseMedia.label")}
        description={t("settings.sound.pauseMedia.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });

PauseMediaOnDictate.displayName = "PauseMediaOnDictate";
