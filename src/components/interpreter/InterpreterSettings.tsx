import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type InterpreterRoom } from "@/bindings";
import { Button } from "../ui/Button";

const INTERP_LANGUAGES: { value: string; label: string }[] = [
  { value: "es", label: "Español" },
  { value: "en", label: "English" },
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
];

/**
 * Modo Intérprete (guía): levanta la sala local, muestra código + QR, y
 * publica líneas de prueba (Fase 1). El audio real llega en la Fase 2.
 */
export const InterpreterSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [room, setRoom] = useState<InterpreterRoom | null>(null);
  const [listeners, setListeners] = useState(0);
  const [langs, setLangs] = useState<string[]>([]);
  const [testText, setTestText] = useState("");
  const [listening, setListening] = useState(false);
  const [sourceLang, setSourceLang] = useState(
    (i18n.language || "es").split("-")[0],
  );
  const pollRef = useRef<number | null>(null);

  const refreshStatus = useCallback(async () => {
    const s = await commands.interpreterStatus();
    setListeners(s.listeners);
    setLangs(s.active_languages);
    if (!s.running) setRoom(null);
  }, []);

  useEffect(() => {
    refreshStatus();
    pollRef.current = window.setInterval(refreshStatus, 2000);
    return () => {
      if (pollRef.current) window.clearInterval(pollRef.current);
    };
  }, [refreshStatus]);

  const start = async () => {
    const result = await commands.interpreterStart();
    if (result.status === "ok") setRoom(result.data);
  };

  const stop = async () => {
    await commands.interpreterStop();
    setListening(false);
    setRoom(null);
  };

  const publishTest = () => {
    if (!testText.trim()) return;
    commands.interpreterPublish(testText.trim(), sourceLang);
    setTestText("");
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="rounded-lg border border-mid-gray/30 p-5 space-y-3">
        <h3 className="font-semibold text-text">{t("interpreter.title")}</h3>
        <p className="text-sm text-mid-gray">{t("interpreter.subtitle")}</p>

        <div>
          <label className="text-xs text-mid-gray block mb-1">
            {t("interpreter.sourceLang")}
          </label>
          <select
            value={sourceLang}
            onChange={(e) => {
              setSourceLang(e.target.value);
              commands.interpreterSetSourceLang(e.target.value);
            }}
            className="w-full px-3 py-2 rounded-lg border border-mid-gray/30 bg-background text-text text-sm"
          >
            {INTERP_LANGUAGES.map((l) => (
              <option key={l.value} value={l.value}>
                {l.label}
              </option>
            ))}
          </select>
        </div>

        {!room ? (
          <Button variant="primary" size="md" onClick={start}>
            {t("interpreter.start")}
          </Button>
        ) : (
          <div className="space-y-4">
            <div className="flex flex-col items-center gap-3 py-2">
              <div
                className="bg-white p-3 rounded-xl"
                dangerouslySetInnerHTML={{ __html: room.qr_svg }}
                style={{ width: 200, height: 200 }}
              />
              <div className="text-center">
                <div className="text-xs text-mid-gray">
                  {t("interpreter.roomCode")}
                </div>
                <div className="text-4xl font-bold tracking-widest text-logo-primary">
                  {room.code}
                </div>
              </div>
              <div className="text-xs text-mid-gray break-all text-center">
                {room.url}
              </div>
            </div>

            <div className="flex items-center justify-between text-sm">
              <span className="text-text">
                {t("interpreter.listeners", { count: listeners })}
              </span>
              {langs.length > 0 && (
                <span className="text-mid-gray">
                  {t("interpreter.languages")}: {langs.join(", ")}
                </span>
              )}
            </div>

            <div className="rounded-lg bg-logo-primary/10 border border-logo-primary/30 p-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-text font-medium">
                  {listening
                    ? t("interpreter.listeningOn")
                    : t("interpreter.listeningOff")}
                </span>
                <Button
                  variant={listening ? "secondary" : "primary"}
                  size="sm"
                  onClick={() => {
                    const next = !listening;
                    setListening(next);
                    commands.interpreterSetSourceLang(sourceLang);
                    commands.interpreterSetListening(next);
                  }}
                >
                  {listening
                    ? t("interpreter.stopMic")
                    : t("interpreter.startMic")}
                </Button>
              </div>
              <p className="text-xs text-mid-gray mt-2">
                {t("interpreter.micHint")}
              </p>
            </div>

            {/* Prueba manual (por si no quieres usar el microfono) */}
            <div className="flex gap-2">
              <input
                value={testText}
                onChange={(e) => setTestText(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && publishTest()}
                placeholder={t("interpreter.testPlaceholder")}
                className="flex-1 px-3 py-2 rounded-lg border border-mid-gray/30 bg-background text-text text-sm"
              />
              <Button variant="secondary" size="sm" onClick={publishTest}>
                {t("interpreter.sendTest")}
              </Button>
            </div>

            <Button variant="secondary" size="md" onClick={stop}>
              {t("interpreter.stop")}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
};
