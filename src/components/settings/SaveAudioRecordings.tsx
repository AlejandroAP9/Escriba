import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface SaveAudioRecordingsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Guardar el .wav de cada dictado en el disco.
 *
 * Viene apagado. La grabación de tu propia voz es el dato más sensible que
 * produce la app y antes se escribía siempre, sin que nadie lo pidiera: quien
 * no entraba a Ajustes acumulaba grabaciones que además se van en cualquier
 * copia de seguridad. El texto de la transcripción sí se guarda igual, que es
 * lo que hace útil al historial.
 *
 * Encenderlo habilita el reproductor del historial, que sin audio no tiene nada
 * que reproducir.
 */
export const SaveAudioRecordings: React.FC<SaveAudioRecordingsProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const enabled = getSetting("save_audio_recordings") || false;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(value) => updateSetting("save_audio_recordings", value)}
        isUpdating={isUpdating("save_audio_recordings")}
        label={t("settings.privacy.saveAudio.label")}
        description={t("settings.privacy.saveAudio.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });

SaveAudioRecordings.displayName = "SaveAudioRecordings";
