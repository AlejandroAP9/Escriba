import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Copy, Link2, Mic, Radio, Square, Users } from "lucide-react";
import { commands, type InterpreterRoom } from "@/bindings";
import { Button } from "../ui/Button";
import { EngineRequiredCard } from "../shared/EngineRequiredCard";

const INTERP_LANGUAGES: { value: string; label: string; flag: string }[] = [
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

const langInfo = (code: string) =>
  INTERP_LANGUAGES.find((l) => l.value === code);

/**
 * Modo Intérprete (guía): levanta la sala local y se convierte en un panel de
 * interpretación simultánea (QR, estado de sala, participantes por idioma, y el
 * control de micrófono como cabina). El audio real llega en la Fase 2.
 */
export const InterpreterSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const [room, setRoom] = useState<InterpreterRoom | null>(null);
  const [listeners, setListeners] = useState(0);
  const [langs, setLangs] = useState<string[]>([]);
  const [testText, setTestText] = useState("");
  const [lastSent, setLastSent] = useState<string | null>(null);
  const [listening, setListening] = useState(false);
  const [copied, setCopied] = useState<"code" | "link" | null>(null);
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
    if (result.status === "ok") {
      setRoom(result.data);
    } else {
      // Puerto ocupado / fallo de red local: sin esto el botón parece muerto.
      toast.error(t("interpreter.startError"), { description: result.error });
    }
  };

  const stop = async () => {
    await commands.interpreterStop();
    setListening(false);
    setRoom(null);
    setLastSent(null);
  };

  const publishTest = () => {
    if (!testText.trim()) return;
    commands.interpreterPublish(testText.trim(), sourceLang);
    setLastSent(testText.trim());
    setTestText("");
  };

  const copy = (text: string, which: "code" | "link") => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(which);
      window.setTimeout(() => setCopied(null), 1500);
    });
  };

  // ---- Estado inicial: elegir idioma y crear la sala ----
  if (!room) {
    return (
      <div className="max-w-3xl w-full mx-auto space-y-6 py-2">
        <div>
          <h1
            className="text-3xl leading-tight text-text sm:text-[2rem]"
            style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
          >
            {t("interpreter.title")}
          </h1>
          <p className="mt-2 max-w-xl text-sm text-mid-gray">
            {t("interpreter.subtitle")}
          </p>
        </div>

        <EngineRequiredCard />

        {/* Cómo funciona */}
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
          {[1, 2, 3, 4].map((n) => (
            <div
              key={n}
              className="rounded-card border border-line bg-background p-3 shadow-card"
            >
              <div className="font-mono text-xs font-semibold text-gold-text">
                {n}
              </div>
              <p className="mt-1 text-xs leading-snug text-mid-gray">
                {t(`interpreter.steps.s${n}`)}
              </p>
            </div>
          ))}
        </div>

        <div className="max-w-sm">
          <label className="mb-1 block text-xs text-mid-gray">
            {t("interpreter.sourceLang")}
          </label>
          <select
            value={sourceLang}
            onChange={(e) => {
              setSourceLang(e.target.value);
              commands.interpreterSetSourceLang(e.target.value);
            }}
            className="w-full rounded-card border border-mid-gray/25 bg-background px-4 py-3 text-base font-medium text-text shadow-card focus:outline-none focus:ring-1 focus:ring-logo-primary"
          >
            {INTERP_LANGUAGES.map((l) => (
              <option key={l.value} value={l.value}>
                {l.flag} {l.label}
              </option>
            ))}
          </select>
        </div>

        <Button
          variant="primary"
          size="lg"
          className="flex w-full items-center justify-center gap-2.5 py-4 text-base"
          onClick={start}
        >
          <Radio width={20} height={20} />
          {t("interpreter.start")}
        </Button>
      </div>
    );
  }

  // ---- Sala activa: panel de interpretación en vivo ----
  return (
    <div className="max-w-3xl w-full mx-auto space-y-4 py-2">
      {/* Estado de la sala */}
      <div className="flex items-center gap-3 rounded-card border border-line bg-background px-4 py-3 shadow-card">
        <span className="relative flex h-2.5 w-2.5">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-success/60" />
          <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-success" />
        </span>
        <span className="text-sm font-semibold text-text">
          {t("interpreter.roomActive")}
        </span>
        <span className="ms-auto flex items-center gap-1.5 text-sm text-mid-gray">
          <Users width={15} height={15} />
          {t("interpreter.listeners", { count: listeners })}
        </span>
      </div>

      {/* Aviso honesto (auditoría #8): la sala viaja por la WiFi local sin
          cifrar; en una red pública conviene un hotspot privado. */}
      <div className="rounded-card border border-warning/30 bg-warning/10 px-4 py-2.5 text-xs text-text/80">
        {t("interpreter.networkNote")}
      </div>

      <div className="grid gap-4 lg:grid-cols-[1fr_1fr] lg:items-start">
        {/* QR protagonista */}
        <div className="flex flex-col items-center rounded-card border border-line bg-vitela/30 p-5 text-center shadow-card">
          <div
            className="rounded-card bg-white p-3"
            dangerouslySetInnerHTML={{ __html: room.qr_svg }}
            style={{ width: 208, height: 208 }}
          />
          <p className="mt-3 text-xs text-mid-gray">
            {t("interpreter.scanHint")}
          </p>
          <div className="mt-4 w-full rounded-card border border-line bg-background px-4 py-3">
            <div className="font-mono text-3xs uppercase tracking-[0.14em] text-mid-gray">
              {t("interpreter.roomCode")}
            </div>
            <div
              className="text-4xl tracking-[0.2em] text-gold-text"
              style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
            >
              {room.code}
            </div>
          </div>
          <div className="mt-2 flex w-full gap-2">
            <button
              type="button"
              onClick={() => copy(room.code, "code")}
              className="flex flex-1 items-center justify-center gap-1.5 rounded-lg border border-mid-gray/20 py-2 text-xs font-medium text-text transition-colors hover:border-logo-primary/50 hover:text-gold-text"
            >
              <Copy width={13} height={13} />
              {copied === "code"
                ? t("interpreter.copied")
                : t("interpreter.copyCode")}
            </button>
            <button
              type="button"
              onClick={() => copy(room.url, "link")}
              className="flex flex-1 items-center justify-center gap-1.5 rounded-lg border border-mid-gray/20 py-2 text-xs font-medium text-text transition-colors hover:border-logo-primary/50 hover:text-gold-text"
            >
              <Link2 width={13} height={13} />
              {copied === "link"
                ? t("interpreter.copied")
                : t("interpreter.copyLink")}
            </button>
          </div>
        </div>

        {/* Cabina: micrófono + participantes */}
        <div className="space-y-4">
          {/* Micrófono */}
          <div
            className={`rounded-card border p-5 text-center shadow-card ${
              listening
                ? "border-lacre/40 bg-lacre/6"
                : "border-line bg-background"
            }`}
          >
            <div
              className={`mx-auto flex h-14 w-14 items-center justify-center rounded-card ${
                listening
                  ? "bg-lacre/15 text-lacre"
                  : "bg-logo-primary/10 text-gold-text"
              }`}
            >
              <Mic width={26} height={26} />
            </div>
            <p className="mt-3 text-base font-semibold text-text">
              {listening
                ? t("interpreter.listeningNow")
                : t("interpreter.waitingVoice")}
            </p>
            <p className="mt-1 text-xs text-mid-gray">
              {t("interpreter.micHint")}
            </p>
            <Button
              variant={listening ? "secondary" : "primary"}
              size="lg"
              className={`mt-4 flex w-full items-center justify-center gap-2 py-3 ${
                listening ? "" : "gold-surface"
              }`}
              onClick={() => {
                const next = !listening;
                setListening(next);
                commands.interpreterSetSourceLang(sourceLang);
                commands.interpreterSetListening(next);
              }}
            >
              {listening ? t("interpreter.stopMic") : t("interpreter.startMic")}
            </Button>
          </div>

          {/* Participantes (por idioma elegido) */}
          <div className="rounded-card border border-line bg-background p-4 shadow-card">
            <p className="font-mono text-3xs font-semibold uppercase tracking-[0.14em] text-mid-gray">
              {t("interpreter.participants")}
            </p>
            {langs.length === 0 ? (
              <p className="mt-3 text-sm text-mid-gray">
                {t("interpreter.waitingParticipants")}
              </p>
            ) : (
              <ul className="mt-3 space-y-2">
                {langs.map((code) => {
                  const info = langInfo(code);
                  return (
                    <li
                      key={code}
                      className="flex items-center gap-2.5 text-sm text-text"
                    >
                      <span className="text-base">{info?.flag ?? "🌐"}</span>
                      <span>{info?.label ?? code}</span>
                      <span className="ms-auto flex items-center gap-1.5 text-xs text-success">
                        <span className="h-1.5 w-1.5 rounded-full bg-success" />
                        {t("interpreter.connected")}
                      </span>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </div>
      </div>

      {/* Última frase enviada */}
      {lastSent && (
        <div className="rounded-card border border-logo-primary/25 bg-logo-primary/5 px-4 py-3">
          <p className="font-mono text-3xs uppercase tracking-[0.14em] text-gold-text">
            {t("interpreter.lastPhrase")}
          </p>
          <p className="mt-1 text-sm text-text">“{lastSent}”</p>
        </div>
      )}

      {/* Modo prueba (plegado por defecto) */}
      <details className="rounded-card border border-line bg-background px-4 py-3">
        <summary className="cursor-pointer text-xs font-medium text-mid-gray">
          {t("interpreter.testMode")}
        </summary>
        <div className="mt-3 flex gap-2">
          <input
            value={testText}
            onChange={(e) => setTestText(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && publishTest()}
            placeholder={t("interpreter.testPlaceholder")}
            className="flex-1 rounded-lg border border-mid-gray/30 bg-background px-3 py-2 text-sm text-text"
          />
          <Button variant="secondary" size="sm" onClick={publishTest}>
            {t("interpreter.sendTest")}
          </Button>
        </div>
      </details>

      {/* Detener debe ser inconfundible: una sala olvidada se traga los
          dictados (feedback de Flor). Nada de enlaces grises tímidos. */}
      <Button
        variant="secondary"
        size="lg"
        className="flex w-full items-center justify-center gap-2 border-lacre/40 text-lacre hover:bg-lacre/10"
        onClick={stop}
      >
        <Square width={14} height={14} />
        {t("interpreter.stop")}
      </Button>
    </div>
  );
};
