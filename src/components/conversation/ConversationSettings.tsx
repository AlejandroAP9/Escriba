import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { scrollBehavior } from "@/lib/motion";
import { listen } from "@tauri-apps/api/event";
import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { requestObsidianExport } from "@/stores/obsidianStore";
import {
  ArrowRight,
  Check,
  Copy,
  Ear,
  FileDown,
  FileText,
  Languages,
  Lock,
  MessageCircle,
  Mic,
  MonitorSpeaker,
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
import { EngineRequiredCard } from "../shared/EngineRequiredCard";
import { Plumin } from "../shared/Plumin";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

/**
 * Conversación: una sesión de voz que termina en un documento. Dos modos sobre
 * el mismo núcleo: "converse" (la IA responde cada turno y se lee en voz alta)
 * y "listen" (la IA calla: entrevistas, reuniones, actas). El dictado entra por
 * el atajo global de siempre; el backend lo intercepta como turno de la sesión.
 */

type Mode = "converse" | "listen";
/// Variantes del modo Escuchar: mismo núcleo silencioso, documento distinto.
type Variant = "listen" | "interview" | "class" | "brainstorm";
const VARIANTS: Variant[] = ["listen", "interview", "class", "brainstorm"];
type Phase = "idle" | "active" | "doc";

// Idiomas de reunión para el Intérprete de reuniones (idea de John Walter):
// misma lista y banderas del Traductor e Intérprete, coherencia entre pantallas.
// Etiquetas cortas a propósito: el select sin chevron toma el ancho de su
// opción más larga, y la bandera ya identifica el idioma de un vistazo.
const MEETING_LANGUAGES: { value: string; label: string; flag: string }[] = [
  { value: "en", label: "English", flag: "🇬🇧" },
  { value: "es", label: "Español", flag: "🇪🇸" },
  { value: "pt", label: "Português", flag: "🇵🇹" },
  { value: "fr", label: "Français", flag: "🇫🇷" },
  { value: "de", label: "Deutsch", flag: "🇩🇪" },
  { value: "it", label: "Italiano", flag: "🇮🇹" },
  { value: "zh", label: "中文", flag: "🇨🇳" },
  { value: "ja", label: "日本語", flag: "🇯🇵" },
  { value: "ko", label: "한국어", flag: "🇰🇷" },
  { value: "ru", label: "Русский", flag: "🇷🇺" },
  { value: "lt", label: "Lietuvių", flag: "🇱🇹" },
  { value: "ar", label: "العربية", flag: "🇸🇦" },
];

// Idiomas con voz neural incluida (Piper vía sherpa-onnx); el resto usa la
// voz del sistema. Coincide con el registro del backend (managers/tts.rs).
const NEURAL_VOICE_LANGS = ["en", "es"];

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
  const [variant, setVariant] = useState<Variant>("listen");
  const [listening, setListening] = useState(false);
  const [turns, setTurns] = useState<Turn[]>([]);
  const [thinking, setThinking] = useState(false);
  const [voiceOn, setVoiceOn] = useState(true);
  const [doc, setDoc] = useState("");
  // Ánimo de la sesión (idea de Pedro Sánchez): define la carita de Plumín.
  const [docMood, setDocMood] = useState("neutral");
  const [creating, setCreating] = useState(false);
  const [copied, setCopied] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const voiceOnRef = useRef(voiceOn);
  voiceOnRef.current = voiceOn;
  const modeRef = useRef(mode);
  modeRef.current = mode;
  // Motor de voz elegido por el usuario (persistente). Default: la voz del
  // sistema (ganó el A/B contra la incluida); la incluida queda para equipos
  // sin voces Premium/Mejorada.
  const [voiceEngine, setVoiceEngine] = useState<"included" | "system">(() =>
    localStorage.getItem("escriba.voiceEngine") === "included"
      ? "included"
      : "system",
  );
  const voiceEngineRef = useRef(voiceEngine);
  voiceEngineRef.current = voiceEngine;
  const pickEngine = (e: "included" | "system") => {
    setVoiceEngine(e);
    localStorage.setItem("escriba.voiceEngine", e);
  };

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
      // Cascada nativa vía backend (voz incluida o del sistema, según el
      // selector); el webview queda como último respaldo.
      try {
        if (await commands.conversationSpeak(text, voiceEngineRef.current))
          return;
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
      // Los tres modos se restauran SIEMPRE, fuera del `if` de abajo: no
      // dependen de que haya turnos acumulados, y son justo los que quedaban
      // desincronizados al desmontar la vista. Si el backend dice que el audio
      // del sistema sigue capturando, el interruptor tiene que decirlo aunque
      // la sesión no tenga ni un turno todavía.
      setHandsFree(s.hands_free);
      setSysAudio(s.system_audio);
      setSysTranslate(s.sys_translate);

      if (s.turns.length > 0 || s.listening) {
        if (s.mode === "converse") {
          setMode("converse");
        } else {
          setMode("listen");
          setVariant(
            (VARIANTS as string[]).includes(s.mode)
              ? (s.mode as Variant)
              : "listen",
          );
        }
        setTurns(s.turns);
        setListening(s.listening);
        setPhase("active");
      }
    });
  }, []);

  // Lee un texto en un idioma arbitrario (el del otro lado de la reunión):
  // speechSynthesis con la mejor voz instalada para ese idioma, igual que el
  // Traductor. La cascada nativa no sirve acá (leería con la voz local).
  const speakIn = useCallback(
    (lang: string, text: string) => {
      if (!("speechSynthesis" in window)) return;
      window.speechSynthesis.cancel();
      const u = new SpeechSynthesisUtterance(text);
      u.lang = lang;
      const voice = pickVoice(lang);
      if (voice) u.voice = voice;
      u.rate = 1.0;
      window.speechSynthesis.speak(u);
    },
    [pickVoice],
  );

  // Cable de vuelta del Intérprete de reuniones (idea de John Walter): tu
  // dictado llega traducido desde el backend; se anexa al turno, se lee en
  // voz alta en el idioma del otro y ya quedó copiado listo para pegar.
  useEffect(() => {
    const unReply = listen<{
      text: string;
      lang: string;
      translated: boolean;
      failed: boolean;
    }>("conversation-reply-translated", (e) => {
      // La traducción local falló: NO mandamos español crudo a la reunión
      // (mejor silencio honesto); avisamos y marcamos el turno para que no te
      // pases la reunión creyendo que te traducen (auditoría #9).
      if (e.payload.failed) {
        toast.error(t("conversation.systemTranslate.replyFailed"));
        setTurns((prev) => {
          const next = [...prev];
          for (let i = next.length - 1; i >= 0; i--) {
            if (next[i].role === "user") {
              next[i] = {
                ...next[i],
                text: `${next[i].text}\n⚠ ${t("conversation.systemTranslate.notSent")}`,
              };
              break;
            }
          }
          return next;
        });
        return;
      }
      // La flecha ⇢ solo cuando hubo traducción real; si la frase ya iba en el
      // idioma del otro, sale tal cual al cable sin marca.
      if (e.payload.translated) {
        setTurns((prev) => {
          const next = [...prev];
          for (let i = next.length - 1; i >= 0; i--) {
            if (next[i].role === "user") {
              next[i] = {
                ...next[i],
                text: `${next[i].text}\n⇢ ${e.payload.text}`,
              };
              break;
            }
          }
          return next;
        });
      }
      // Con dispositivo elegido, la voz sale por ahí (micrófono virtual = la
      // escucha la otra persona de la llamada); si falla, parlantes.
      const device = voiceOutRef.current;
      if (device) {
        commands
          .conversationSpeakVia(
            e.payload.text,
            e.payload.lang,
            device,
            voiceGenderRef.current,
          )
          .then((ok) => {
            if (!ok) speakIn(e.payload.lang, e.payload.text);
          })
          .catch(() => speakIn(e.payload.lang, e.payload.text));
      } else {
        speakIn(e.payload.lang, e.payload.text);
      }
    });
    return () => {
      unReply.then((fn) => fn());
    };
  }, [speakIn]);

  // Turnos en vivo desde el backend.
  useEffect(() => {
    const unTurn = listen<Turn>("conversation-turn", (e) => {
      setTurns((prev) => [...prev, e.payload]);
      if (e.payload.role === "user") {
        if (modeRef.current === "converse") setThinking(true);
      } else if (e.payload.role === "assistant") {
        // Solo la respuesta de la IA se lee en voz alta. Los turnos "system"
        // (audio del sistema) jamás: la app se pondría a repetir la reunión.
        setThinking(false);
        speak(e.payload.text);
      }
    });
    const unPhase = listen<string>("conversation-turn-phase", (e) => {
      const phase = e.payload;
      if (
        phase === "listening" ||
        phase === "closing" ||
        phase === "transcribing"
      ) {
        setTurnPhase(phase);
      }
    });
    const unFail = listen("conversation-reply-failed", () => {
      setThinking(false);
      toast.error(t("conversation.replyFailed"));
    });
    return () => {
      unTurn.then((fn) => fn());
      unPhase.then((fn) => fn());
      unFail.then((fn) => fn());
    };
  }, [speak, t]);

  // Autoscroll al último turno.
  useEffect(() => {
    scrollRef.current?.scrollTo({
      top: scrollRef.current.scrollHeight,
      behavior: scrollBehavior(),
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

  // La captura del audio del sistema se cayó sola (permiso revocado, audio
  // reiniciado): apagamos el toggle y avisamos en vez de fingir que escucha
  // (auditoría #10).
  useEffect(() => {
    const un = listen("system-audio-died", () => {
      setSysAudio(false);
      setSysTranslate(false);
      toast.error(t("conversation.systemAudio.died"));
    });
    return () => {
      un.then((fn) => fn());
    };
  }, [t]);

  // El modo efectivo que viaja al backend: converse, o la variante de escucha.
  const effectiveMode = mode === "converse" ? "converse" : variant;

  const start = async () => {
    const s = await commands.conversationStart(effectiveMode);
    setTurns(s.turns);
    setListening(true);
    setPhase("active");
  };

  // Manos libres: el micrófono queda abierto y el VAD corta cada intervención.
  const [handsFree, setHandsFree] = useState(false);
  // Fase del turno en manos libres, emitida por el backend. Durante la cuenta
  // de silencio la interfaz decía "escuchando", así que el usuario no
  // distinguía "te sigo oyendo" de "ya terminé, estoy esperando" y tendía a
  // repetirse o a hablar encima.
  const [turnPhase, setTurnPhase] = useState<
    "listening" | "closing" | "transcribing"
  >("listening");
  const toggleHandsFree = async () => {
    const next = !handsFree;
    const r = await commands.conversationHandsFree(next);
    if (r.status === "ok") {
      setHandsFree(r.data);
    } else {
      toast.error(t("conversation.handsFree.error"));
    }
  };

  // Audio del sistema: lo que suena en el computador (la otra parte de una
  // reunión Zoom/Meet, un video) entra a la sesión como turnos de "Otros".
  const [sysAudio, setSysAudio] = useState(false);
  // Intérprete de reuniones (idea de John Walter): traducir los turnos de
  // "Otros" al idioma del usuario. Solo tiene sentido con Audio del sistema.
  const [sysTranslate, setSysTranslate] = useState(false);
  const [sysTranslateLang, setSysTranslateLang] = useState("en");
  const toggleSysTranslate = async () => {
    const next = !sysTranslate;
    await commands.conversationSystemTranslate(next, sysTranslateLang);
    setSysTranslate(next);
  };
  const changeSysTranslateLang = async (lang: string) => {
    setSysTranslateLang(lang);
    if (sysTranslate) {
      await commands.conversationSystemTranslate(true, lang);
    }
  };
  // Voz de salida del Intérprete (remate de la idea de John Walter): "" = los
  // parlantes (speechSynthesis); un dispositivo = voz renderizada vía backend.
  // Con un micrófono virtual (BlackHole) elegido aquí y como micrófono de la
  // reunión, la otra persona escucha tu dictado ya traducido en la llamada.
  const [voiceOut, setVoiceOut] = useState<string>(
    () => localStorage.getItem("escriba.interpreterVoiceOut") ?? "",
  );
  const voiceOutRef = useRef(voiceOut);
  voiceOutRef.current = voiceOut;
  const pickVoiceOut = (device: string) => {
    setVoiceOut(device);
    localStorage.setItem("escriba.interpreterVoiceOut", device);
  };
  // Género de la voz del Intérprete (pedido de Alejandro en QA): mujer u
  // hombre, persistente. El backend elige la mejor voz de ese género.
  const [voiceGender, setVoiceGender] = useState<string>(
    () => localStorage.getItem("escriba.interpreterVoiceGender") ?? "f",
  );
  const voiceGenderRef = useRef(voiceGender);
  voiceGenderRef.current = voiceGender;
  const pickVoiceGender = (g: string) => {
    setVoiceGender(g);
    localStorage.setItem("escriba.interpreterVoiceGender", g);
  };
  const [outputDevices, setOutputDevices] = useState<string[]>([]);
  // Micrófono virtual integrado (patrón del motor Qwen): si falta, un botón
  // lo descarga verificado y lo instala sin salir de Escriba; macOS pide la
  // contraseña con su diálogo nativo (es un driver del sistema).
  // Ante la duda (comando viejo, error), el botón se muestra: peor es
  // esconder el instalador a quien lo necesita.
  const [vmInstalled, setVmInstalled] = useState(false);
  const [vmInstalling, setVmInstalling] = useState(false);
  useEffect(() => {
    if (!sysTranslate) return;
    commands.getAvailableOutputDevices().then((r) => {
      // "Default" ya está como opción propia (Parlantes): fuera el duplicado.
      if (r.status === "ok")
        setOutputDevices(
          r.data.map((d) => d.name).filter((n) => n !== "Default"),
        );
    });
    commands
      .virtualMicInstalled()
      .then(setVmInstalled)
      .catch(() => setVmInstalled(false));
  }, [sysTranslate]);
  // Traduce las claves de error del micrófono virtual (auditoría #14).
  const vmError = (key: string): string => {
    const map: Record<string, string> = {
      "vm.install_failed": t("conversation.systemTranslate.vmInstallFailed"),
      "vm.uninstall_failed": t(
        "conversation.systemTranslate.vmUninstallFailed",
      ),
      "vm.download_failed": t("conversation.systemTranslate.vmDownloadFailed"),
      "vm.sha_failed": t("conversation.systemTranslate.vmShaFailed"),
      "vm.only_macos": t("conversation.systemTranslate.vmOnlyMacos"),
    };
    return map[key] ?? key;
  };
  const installVirtualMic = async () => {
    // Instalar reinicia coreaudiod, lo que corta un instante el audio de la
    // Mac y puede tumbar la captura del sistema en plena llamada (auditoría
    // #6). Si hay sesión con audio activo, avisamos antes.
    if (listening && sysAudio) {
      const proceed = await confirmDialog(
        t("conversation.systemTranslate.vmRestartWarn"),
        { title: "Escriba", kind: "warning" },
      );
      if (!proceed) return;
    }
    setVmInstalling(true);
    try {
      const r = await commands.virtualMicInstall();
      if (r.status === "error") {
        toast.error(vmError(r.error));
        return;
      }
      if (!r.data) return; // canceló el diálogo de contraseña: sin drama
      setVmInstalled(true);
      const devs = await commands.getAvailableOutputDevices();
      if (devs.status === "ok") {
        setOutputDevices(devs.data.map((d) => d.name));
        const bh = devs.data.find((d) => d.name.includes("BlackHole"));
        if (bh) pickVoiceOut(bh.name);
      }
      toast.success(t("conversation.systemTranslate.vmReady"));
    } finally {
      setVmInstalling(false);
    }
  };
  const uninstallVirtualMic = async () => {
    const proceed = await confirmDialog(
      t("conversation.systemTranslate.vmUninstallConfirm"),
      { title: "Escriba", kind: "warning" },
    );
    if (!proceed) return;
    const r = await commands.virtualMicUninstall();
    if (r.status === "error") {
      toast.error(vmError(r.error));
      return;
    }
    if (!r.data) return; // canceló el diálogo
    setVmInstalled(false);
    if (voiceOut.includes("BlackHole")) pickVoiceOut("");
    toast.success(t("conversation.systemTranslate.vmUninstalled"));
  };
  const [sysAudioSupported, setSysAudioSupported] = useState(false);
  useEffect(() => {
    commands.systemAudioSupported().then(setSysAudioSupported);
  }, []);
  const toggleSysAudio = async () => {
    const next = !sysAudio;
    const r = await commands.conversationSystemAudio(next);
    if (r.status === "ok") {
      setSysAudio(r.data);
      // Para un acta de ambos lados hace falta también el micrófono abierto.
      if (r.data && !handsFree) {
        toast.info(t("conversation.systemAudio.micTip"), { duration: 7000 });
      }
    } else if (r.error === "screen_permission") {
      toast.error(t("conversation.systemAudio.permissionTitle"), {
        description: t("conversation.systemAudio.permissionHint"),
        duration: 9000,
      });
    } else {
      toast.error(t("conversation.systemAudio.error"));
    }
  };

  const togglePause = async () => {
    const s = listening
      ? await commands.conversationStop()
      : await commands.conversationStart(effectiveMode);
    setListening(s.listening);
    // Se lee lo que el backend REALMENTE dejó, en vez de suponer que apagó los
    // tres. Antes se ponían a `false` a mano, que acertaba solo porque
    // `conversation_stop` los apaga: la interfaz estaba adivinando el efecto de
    // un comando en lugar de leer su resultado.
    setHandsFree(s.hands_free);
    setSysAudio(s.system_audio);
    setSysTranslate(s.sys_translate);
    if (!s.listening) {
      commands.conversationSystemTranslate(false, sysTranslateLang);
    }
  };

  const discard = async () => {
    // `conversation_reset` apaga los tres y devuelve el estado ya reconciliado.
    const s = await commands.conversationReset();
    setHandsFree(s.hands_free);
    setSysAudio(s.system_audio);
    setSysTranslate(s.sys_translate);
    window.speechSynthesis?.cancel();
    commands.conversationSpeakStop();
    setTurns([]);
    setThinking(false);
    setDoc("");
    setPhase("idle");
    setListening(false);
  };

  // Voz neural incluida: estado + instalación con progreso.
  const [ttsReady, setTtsReady] = useState<boolean | null>(null);
  const [ttsBusy, setTtsBusy] = useState(false);
  const [ttsPct, setTtsPct] = useState(0);
  useEffect(() => {
    commands.ttsStatus().then(setTtsReady);
    const un = listen<{ stage: string; downloaded: number; total: number }>(
      "tts-setup-progress",
      (e) => {
        const { downloaded, total, stage } = e.payload;
        if (stage === "voice" && total > 0) {
          // El runtime es ~30% del total; la voz completa el resto.
          setTtsPct(30 + Math.round((downloaded / total) * 70));
        } else if (stage === "runtime" && total > 0) {
          setTtsPct(Math.round((downloaded / total) * 30));
        }
      },
    );
    return () => {
      un.then((fn) => fn());
    };
  }, []);

  // Voz neural del Intérprete para el idioma de la reunión (en/es la tienen;
  // otros idiomas usan la voz del sistema). Estado + descarga con el mismo
  // progreso (ttsPct). La voz neural es femenina: mejora la calidad cuando se
  // pide ♀; con ♂ el Intérprete sigue usando la voz del sistema.
  const [neuralReady, setNeuralReady] = useState(false);
  const [neuralBusy, setNeuralBusy] = useState(false);
  useEffect(() => {
    if (!NEURAL_VOICE_LANGS.includes(sysTranslateLang)) {
      setNeuralReady(false);
      return;
    }
    commands.interpreterVoiceStatus(sysTranslateLang).then(setNeuralReady);
  }, [sysTranslateLang]);
  const installNeuralVoice = async () => {
    setNeuralBusy(true);
    setTtsPct(0);
    try {
      const r = await commands.interpreterVoiceSetup(sysTranslateLang);
      if (r.status === "ok") {
        setNeuralReady(true);
        toast.success(t("conversation.systemTranslate.neuralReady"));
      } else {
        toast.error(r.error);
      }
    } finally {
      setNeuralBusy(false);
    }
  };

  const installTts = async () => {
    setTtsBusy(true);
    setTtsPct(0);
    try {
      const r = await commands.ttsSetup();
      if (r.status === "ok") {
        setTtsReady(true);
        toast.success(t("conversation.tts.ready"));
      } else {
        toast.error(t("conversation.tts.error"), { description: r.error });
      }
    } finally {
      setTtsBusy(false);
    }
  };

  const finish = async () => {
    if (turns.length === 0) {
      toast.error(t("conversation.emptyFinish"));
      return;
    }
    setCreating(true);
    setListening(false);
    setHandsFree(false);
    setSysAudio(false);
    try {
      const r = await commands.conversationFinish();
      if (r.status === "ok") {
        setDoc(r.data.text);
        setDocMood(r.data.mood);
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

  // Enviar a Obsidian: el título es la primera línea con texto del documento
  // (o el nombre del modo), y el cuerpo es el documento tal cual.
  const [sendingObsidian, setSendingObsidian] = useState(false);
  const sendDocToObsidian = async () => {
    const firstLine =
      doc
        .split("\n")
        .map((l) => l.trim())
        .find((l) => l.length > 0) || t(`conversation.variant.${variant}`);
    const title = firstLine.replace(/^#+\s*/, "").slice(0, 80);
    setSendingObsidian(true);
    try {
      requestObsidianExport(title, doc);
    } finally {
      setSendingObsidian(false);
    }
  };

  const MODES: { id: Mode; icon: typeof Mic }[] = [
    { id: "converse", icon: MessageCircle },
    { id: "listen", icon: Ear },
  ];

  // Atajo de dictado real, para enseñarlo desde el primer día.
  const bindings = getSetting("bindings") as
    Record<string, { current_binding?: string }> | undefined;
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
          <EngineRequiredCard />
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
                          ? "bg-logo-primary/20 text-gold-text"
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
                      <span className="block text-2xs leading-snug text-mid-gray">
                        {t(`conversation.mode.${id}.desc`)}
                      </span>
                    </div>
                    {active && (
                      <Check
                        width={16}
                        height={16}
                        className="ml-auto shrink-0 text-gold-text"
                      />
                    )}
                  </div>
                  {/* Variantes de escucha: mismo núcleo, documento distinto. */}
                  {id === "listen" && active && (
                    <div className="mt-3 border-t border-line pt-2.5">
                      <p className="mb-1.5 text-3xs font-semibold uppercase tracking-wide text-mid-gray">
                        {t("conversation.variantTitle")}
                      </p>
                      <div className="flex flex-wrap gap-1.5">
                        {VARIANTS.map((v) => (
                          <span
                            key={v}
                            role="button"
                            tabIndex={0}
                            onClick={(e) => {
                              e.stopPropagation();
                              setVariant(v);
                            }}
                            onKeyDown={(e) => {
                              if (e.key === "Enter" || e.key === " ") {
                                e.stopPropagation();
                                setVariant(v);
                              }
                            }}
                            title={t(`conversation.variantDoc.${v}`)}
                            className={`cursor-pointer rounded-full border px-2.5 py-1 text-2xs font-medium transition-colors ${
                              variant === v
                                ? "border-logo-primary bg-logo-primary/15 text-text"
                                : "border-line text-mid-gray hover:border-mid-gray/40"
                            }`}
                          >
                            {t(`conversation.variant.${v}`)}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
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
                    className="rounded-md border border-mid-gray/25 bg-background px-1.5 py-0.5 font-mono text-2xs text-text"
                  >
                    {k}
                  </kbd>
                ))}
              </p>
            )}
          </div>

          {/* Voz neural incluida: instalación de una sola vez. */}
          {ttsReady === false && (
            <Card className="flex flex-wrap items-center gap-3 px-4 py-3.5">
              <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-logo-primary/10 text-gold-text">
                <Sparkles width={17} height={17} />
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-semibold text-text">
                  {t("conversation.tts.title")}
                </p>
                <p className="text-xs leading-snug text-mid-gray">
                  {ttsBusy
                    ? t("conversation.tts.installing")
                    : t("conversation.tts.desc")}
                </p>
                {ttsBusy && (
                  <div className="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-mid-gray/15">
                    <div
                      className="h-full rounded-full bg-logo-primary transition-all duration-300"
                      style={{ width: `${ttsPct}%` }}
                    />
                  </div>
                )}
              </div>
              {!ttsBusy && (
                <Button variant="secondary" size="sm" onClick={installTts}>
                  {t("conversation.tts.install")}
                </Button>
              )}
            </Card>
          )}

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
                        ? "border-logo-primary/40 bg-logo-primary/10 text-gold-text"
                        : "border-line bg-background text-mid-gray"
                    }`}
                  >
                    {Icon && <Icon width={14} height={14} />}
                  </span>
                  <span className="text-center text-2xs leading-tight text-mid-gray">
                    {t(`conversation.flow.${key}`)}
                  </span>
                </div>
              </React.Fragment>
            ))}
          </div>

          {/* Casos de uso: entrada → resultado, con el principal destacado. */}
          <div>
            <p className="mb-2 font-mono text-3xs font-semibold uppercase tracking-[0.14em] text-mid-gray">
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
                        className="shrink-0 fill-logo-primary/30 text-gold-text"
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
            <p className="mb-2 text-center font-mono text-3xs font-semibold uppercase tracking-[0.14em] text-mid-gray">
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
                    <p className="text-3xs font-semibold uppercase tracking-wide text-gold-text">
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
              <Lock width={14} height={14} className="text-gold-text" />
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
                    className="mt-0.5 shrink-0 text-gold-text"
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
                  <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-success/60" />
                )}
                <span
                  className={`relative inline-flex h-2.5 w-2.5 rounded-full ${
                    listening ? "bg-success" : "bg-mid-gray/50"
                  }`}
                />
              </span>
              <span className="text-sm font-semibold text-text">
                {listening
                  ? t("conversation.active")
                  : t("conversation.paused")}
              </span>
              <span className="rounded-full border border-line px-2 py-0.5 text-2xs text-mid-gray">
                {mode === "converse"
                  ? t("conversation.mode.converse.label")
                  : t(`conversation.variant.${variant}`)}
              </span>
            </div>
            <div className="flex items-center gap-2">
              {/* Manos libres: solo en modos de escucha (en Conversar la voz
                  de respuesta se re-capturaría a sí misma). */}
              {mode !== "converse" && listening && (
                <>
                  <Button
                    variant={handsFree ? "primary-soft" : "secondary"}
                    size="sm"
                    onClick={toggleHandsFree}
                  >
                    <Mic width={13} height={13} className="mr-1.5" />
                    {t("conversation.handsFree.label")}
                  </Button>
                  {handsFree && turnPhase !== "listening" && (
                    <span
                      role="status"
                      aria-live="polite"
                      className="text-xs text-mid-gray"
                    >
                      {turnPhase === "closing"
                        ? t("overlay.closing")
                        : t("overlay.transcribing")}
                    </span>
                  )}
                </>
              )}
              {/* Audio del sistema: solo modos de escucha y equipos que lo
                  soportan (macOS 13+). Capta la otra parte de la reunión. */}
              {mode !== "converse" && listening && sysAudioSupported && (
                <Button
                  variant={sysAudio ? "primary-soft" : "secondary"}
                  size="sm"
                  onClick={toggleSysAudio}
                >
                  <MonitorSpeaker width={13} height={13} className="mr-1.5" />
                  {t("conversation.systemAudio.label")}
                </Button>
              )}
              {/* Intérprete de reuniones (idea de John Walter): traducir los
                  turnos de "Otros" al idioma de la app. */}
              {mode !== "converse" && listening && sysAudio && (
                <div className="flex items-center gap-1">
                  <Button
                    variant={sysTranslate ? "primary-soft" : "secondary"}
                    size="sm"
                    onClick={toggleSysTranslate}
                    title={t("conversation.systemTranslate.hint")}
                  >
                    <Languages width={13} height={13} className="mr-1.5" />
                    {t("conversation.systemTranslate.label")}
                  </Button>
                  {sysTranslate && (
                    /* Un solo grupo sellado (idioma · salida · voz): se lee
                       como extensión del botón Traducir, no como formulario. */
                    <div className="flex items-stretch divide-x divide-logo-primary/15 overflow-hidden rounded-lg border border-logo-primary/25 bg-logo-primary/5">
                      <select
                        value={sysTranslateLang}
                        onChange={(e) => changeSysTranslateLang(e.target.value)}
                        title={t("conversation.systemTranslate.langHint")}
                        className="cursor-pointer appearance-none bg-transparent px-2.5 py-1 text-xs text-text transition-colors hover:bg-logo-primary/10 focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
                      >
                        {MEETING_LANGUAGES.map((l) => (
                          <option key={l.value} value={l.value}>
                            {l.flag} {l.label}
                          </option>
                        ))}
                      </select>
                      {outputDevices.length > 0 && (
                        <select
                          value={voiceOut}
                          onChange={(e) => pickVoiceOut(e.target.value)}
                          title={t("conversation.systemTranslate.voiceOutHint")}
                          className="max-w-36 cursor-pointer appearance-none bg-transparent px-2.5 py-1 text-xs text-text transition-colors hover:bg-logo-primary/10 focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
                        >
                          <option value="">
                            {"🔊 " + t("conversation.systemTranslate.speakers")}
                          </option>
                          {outputDevices.map((d) => (
                            <option key={d} value={d}>
                              {"🎙️ " + d}
                            </option>
                          ))}
                        </select>
                      )}
                      <select
                        value={voiceGender}
                        onChange={(e) => pickVoiceGender(e.target.value)}
                        title={t(
                          "conversation.systemTranslate.voiceGenderHint",
                        )}
                        className="cursor-pointer appearance-none bg-transparent px-2.5 py-1 text-xs text-text transition-colors hover:bg-logo-primary/10 focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
                      >
                        <option value="f">
                          {"♀ " + t("conversation.systemTranslate.voiceFemale")}
                        </option>
                        <option value="m">
                          {"♂ " + t("conversation.systemTranslate.voiceMale")}
                        </option>
                      </select>
                    </div>
                  )}
                  {sysTranslate &&
                    NEURAL_VOICE_LANGS.includes(sysTranslateLang) &&
                    !neuralReady && (
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={installNeuralVoice}
                        disabled={neuralBusy}
                        title={t("conversation.systemTranslate.neuralHint")}
                      >
                        <Sparkles width={13} height={13} className="mr-1.5" />
                        {neuralBusy
                          ? `${ttsPct}%`
                          : t("conversation.systemTranslate.neuralInstall")}
                      </Button>
                    )}
                  {sysTranslate && !vmInstalled && (
                    <Button
                      variant="secondary"
                      size="sm"
                      onClick={installVirtualMic}
                      disabled={vmInstalling}
                      title={t("conversation.systemTranslate.vmInstallHint")}
                    >
                      <Mic width={13} height={13} className="mr-1.5" />
                      {vmInstalling
                        ? t("conversation.systemTranslate.vmInstalling")
                        : t("conversation.systemTranslate.vmInstall")}
                    </Button>
                  )}
                </div>
              )}
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
                  plumin="escucha"
                  // Con Manos libres activo, la instrucción del atajo es la del
                  // modo CONTRARIO: la usuaria lee "mantén tu atajo" cuando lo
                  // correcto es solo hablar (QA de Flor, 29-jul: su captura
                  // muestra el botón encendido y el texto del atajo debajo).
                  title={t(
                    handsFree
                      ? "conversation.emptyHandsFree"
                      : "conversation.empty",
                  )}
                  description={t(
                    handsFree
                      ? "conversation.hintHandsFree"
                      : "conversation.hint",
                  )}
                />
              ) : (
                <>
                  {turns.map((turn, i) =>
                    turn.role === "assistant" ? (
                      <div
                        key={i}
                        className="me-8 rounded-2xl rounded-tl-sm border border-logo-primary/25 bg-logo-primary/5 px-4 py-3"
                      >
                        <p className="mb-1 text-3xs uppercase tracking-wide text-gold-text">
                          {t("conversation.assistant")}
                        </p>
                        <p
                          className="text-base leading-snug text-text"
                          style={{ fontFamily: "var(--font-serif)" }}
                        >
                          {turn.text}
                        </p>
                      </div>
                    ) : turn.role === "system" ? (
                      // Audio del sistema: la otra parte de la reunión / el video.
                      <div
                        key={i}
                        className="me-8 rounded-2xl rounded-tl-sm border border-line bg-mid-gray/5 px-4 py-3"
                      >
                        <p className="mb-1 flex items-center justify-between text-3xs uppercase tracking-wide text-mid-gray">
                          <span className="flex items-center gap-1.5">
                            <MonitorSpeaker width={11} height={11} />
                            {t("conversation.others")}
                          </span>
                          <span className="font-mono normal-case">
                            {stamp(turn.at_secs)}
                          </span>
                        </p>
                        <p className="text-sm leading-relaxed text-text/85">
                          {turn.text}
                        </p>
                      </div>
                    ) : (
                      <div
                        key={i}
                        className="ms-8 rounded-2xl rounded-tr-sm bg-mid-gray/5 px-4 py-3"
                      >
                        <p className="mb-1 flex items-center justify-between text-3xs uppercase tracking-wide text-mid-gray">
                          <span>{t("conversation.you")}</span>
                          <span className="font-mono normal-case">
                            {stamp(turn.at_secs)}
                          </span>
                        </p>
                        <p className="whitespace-pre-line text-sm leading-relaxed text-text/85">
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
                        className="animate-pulse text-gold-text"
                      />
                      {t("conversation.thinking")}
                    </p>
                  )}
                </>
              )}
            </div>
            {turns.length > 0 && (
              <p className="mt-3 border-t border-line pt-2 text-center text-2xs text-mid-gray">
                {sysTranslate
                  ? t("conversation.systemTranslate.active")
                  : sysAudio
                    ? t("conversation.systemAudio.hint")
                    : handsFree
                      ? t("conversation.handsFree.hint")
                      : t("conversation.hint")}
              </p>
            )}
            {sysTranslate && voiceOut && (
              <p className="mt-1 text-center text-2xs text-mid-gray">
                {t("conversation.systemTranslate.virtualMicHint")}
              </p>
            )}
            {sysTranslate && vmInstalled && (
              <p className="mt-1 text-center text-2xs text-mid-gray">
                <button
                  type="button"
                  onClick={uninstallVirtualMic}
                  className="cursor-pointer underline-offset-2 hover:underline"
                >
                  {t("conversation.systemTranslate.vmUninstall")}
                </button>
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
                }}
                label={t("conversation.speakAloud")}
                description={NO_DESCRIPTION}
              />
              {/* Selector de motor: la voz del sistema (ganadora del A/B) vs
                  la incluida (para equipos sin voces Premium). */}
              {voiceOn && ttsReady === true && (
                <div className="flex items-center gap-1.5 border-t border-line py-2.5">
                  {(["system", "included"] as const).map((e) => (
                    <button
                      key={e}
                      type="button"
                      onClick={() => pickEngine(e)}
                      className={`rounded-full border px-3 py-1 text-2xs font-medium transition-colors ${
                        voiceEngine === e
                          ? "border-logo-primary bg-logo-primary/15 text-text"
                          : "border-line text-mid-gray hover:border-mid-gray/40"
                      }`}
                    >
                      {t(`conversation.voiceEngine.${e}`)}
                    </button>
                  ))}
                </div>
              )}
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
          {/* Plumín entrega el documento con la carita según el ánimo de la
              sesión (idea de Pedro Sánchez): tensa → empatía, el resto → fiesta. */}
          <div className="flex justify-center">
            <Plumin
              pose={docMood === "tenso" ? "disculpa" : "celebra"}
              size={92}
            />
          </div>
          <Card className="p-5">
            <div className="mb-3 flex items-center justify-between border-b border-line pb-3">
              <p className="flex items-center gap-2 text-sm font-semibold text-text">
                <FileText width={15} height={15} className="text-gold-text" />
                {t("conversation.docTitle")}
              </p>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={sendDocToObsidian}
                  disabled={sendingObsidian}
                  className="flex items-center gap-1.5 rounded-lg border border-mid-gray/20 px-3 py-1.5 text-xs font-medium text-text transition-colors hover:border-mid-gray/40 disabled:opacity-60"
                >
                  <FileDown width={13} height={13} />
                  {sendingObsidian ? t("obsidian.sending") : t("obsidian.send")}
                </button>
                <button
                  type="button"
                  onClick={copyDoc}
                  className="flex items-center gap-1.5 rounded-lg border border-mid-gray/20 px-3 py-1.5 text-xs font-medium text-text transition-colors hover:border-mid-gray/40"
                >
                  {copied ? (
                    <>
                      <Check width={13} height={13} className="text-success" />
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
