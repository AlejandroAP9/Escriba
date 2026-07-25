import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Plus, Sparkles } from "lucide-react";
import { commands } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";
import { MicButton } from "../shared/MicButton";

interface CustomWordsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const CustomWords: React.FC<CustomWordsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [newWord, setNewWord] = useState("");
    const customWords = getSetting("custom_words") || [];

    // Sugerencias desde el historial (idea de Benjamín Carreño, comunidad):
    // palabras inusuales que el usuario repite y el modelo puede escribir mal.
    const [suggestions, setSuggestions] = useState<string[]>([]);
    useEffect(() => {
      commands.suggestCustomWords().then((r) => {
        if (r.status === "ok") setSuggestions(r.data);
      });
    }, []);
    const addSuggestion = (word: string) => {
      if (!customWords.includes(word)) {
        updateSetting("custom_words", [...customWords, word]);
      }
      setSuggestions((prev) => prev.filter((w) => w !== word));
    };

    const handleAddWord = () => {
      const trimmedWord = newWord.trim();
      const sanitizedWord = trimmedWord.replace(/[<>"'&]/g, "");
      if (
        sanitizedWord &&
        !sanitizedWord.includes(" ") &&
        sanitizedWord.length <= 50
      ) {
        if (customWords.includes(sanitizedWord)) {
          toast.error(
            t("settings.advanced.customWords.duplicate", {
              word: sanitizedWord,
            }),
          );
          return;
        }
        updateSetting("custom_words", [...customWords, sanitizedWord]);
        setNewWord("");
      }
    };

    const handleRemoveWord = (wordToRemove: string) => {
      updateSetting(
        "custom_words",
        customWords.filter((word) => word !== wordToRemove),
      );
    };

    const handleKeyPress = (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleAddWord();
      }
    };

    return (
      <>
        <SettingContainer
          title={t("settings.advanced.customWords.title")}
          description={t("settings.advanced.customWords.description")}
          descriptionMode={descriptionMode}
          grouped={grouped}
        >
          <div className="flex items-center gap-2">
            <Input
              type="text"
              className="max-w-40"
              value={newWord}
              onChange={(e) => setNewWord(e.target.value)}
              onKeyDown={handleKeyPress}
              placeholder={t("settings.advanced.customWords.placeholder")}
              variant="compact"
              disabled={isUpdating("custom_words")}
            />
            <MicButton
              onText={(text) =>
                setNewWord(text.trim().replace(/[.,!?;:]+$/, ""))
              }
              disabled={isUpdating("custom_words")}
            />
            <Button
              onClick={handleAddWord}
              disabled={
                !newWord.trim() ||
                newWord.includes(" ") ||
                newWord.trim().length > 50 ||
                isUpdating("custom_words")
              }
              variant="primary"
              size="md"
            >
              {t("settings.advanced.customWords.add")}
            </Button>
          </div>
        </SettingContainer>
        {suggestions.length > 0 && (
          <div
            className={`px-4 py-2.5 ${grouped ? "" : "rounded-lg border border-mid-gray/20"}`}
          >
            <p className="mb-1.5 flex items-center gap-1.5 text-2xs font-medium uppercase tracking-wide text-mid-gray">
              <Sparkles width={11} height={11} className="text-gold-text" />
              {t("settings.advanced.customWords.suggestionsTitle")}
            </p>
            <div className="flex flex-wrap gap-1">
              {suggestions.map((word) => (
                <button
                  key={word}
                  type="button"
                  onClick={() => addSuggestion(word)}
                  title={t("settings.advanced.customWords.suggestionsAdd")}
                  className="flex items-center gap-1 rounded-full border border-logo-primary/30 bg-logo-primary/5 px-2.5 py-1 text-xs text-text transition-colors hover:bg-logo-primary/15 focus:outline-none focus:ring-1 focus:ring-logo-primary"
                >
                  <Plus width={11} height={11} className="text-gold-text" />
                  {word}
                </button>
              ))}
            </div>
          </div>
        )}
        {customWords.length > 0 && (
          <div
            className={`px-4 p-2 ${grouped ? "" : "rounded-lg border border-mid-gray/20"} flex flex-wrap gap-1`}
          >
            {customWords.map((word) => (
              <Button
                key={word}
                onClick={() => handleRemoveWord(word)}
                disabled={isUpdating("custom_words")}
                variant="secondary"
                size="sm"
                className="inline-flex items-center gap-1 cursor-pointer"
                aria-label={t("settings.advanced.customWords.remove", { word })}
              >
                <span>{word}</span>
                <svg
                  className="w-3 h-3"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </Button>
            ))}
          </div>
        )}
      </>
    );
  },
);
