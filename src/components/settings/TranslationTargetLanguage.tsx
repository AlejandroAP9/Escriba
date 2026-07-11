import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown, SettingContainer } from "@/components/ui";
import { useSettings } from "../../hooks/useSettings";

// Lista curada: idiomas donde Qwen3-4B traduce con calidad validable.
// Los nombres nativos son sustantivos propios, no strings traducibles.
const TARGET_LANGUAGES: { value: string; label: string }[] = [
  { value: "en", label: "English (Inglés)" },
  { value: "es", label: "Español" },
  { value: "pt", label: "Português (Portugués)" },
  { value: "fr", label: "Français (Francés)" },
  { value: "de", label: "Deutsch (Alemán)" },
  { value: "it", label: "Italiano" },
  { value: "zh", label: "中文 (Chino)" },
  { value: "ja", label: "日本語 (Japonés)" },
  { value: "ko", label: "한국어 (Coreano)" },
  { value: "ru", label: "Русский (Ruso)" },
  { value: "lt", label: "Lietuvių (Lituano)" },
  { value: "ar", label: "العربية (Árabe)" },
  { value: "nl", label: "Nederlands (Neerlandés)" },
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
