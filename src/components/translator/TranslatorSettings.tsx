import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { scrollBehavior } from "@/lib/motion";
import { listen } from "@tauri-apps/api/event";
import {
  ArrowLeftRight,
  Check,
  Copy,
  Globe,
  Lock,
  Mic,
  Volume2,
  Zap,
} from "lucide-react";
import { commands } from "@/bindings";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { EngineRequiredCard } from "../shared/EngineRequiredCard";

const LANGUAGES: { value: string; label: string; flag: string }[] = [
  { value: "es", label: "Español", flag: "🇪🇸" },
  { value: "en", label: "English (Inglés)", flag: "🇬🇧" },
  { value: "pt", label: "Português (Portugués)", flag: "🇵🇹" },
  { value: "fr", label: "Français (Francés)", flag: "🇫🇷" },
  { value: "de", label: "Deutsch (Alemán)", flag: "🇩🇪" },
  { value: "it", label: "Italiano", flag: "🇮🇹" },
  { value: "zh", label: "中文 (Chino)", flag: "🇨🇳" },
  { value: "ja", label: "日本語 (Japonés)", flag: "🇯🇵" },
  { value: "ko", label: "한국어 (Coreano)", flag: "🇰🇷" },
  { value: "ru", label: "Русский (Ruso)", flag: "🇷🇺" },
  { value: "lt", label: "Lietuvių (Lituano)", flag: "🇱🇹" },
  { value: "ar", label: "العربية (Árabe)", flag: "🇸🇦" },
];

type Result = { source: string; target_lang: string; translation: string };

