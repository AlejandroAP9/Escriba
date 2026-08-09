import React from "react";
import { useTranslation } from "react-i18next";
import type { AppSettings } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { SettingContainer } from "../ui/SettingContainer";

type VoiceEngineSetting =
  | "conversation_voice_engine"
  | "interpreter_voice_engine"
  | "read_selection_voice_engine";

interface VoiceEngineSelectorProps {
  setting: VoiceEngineSetting;
  titleKey: string;
  descriptionKey: string;
  grouped?: boolean;
}

const ENGINES = ["system", "included"] as const;

export const VoiceEngineSelector: React.FC<VoiceEngineSelectorProps> = ({
  setting,
  titleKey,
  descriptionKey,
  grouped = true,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const selected = getSetting(setting) === "included" ? "included" : "system";

  return (
    <SettingContainer
      title={t(titleKey)}
      description={t(descriptionKey)}
      grouped={grouped}
      layout="horizontal"
    >
      <div
        className="flex items-center gap-1 rounded-lg border border-line bg-background p-0.5"
        role="group"
        aria-label={t(titleKey)}
      >
        {ENGINES.map((engine) => (
          <button
            key={engine}
            type="button"
            onClick={() =>
              updateSetting(setting, engine as AppSettings[VoiceEngineSetting])
            }
            disabled={isUpdating(setting)}
            aria-pressed={selected === engine}
            className={`rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors focus:outline-none focus:ring-1 focus:ring-logo-primary disabled:opacity-50 ${
              selected === engine
                ? "bg-logo-primary/15 text-gold-text"
                : "text-mid-gray hover:text-text"
            }`}
          >
            {t(`conversation.voiceEngine.${engine}`)}
          </button>
        ))}
      </div>
    </SettingContainer>
  );
};
