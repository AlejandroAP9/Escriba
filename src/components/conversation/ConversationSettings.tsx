import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import {
  ArrowRight,
  Check,
  Copy,
  Ear,
  FileText,
  Lock,
  MessageCircle,
  Mic,
  Pause,
  Play,
  Sparkles,
  Star,
  Trash2,
} from "lucide-react";
import { commands, type Turn } from "@/bindings";
import { Button } from "../ui/Button";
import { Card } from "../ui/Card";
import { EmptyState } from "../ui/EmptyState";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

/**
 * Conversación: una sesión de voz que termina en un documento. Dos modos sobre
 * el mismo núcleo: "converse" (la IA responde cada turno y se lee en voz alta)
 * y "listen" (la IA calla: entrevistas, reuniones, actas). El dictado entra por
 * el atajo global de siempre; el backend lo intercepta como turno de la sesión.
 */

type Mode = "converse" | "listen";
type Phase = "idle" | "active" | "doc";

// ToggleSwitch exige description; aquí el label se explica solo.
const NO_DESCRIPTION = "";

// mm:ss para las marcas de tiempo de los turnos.
function stamp(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

// Atajo de dictado en símbolos (mismo formato que Inicio).
const KEY_SYMBOLS: Record<string, string> = {
  cmd: "⌘",
  command: "⌘",
  super: "⌘",
  alt: "⌥",
  option: "⌥",
  ctrl: "⌃",
  control: "⌃",
  shift: "⇧",
  space: "Space",
  enter: "↵",
  return: "↵",
};
function formatKeys(binding: string | undefined): string[] {
  if (!binding) return [];
  return binding.split("+").map((raw) => {
    const k = raw.trim().toLowerCase();
    if (KEY_SYMBOLS[k]) return KEY_SYMBOLS[k];
    return raw.trim().length === 1
      ? raw.trim().toUpperCase()
      : raw.trim().charAt(0).toUpperCase() + raw.trim().slice(1);
  });
}

export const ConversationSettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  const { getSetting } = useSettings();
  const [phase, setPhase] = useState<Phase>("idle");
  const [mode, setMode] = useState<Mode>("converse");
  const [listening, setListening] = useState(false);
  const [turns, setTurns] = useState<Turn[]>([]);
  const [thinking, setThinking] = useState(false);
  const [voiceOn, setVoiceOn] = useState(true);
  const [doc, setDoc] = useState("");
  const [creating, setCreating] = useState(false);
  const [copied, setCopied] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const voiceOnRef = useRef(voiceOn);
  voiceOnRef.current = voiceOn;
  const modeRef = useRef(mode);
  modeRef.current = mode;

  // Voces del sistema (getVoices llega vacío hasta el evento voiceschanged).
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

  // La mejor voz instalada para el idioma: Premium > Enhanced > local > resto.
  // (macOS marca la calidad en el nombre; las Premium/Enhanced son neurales.)
  const pickVoice = useCallback((lang: string) => {
    const base = lang.split("-")[0].toLowerCase();
    // Región real del sistema (es-CL) para desempatar entre voces de calidad
    // igual: Francisca (es_CL) le gana a Mónica (es_ES) en un Mac chileno.
    const region = (navigator.language || lang).toLowerCase().replace("_", "-");
    const candidates = voicesRef.current.filter((v) =>
      v.lang.toLowerCase().replace("_", "-").startsWith(base),
    );
    const score = (v: SpeechSynthesisVoice) => {
      const quality = /premium/i.test(v.name)
        ? 30
        : /enhanced|mejorada|siri/i.test(v.name)
          ? 20
          : v.localService
            ? 10
            : 0;
      const regionMatch =
        v.lang.toLowerCase().replace("_", "-") === region ? 5 : 0;
      return quality + regionMatch;
    };
    return candidates.sort((a, b) => score(b) - score(a))[0];
  }, []);

  const speak = useCallback(
    async (text: string) => {
      if (!voiceOnRef.current) return;
      // Primero la VOZ DEL SISTEMA vía backend (`say`): usa las voces neurales
      // que el usuario eligió en Ajustes de macOS y que el webview no ve.
      try {
        if (await commands.conversationSpeak(text)) return;
      } catch {
        /* cae al respaldo */
      }
      // Respaldo (Windows/Linux): speechSynthesis con la mejor voz visible.
      if (!("speechSynthesis" in window)) return;
      window.speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(text);
      u.lang = i18n.language;
      const voice = pickVoice(i18n.language);
      if (voice) u.voice = voice;
      u.rate = 1.0;
      window.speechSynthesis.speak(u);
    },
    [i18n.language, pickVoice],
  );

  // Reconectar con una sesión viva (cambio de pantalla y vuelta).
  useEffect(() => {
    commands.conversationStatus().then((s) => {
      if (s.turns.length > 0 || s.listening) {
        setMode(s.mode === "listen" ? "listen" : "converse");
        setTurns(s.turns);
        setListening(s.listening);
        setPhase("active");
      }
    });
  }, []);

  // Turnos en vivo desde el backend.
  useEffect(() => {
    const unTurn = listen<Turn>("conversation-turn", (e) => {
      setTurns((prev) => [...prev, e.payload]);
      if (e.payload.role === "user") {
        if (modeRef.current === "converse") setThinking(true);
      } else {
        setThinking(false);
        speak(e.payload.text);
      }
    });
    const unFail = listen("conversation-reply-failed", () => {
      setThinking(false);
      toast.error(t("conversation.replyFailed"));
    });
    return () => {
      unTurn.then((fn) => fn());
      unFail.then((fn) => fn());
    };
  }, [speak, t]);

  // Autoscroll al último turno.
  useEffect(() => {
    scrollRef.current?.scrollTo({
      top: scrollRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, [turns.length, thinking]);

  // Al salir de la pantalla, la sesión queda en pausa (no se pierde).
  useEffect(() => {
    return () => {
      commands.conversationStop();
      window.speechSynthesis?.cancel();
      commands.conversationSpeakStop();
    };
  }, []);

  const start = async () => {
    const s = await commands.conversationStart(mode);
    setTurns(s.turns);
    setListening(true);
    setPhase("active");
  };

  const togglePause = async () => {
    const s = listening
      ? await commands.conversationStop()
      : await commands.conversationStart(mode);
    setListening(s.listening);
  };

  const discard = async () => {
    await commands.conversationReset();
    window.speechSynthesis?.cancel();
    commands.conversationSpeakStop();
    setTurns([]);
    setThinking(false);
    setDoc("");
    setPhase("idle");
    setListening(false);
  };

  const finish = async () => {
    if (turns.length === 0) {
      toast.error(t("conversation.emptyFinish"));
      return;
    }
    setCreating(true);
    setListening(false);
    try {
      const r = await commands.conversationFinish();
      if (r.status === "ok") {
        setDoc(r.data);
        setPhase("doc");
      } else {
        toast.error(t("conversation.docError"));
        setPhase("active");
      }
    } finally {
      setCreating(false);
    }
  };

  const copyDoc = async () => {
    try {
      await navigator.clipboard.writeText(doc);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* portapapeles no disponible */
    }
  };

  const MODES: { id: Mode; icon: typeof Mic }[] = [
    { id: "converse", icon: MessageCircle },
    { id: "listen", icon: Ear },
  ];

  // Atajo de dictado real, para enseñarlo desde el primer día.
  const bindings = getSetting("bindings") as
    | Record<string, { current_binding?: string }>
    | undefined;
  const shortcutKeys = formatKeys(bindings?.transcribe?.current_binding);

  const FLOW: { key: string; icon: typeof Mic | null }[] = [
    { key: "speak", icon: Mic },
    { key: "listen", icon: Ear },
    { key: "organize", icon: Sparkles },
    { key: "doc", icon: FileText },
  ];
  const EXAMPLES = ["meeting", "brainstorm", "interview", "journal"];

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 py-2">
      {/* Héroe */}
      <div>
        <h1
          className="text-3xl leading-tight text-text sm:text-[2rem]"
          style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
        >
          {t("conversation.heroTitle")}
        </h1>
        <p className="mt-2 max-w-2xl text-sm leading-relaxed text-mid-gray">
          {t("conversation.heroSubtitle")}
        </p>
      </div>

      {phase === "idle" && (
        <div className="space-y-6 pt-3">
          {/* Modos: un selector compacto, pensado para admitir más modos. */}
          <div className="grid gap-3 sm:grid-cols-2">
            {MODES.map(({ id, icon: Icon }) => {
              const active = mode === id;
              return (
                <button
                  key={id}
                  type="button"
                  onClick={() => setMode(id)}
                  className={`rounded-card border px-4 py-3.5 text-left transition-all duration-150 ${
                    active
                      ? "border-logo-primary bg-logo-primary/10 shadow-card"
                      : "border-line bg-background hover:-translate-y-0.5 hover:border-mid-gray/30"
                  }`}
                >
                  <div className="flex items-center gap-2.5">
                    <span
                      className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-lg transition-colors ${
                        active
                          ? "bg-logo-primary/20 text-logo-primary"
                          : "bg-mid-gray/10 text-mid-gray"
                      }`}
                    >
                      <Icon width={20} height={20} />
                    </span>
                    <div className="min-w-0">
                      <span
                        className="block text-base leading-tight text-text"
                        style={{
                          fontFamily: "var(--font-serif)",
                          fontWeight: 600,
                        }}
                      >
                        {t(`conversation.mode.${id}.label`)}
                      </span>
                      <span className="block text-[11px] leading-snug text-mid-gray">
                        {t(`conversation.mode.${id}.desc`)}
                      </span>
                    </div>
                    {active && (
                      <Check
                        width={16}
                        height={16}
                        className="ml-auto shrink-0 text-logo-primary"
                      />
                    )}
                  </div>
                </button>
              );
            })}
          </div>

          {/* CTA + el atajo real: se aprende desde el primer día. */}
          <div>
            <Button
              variant="primary"
              size="lg"
              className="flex w-full items-center justify-center gap-2.5 py-4 text-base hover:-translate-y-0.5 hover:shadow-lift"
              onClick={start}
            >
              <Mic width={20} height={20} />
              {t("conversation.start")}
            </Button>
            {shortcutKeys.length > 0 && (
              <p className="mt-2.5 flex items-center justify-center gap-1.5 text-xs text-mid-gray">
                {t("conversation.afterStart")}
                {shortcutKeys.map((k) => (
                  <kbd
                    key={k}
                    className="rounded-md border border-mid-gray/25 bg-background px-1.5 py-0.5 font-mono text-[11px] text-text"
                  >
                    {k}
                  </kbd>
                ))}
              </p>
            )}
          </div>

          {/* Qué ocurre después: círculos conectados, se lee de un vistazo. */}
          <div className="flex items-start justify-center">
            {FLOW.map(({ key, icon: Icon }, i) => (
              <React.Fragment key={key}>
                {i > 0 && (
                  <div className="mt-4 h-px w-8 shrink-0 bg-logo-primary/25 sm:w-14" />
                )}
                <div className="flex w-20 flex-col items-center gap-1.5 sm:w-24">
                  <span
                    className={`flex h-8 w-8 items-center justify-center rounded-full border ${
                      key === "organize"
                        ? "border-logo-primary/40 bg-logo-primary/10 text-logo-primary"
                        : "border-line bg-background text-mid-gray"
                    }`}
                  >
                    {Icon && <Icon width={14} height={14} />}
                  </span>
                  <span className="text-center text-[11px] leading-tight text-mid-gray">
                    {t(`conversation.flow.${key}`)}
                  </span>
                </div>
              </React.Fragment>
            ))}
          </div>

          {/* Casos de uso: entrada → resultado, con el principal destacado. */}
          <div>
            <p className="mb-2 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-mid-gray">
              {t("conversation.examplesTitle")}
            </p>
            <div className="grid gap-2 sm:grid-cols-2">
              {EXAMPLES.map((id) => {
                const featured = id === "meeting";
                return (
                  <div
                    key={id}
                    className={`flex items-center gap-2.5 rounded-card border px-4 py-3 shadow-card ${
                      featured
                        ? "border-logo-primary/35 bg-logo-primary/5"
                        : "border-line bg-background"
                    }`}
                  >
                    {featured ? (
                      <Star
                        width={14}
                        height={14}
                        className="shrink-0 fill-logo-primary/30 text-logo-primary"
                      />
                    ) : (
                      <Mic
                        width={14}
                        height={14}
                        className="shrink-0 text-mid-gray"
                      />
                    )}
                    <span className="text-sm text-text">
                      {t(`conversation.example.${id}.label`)}
                    </span>
                    <ArrowRight
                      width={13}
                      height={13}
                      className="shrink-0 text-mid-gray/50"
                    />
                    <span className="ml-auto text-right text-xs text-mid-gray">
                      {t(`conversation.example.${id}.result`)}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Vista previa: la forma del resultado (las secciones son las reales
              del acta; las barras son abstractas, no texto inventado). */}
          <div>
            <p className="mb-2 text-center font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-mid-gray">
              {t("conversation.preview.title")}
            </p>
            <div className="mx-auto max-w-xs rounded-card border border-line bg-background p-5 shadow-lift">
              <p
                className="text-base text-text"
                style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
              >
                {t("conversation.preview.doc")}
              </p>
              <div className="mt-3 space-y-3">
                {(["s1", "s2", "s3"] as const).map((s, i) => (
                  <div key={s}>
                    <p className="text-[10px] font-semibold uppercase tracking-wide text-logo-primary">
                      {t(`conversation.preview.${s}`)}
                    </p>
                    <div className="mt-1.5 space-y-1.5">
                      <div className="h-1.5 w-full rounded-full bg-mid-gray/15" />
                      <div
                        className={`h-1.5 rounded-full bg-mid-gray/15 ${
                          i === 0 ? "w-4/5" : i === 1 ? "w-2/3" : "w-3/4"
                        }`}
                      />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* Privacidad: el diferenciador, como bloque. */}
          <div className="rounded-card border border-logo-primary/20 bg-logo-primary/5 p-4">
            <p className="mb-2.5 flex items-center gap-2 text-sm font-semibold text-text">
              <Lock width={14} height={14} className="text-logo-primary" />
              {t("conversation.privacyTitle")}
            </p>
            <ul className="grid gap-1.5 sm:grid-cols-2">
              {["p1", "p2", "p3", "p4"].map((k) => (
                <li
                  key={k}
                  className="flex items-start gap-2 text-xs text-text/80"
                >
                  <Check
                    width={13}
                    height={13}
                    className="mt-0.5 shrink-0 text-logo-primary"
                  />
                  {t(`conversation.privacy.${k}`)}
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}

      {phase === "active" && (
        <div className="space-y-4">
          {/* Barra de estado de la sesión. */}
          <Card className="flex flex-wrap items-center justify-between gap-3 px-4 py-3">
            <div className="flex items-center gap-2.5">
              <span className="relative flex h-2.5 w-2.5">
                {listening && (
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-500/60" />
                )}
                <span
                  className={`relative inline-flex h-2.5 w-2.5 rounded-full ${
                    listening ? "bg-green-600" : "bg-mid-gray/50"
                  }`}
                />
              </span>
              <span className="text-sm font-semibold text-text">
                {listening
                  ? t("conversation.active")
                  : t("conversation.paused")}
              </span>
              <span className="rounded-full border border-line px-2 py-0.5 text-[11px] text-mid-gray">
                {t(`conversation.mode.${mode}.label`)}
              </span>
            </div>
            <div className="flex items-center gap-2">
              <Button variant="secondary" size="sm" onClick={togglePause}>
                {listening ? (
                  <Pause width={13} height={13} className="mr-1.5" />
                ) : (
                  <Play width={13} height={13} className="mr-1.5" />
                )}
                {listening ? t("conversation.pause") : t("conversation.resume")}
              </Button>
              <Button variant="ghost" size="sm" onClick={discard}>
                <Trash2 width={13} height={13} className="mr-1.5" />
                {t("conversation.discard")}
              </Button>
            </div>
          </Card>

          {/* Transcript en vivo. */}
          <Card className="p-4">
            <div
              ref={scrollRef}
              className="max-h-[46vh] min-h-[220px] space-y-3 overflow-y-auto pr-1"
            >
              {turns.length === 0 && !thinking ? (
                <EmptyState
                  compact
                  icon={Mic}
                  title={t("conversation.empty")}
                  description={t("conversation.hint")}
                />
              ) : (
                <>
                  {turns.map((turn, i) =>
                    turn.role === "assistant" ? (
                      <div
                        key={i}
                        className="me-8 rounded-2xl rounded-tl-sm border border-logo-primary/25 bg-logo-primary/5 px-4 py-3"
                      >
                        <p className="mb-1 text-[10px] uppercase tracking-wide text-logo-primary">
                          {t("conversation.assistant")}
                        </p>
                        <p
                          className="text-base leading-snug text-text"
                          style={{ fontFamily: "var(--font-serif)" }}
                        >
                          {turn.text}
                        </p>
                      </div>
                    ) : (
                      <div
                        key={i}
                        className="ms-8 rounded-2xl rounded-tr-sm bg-mid-gray/5 px-4 py-3"
                      >
                        <p className="mb-1 flex items-center justify-between text-[10px] uppercase tracking-wide text-mid-gray">
                          <span>{t("conversation.you")}</span>
                          <span className="font-mono normal-case">
                            {stamp(turn.at_secs)}
                          </span>
                        </p>
                        <p className="text-sm leading-relaxed text-text/85">
                          {turn.text}
                        </p>
                      </div>
                    ),
                  )}
                  {thinking && (
                    <p className="flex items-center gap-2 px-1 text-xs text-mid-gray">
                      <Sparkles
                        width={13}
                        height={13}
                        className="animate-pulse text-logo-primary"
                      />
                      {t("conversation.thinking")}
                    </p>
                  )}
                </>
              )}
            </div>
            {turns.length > 0 && (
              <p className="mt-3 border-t border-line pt-2 text-center text-[11px] text-mid-gray">
                {t("conversation.hint")}
              </p>
            )}
          </Card>

          {mode === "converse" && (
            <div className="rounded-card border border-line bg-background px-4 py-1 shadow-card">
              <ToggleSwitch
                checked={voiceOn}
                onChange={(v) => {
                  setVoiceOn(v);
                  if (!v) {
                    window.speechSynthesis?.cancel();
                    commands.conversationSpeakStop();
                  }
                  commands.conversationSpeakStop();
                }}
                label={t("conversation.speakAloud")}
                description={NO_DESCRIPTION}
              />
            </div>
          )}

          <Button
            variant="primary"
            size="lg"
            className="flex w-full items-center justify-center gap-2.5 py-3.5"
            onClick={finish}
            disabled={creating || turns.length === 0}
          >
            <FileText width={18} height={18} />
            {creating ? t("conversation.creating") : t("conversation.finish")}
          </Button>
        </div>
      )}

      {phase === "doc" && (
        <div className="space-y-4">
          <Card className="p-5">
            <div className="mb-3 flex items-center justify-between border-b border-line pb-3">
              <p className="flex items-center gap-2 text-sm font-semibold text-text">
                <FileText
                  width={15}
                  height={15}
                  className="text-logo-primary"
                />
                {t("conversation.docTitle")}
              </p>
              <button
                type="button"
                onClick={copyDoc}
                className="flex items-center gap-1.5 rounded-lg border border-mid-gray/20 px-3 py-1.5 text-xs font-medium text-text transition-colors hover:border-mid-gray/40"
              >
                {copied ? (
                  <>
                    <Check width={13} height={13} className="text-green-600" />
                    {t("conversation.copied")}
                  </>
                ) : (
                  <>
                    <Copy width={13} height={13} />
                    {t("conversation.copy")}
                  </>
                )}
              </button>
            </div>
            <p className="whitespace-pre-line text-sm leading-relaxed text-text/90">
              {doc}
            </p>
          </Card>

          <div className="flex justify-center">
            <Button variant="secondary" size="md" onClick={discard}>
              {t("conversation.newSession")}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};
