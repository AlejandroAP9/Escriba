import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown, SettingContainer } from "@/components/ui";
import { useSettings } from "../../hooks/useSettings";

// Lista curada: idiomas donde Qwen3-4B traduce con calidad validable.
// Los nombres nativos son sustantivos propios, no strings traducibles.
const TARGET_LANGUAGES: { value: string; label: string }[] = [
  { value: "en", label: "English" },
  { value: "es", label: "Español" },
  { value: "pt", label: "Português" },
  { value: "fr", label: "Français" },
  { value: "de", label: "Deutsch" },
  { value: "it", label: "Italiano" },
  { value: "zh", label: "中文" },
  { value: "ja", label: "日本語" },
  { value: "ko", label: "한국어" },
  { value: "ru", label: "Русский" },
  { value: "lt", label: "Lietuvių" },
  { value: "ar", label: "العربية" },
  { value: "nl", label: "Nederlands" },
];

export const TranslationTargetLanguage: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const current = getSetting("translation_target_language") ?? "en";

  return (
    <SettingContainer
      title={t("settings.postProcessing.translation.targetLanguage.title")}
      description={t(
        "settings.postProcessing.translation.targetLanguage.description",
      )}
      descriptionMode="tooltip"
      layout="horizontal"
      grouped={true}
    >
      <Dropdown
        selectedValue={current}
        options={TARGET_LANGUAGES}
        onSelect={(value) =>
          updateSetting("translation_target_language", value)
        }
        disabled={false}
      />
    </SettingContainer>
  );
};