export const TranslatorSettings: React.FC = () => {
  const { t } = useTranslation();
  const [langA, setLangA] = useState("es");
  const [langB, setLangB] = useState("en");
  const [listening, setListening] = useState(false);
  // Conversación completa de la sesión (QA de Flor: "si es un traductor, se
  // supone que es como una sesión"). Se conserva en memoria mientras la
  // pantalla vive; cada intercambio con sus botones.
  const [history, setHistory] = useState<Result[]>([]);
  const last = history.length > 0 ? history[history.length - 1] : null;
  const [voiceOn, setVoiceOn] = useState(true);
  const [copiedIdx, setCopiedIdx] = useState<number | null>(null);
  const convRef = useRef<HTMLDivElement>(null);

  const copyTranslation = (item: Result, idx: number) => {
    navigator.clipboard.writeText(item.translation).then(() => {
      setCopiedIdx(idx);
      window.setTimeout(() => setCopiedIdx(null), 1500);
    });
  };

  // La mejor voz instalada para el idioma destino: Premium > Enhanced > local.
  // (QA de Flor: sin esto, el inglés salía leído "como un español intentando
  // hablar inglés" — la voz por defecto del sistema no siempre es nativa.)
  const voicesRef = useRef<SpeechSynthesisVoice[]>([]);
  useEffect(() => {
    if (!("speechSynthesis" in window)) return;
    const load = () => {
      voicesRef.current = window.speechSynthesis.getVoices();
    };
    load();
    window.speechSynthesis.addEventListener("voiceschanged", load);
    return () =>
      window.speechSynthesis.removeEventListener("voiceschanged", load);
  }, []);
  const pickVoice = useCallback((lang: string) => {
    const base = lang.split("-")[0].toLowerCase();
    const candidates = voicesRef.current.filter((v: SpeechSynthesisVoice) =>
      v.lang.toLowerCase().replace("_", "-").startsWith(base),
    );
    const score = (v: SpeechSynthesisVoice) => {
      if (/premium/i.test(v.name)) return 30;
      if (/enhanced|mejorada|siri/i.test(v.name)) return 20;
      return v.localService ? 10 : 0;
    };
    return candidates.sort(
      (a: SpeechSynthesisVoice, b: SpeechSynthesisVoice) => score(b) - score(a),
    )[0];
  }, []);

  // Habla siempre (para el botón de reproducir): ignora el toggle de voz,
  // porque un clic explícito ES el permiso.
  const speakNow = useCallback(
    (text: string, lang: string) => {
      if (!("speechSynthesis" in window)) return;
      window.speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(text);
      u.lang = lang;
      const voice = pickVoice(lang);
      if (voice) u.voice = voice;
      u.rate = 0.98;
      window.speechSynthesis.speak(u);
    },
    [pickVoice],
  );

  // Reproducción automática de cada resultado, gobernada por el toggle.
  const speak = useCallback(
    (text: string, lang: string) => {
      if (voiceOn) speakNow(text, lang);
    },
    [voiceOn, speakNow],
  );

  useEffect(() => {
    const unlisten = listen<Result>("translator-result", (e) => {
      setHistory((prev) => [...prev.slice(-29), e.payload]);
      speak(e.payload.translation, e.payload.target_lang);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [speak]);

  // Autoscroll al último intercambio.
  useEffect(() => {
    convRef.current?.scrollTo({
      top: convRef.current.scrollHeight,
      behavior: scrollBehavior(),
    });
  }, [history.length]);

  const toggleListening = () => {
    const next = !listening;
    setListening(next);
    commands.translatorSetLangs(langA, langB);
    commands.translatorSetListening(next);
  };

  useEffect(() => {
    if (listening) commands.translatorSetLangs(langA, langB);
  }, [langA, langB, listening]);

  const swapLangs = () => {
    setLangA(langB);
    setLangB(langA);
  };

  const langLabel = (code: string) => {
    const l = LANGUAGES.find((x) => x.value === code);
    return l ? `${l.flag} ${l.label}` : code;
  };

  const capabilities = [
    { icon: Globe, label: t("translator.caps.languages") },
    { icon: Lock, label: t("translator.caps.local") },
    { icon: Zap, label: t("translator.caps.instant") },
  ];

  const LangSelect: React.FC<{
    value: string;
    onChange: (v: string) => void;
  }> = ({ value, onChange }) => (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full rounded-card border border-mid-gray/25 bg-background px-4 py-3 text-base font-medium text-text shadow-card focus:outline-none focus:ring-1 focus:ring-logo-primary"
    >
      {LANGUAGES.map((l) => (
        <option key={l.value} value={l.value}>
          {l.flag} {l.label}
        </option>
      ))}
    </select>
  );

  return (
    <div className="max-w-3xl w-full mx-auto space-y-8 py-2">
      {/* Héroe: vende la experiencia, no la configuración. */}
      <div>
        <h1
          className="text-3xl leading-tight text-text sm:text-[2rem]"
          style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
        >
          {t("translator.heroTitle")}
        </h1>
        <p className="mt-2 max-w-xl text-sm text-mid-gray">
          {t("translator.heroSubtitle")}
        </p>
      </div>

      <EngineRequiredCard />

      <div className="grid gap-8 lg:grid-cols-[1.1fr_1fr] lg:items-start">
        {/* Izquierda: configuración mínima + la acción protagonista. */}
        <div className="space-y-5">
          {/* Par de idiomas con botón de invertir grande. */}
          <div className="flex items-center gap-2">
            <LangSelect value={langA} onChange={setLangA} />
            <button
              type="button"
              onClick={swapLangs}
              title={t("translator.swap")}
              className="flex h-11 w-11 shrink-0 items-center justify-center rounded-card border border-mid-gray/25 bg-background text-mid-gray shadow-card transition-colors hover:border-logo-primary/50 hover:text-gold-text"
            >
              <ArrowLeftRight width={18} height={18} />
            </button>
            <LangSelect value={langB} onChange={setLangB} />
          </div>

          {/* Voz como switch claro. */}
          <div className="rounded-card border border-line bg-background px-4 py-1 shadow-card">
            <ToggleSwitch
              checked={voiceOn}
              onChange={(v) => {
                setVoiceOn(v);
                if (!v) window.speechSynthesis.cancel();
              }}
              label={t("translator.readAloud")}
              description={t("translator.readAloudHint")}
              descriptionMode="tooltip"
            />
          </div>

          {/* La única acción importante: enorme. */}
          <button
            type="button"
            onClick={toggleListening}
            className={`flex w-full items-center justify-center gap-3 rounded-card px-6 py-4 text-base font-semibold shadow-[0_1px_2px_rgba(27,20,38,0.15),inset_0_1px_0_rgba(255,255,255,0.25)] transition-all ${
              listening
                ? "border border-lacre/40 bg-lacre/10 text-lacre"
                : "gold-surface text-ink hover:brightness-105 active:brightness-95"
            }`}
          >
            <Mic width={20} height={20} />
            {listening ? t("translator.stop") : t("translator.start")}
          </button>

          {listening && (
            <p className="flex items-center justify-center gap-2 text-xs text-mid-gray">
              <span className="h-2 w-2 animate-pulse rounded-full bg-lacre" />
              {t("translator.hint")}
            </p>
          )}

          {/* Ventajas. */}
          <div className="grid grid-cols-3 gap-2 pt-1">
            {capabilities.map(({ icon: Icon, label }) => (
              <div
                key={label}
                className="flex flex-col items-center gap-1.5 rounded-card border border-line bg-background px-2 py-3 text-center shadow-card"
              >
                <Icon width={17} height={17} className="text-gold-text" />
                <span className="text-2xs leading-tight text-mid-gray">
                  {label}
                </span>
              </div>
            ))}
          </div>
        </div>

        {/* Derecha: vista previa / conversación en vivo (estilo Messages). */}
        <div className="rounded-card border border-line bg-vitela/30 p-5 shadow-card">
          <div className="mb-3 flex items-center justify-between">
            <p className="font-mono text-3xs font-semibold uppercase tracking-[0.14em] text-mid-gray">
              {last ? t("translator.conversation") : t("translator.preview")}
            </p>
            {voiceOn && (
              <Volume2 width={14} height={14} className="text-gold-text" />
            )}
          </div>

          {history.length > 0 ? (
            // La conversación completa, como sesión: cada intercambio queda a
            // la vista (QA de Flor) y el último llega con autoscroll.
            <div
              ref={convRef}
              className="max-h-[46vh] space-y-3 overflow-y-auto pr-1"
            >
              {history.map((item, idx) => (
                <div key={idx} className="space-y-2">
                  <div className="rounded-card rounded-tl-sm bg-background px-4 py-3 shadow-sm">
                    <p className="mb-1 text-3xs uppercase tracking-wide text-mid-gray">
                      {t("translator.youSaid")}
                    </p>
                    <p className="text-sm text-text/80">{item.source}</p>
                  </div>
                  <div className="ms-6 rounded-card rounded-tr-sm border border-logo-primary/30 bg-logo-primary/5 px-4 py-3 shadow-sm">
                    <div className="mb-1 flex items-center justify-between">
                      <p className="text-3xs uppercase tracking-wide text-gold-text">
                        {langLabel(item.target_lang)}
                      </p>
                      <div className="flex items-center gap-1">
                        <button
                          type="button"
                          title={t("translator.replayTitle")}
                          onClick={() =>
                            speakNow(item.translation, item.target_lang)
                          }
                          className="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-2xs text-mid-gray transition-colors hover:text-text focus:outline-none focus:ring-1 focus:ring-logo-primary"
                        >
                          <Volume2 width={12} height={12} />
                          {t("translator.replay")}
                        </button>
                        <button
                          type="button"
                          title={t("a11y.copyToClipboard")}
                          onClick={() => copyTranslation(item, idx)}
                          className="flex items-center gap-1 rounded-md px-1.5 py-0.5 text-2xs text-mid-gray transition-colors hover:text-text focus:outline-none focus:ring-1 focus:ring-logo-primary"
                        >
                          {copiedIdx === idx ? (
                            <Check
                              width={12}
                              height={12}
                              className="text-success"
                            />
                          ) : (
                            <Copy width={12} height={12} />
                          )}
                          {copiedIdx === idx
                            ? t("mcp.copied")
                            : t("translator.copy")}
                        </button>
                      </div>
                    </div>
                    <p
                      className={`leading-snug text-text ${idx === history.length - 1 ? "text-xl" : "text-base"}`}
                      style={{
                        fontFamily: "var(--font-serif)",
                        fontWeight: 600,
                      }}
                    >
                      {item.translation}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          ) : (
            // Vista previa estática: el usuario imagina el resultado.
            <div className="space-y-3 opacity-90">
              <div className="rounded-card rounded-tl-sm bg-background/70 px-4 py-3">
                <p className="mb-1 text-3xs uppercase tracking-wide text-mid-gray">
                  {t("translator.youSaid")}
                </p>
                <p className="text-sm text-text/70">
                  {t("translator.previewSource")}
                </p>
              </div>
              <div className="ms-6 rounded-card rounded-tr-sm border border-logo-primary/20 bg-logo-primary/5 px-4 py-3">
                <p className="mb-1 text-3xs uppercase tracking-wide text-gold-text">
                  {langLabel(langB)}
                </p>
                <p
                  className="text-xl leading-snug text-text/80"
                  style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
                >
                  {t("translator.previewTarget")}
                </p>
              </div>
              <p className="pt-1 text-center text-xs text-mid-gray">
                {t("translator.previewHint")}
              </p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
