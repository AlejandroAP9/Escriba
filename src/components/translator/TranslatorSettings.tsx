import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands } from "@/bindings";
import { Button } from "../ui/Button";

const LANGUAGES: { value: string; label: string }[] = [
  { value: "es", label: "Español" },
  { value: "en", label: "English (Inglés)" },
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
];

type Result = { source: string; target_lang: string; translation: string };

export const TranslatorSettings: React.FC = () => {
  const { t } = useTranslation();
  const [langA, setLangA] = useState("es");
  const [langB, setLangB] = useState("en");
  const [listening, setListening] = useState(false);
  const [last, setLast] = useState<Result | null>(null);
  const [voiceOn, setVoiceOn] = useState(true);

  const speak = useCallback(
    (text: string, lang: string) => {
      if (!voiceOn || !("speechSynthesis" in window)) return;
      window.speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(text);
      u.lang = lang;
      u.rate = 0.98;
      window.speechSynthesis.speak(u);
    },
    [voiceOn],
  );

  useEffect(() => {
    const unlisten = listen<Result>("translator-result", (e) => {
      setLast(e.payload);
      speak(e.payload.translation, e.payload.target_lang);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [speak]);

  const toggleListening = () => {
    const next = !listening;
    setListening(next);
    commands.translatorSetLangs(langA, langB);
    commands.translatorSetListening(next);
  };

  useEffect(() => {
    if (listening) commands.translatorSetLangs(langA, langB);
  }, [langA, langB, listening]);

  const langLabel = (code: string) =>
    LANGUAGES.find((l) => l.value === code)?.label ?? code;

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="rounded-lg border border-mid-gray/30 p-5 space-y-4">
        <div>
          <h3 className="font-semibold text-text">{t("translator.title")}</h3>
          <p className="text-sm text-mid-gray mt-1">
            {t("translator.subtitle")}
          </p>
        </div>

        <div className="flex items-center gap-2">
          <select
            value={langA}
            onChange={(e) => setLangA(e.target.value)}
            className="flex-1 px-3 py-2 rounded-lg border border-mid-gray/30 bg-background text-text text-sm"
          >
            {LANGUAGES.map((l) => (
              <option key={l.value} value={l.value}>
                {l.label}
              </option>
            ))}
          </select>
          <span className="text-mid-gray" aria-hidden>
            {"⇄"}
          </span>
          <select
            value={langB}
            onChange={(e) => setLangB(e.target.value)}
            className="flex-1 px-3 py-2 rounded-lg border border-mid-gray/30 bg-background text-text text-sm"
          >
            {LANGUAGES.map((l) => (
              <option key={l.value} value={l.value}>
                {l.label}
              </option>
            ))}
          </select>
        </div>

        <div className="flex items-center gap-2">
          <Button
            variant={listening ? "secondary" : "primary"}
            size="md"
            onClick={toggleListening}
          >
            {listening ? t("translator.stop") : t("translator.start")}
          </Button>
          <Button
            variant="secondary"
            size="md"
            onClick={() => {
              setVoiceOn((v) => !v);
              if (voiceOn) window.speechSynthesis.cancel();
            }}
          >
            {voiceOn ? t("translator.voiceOn") : t("translator.voiceOff")}
          </Button>
        </div>

        {listening && (
          <p className="text-xs text-mid-gray">{t("translator.hint")}</p>
        )}
      </div>

      {/* Pantalla grande: la traducción para mostrarle a la otra persona */}
      {last && (
        <div className="rounded-xl border-2 border-logo-primary/40 bg-logo-primary/5 p-6 text-center space-y-3">
          <div className="text-xs uppercase tracking-wide text-mid-gray">
            {langLabel(last.target_lang)}
          </div>
          <div className="text-3xl font-semibold text-text leading-snug">
            {last.translation}
          </div>
          <div className="text-sm text-mid-gray italic border-t border-mid-gray/20 pt-3">
            {last.source}
          </div>
        </div>
      )}
    </div>
  );
};
