import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Languages, MessageCircle, Mic, Radio, Square } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { commands } from "@/bindings";
import type { SidebarSection } from "../Sidebar";

type ActiveMode = {
  // "free" = Dictado libre: no tiene pantalla propia, solo Detener.
  id:
    | Extract<SidebarSection, "conversation" | "interpreter" | "translator">
    | "free";
  labelKey: string;
  icon: LucideIcon;
};

/**
 * Indicador global de modo activo: si hay una sala del Intérprete, un
 * Traductor escuchando o una Sesión viva, un pill persistente lo dice desde
 * cualquier pantalla, con acceso directo y botón Detener. Sin esto, una sala
 * olvidada se traga los dictados en silencio y el usuario no tiene forma de
 * saber por qué "el dictado dejó de andar" (hallazgo de Flor, 15-jul).
 */
export const ActiveModeBanner: React.FC<{
  currentSection: SidebarSection;
  onGo: (s: SidebarSection) => void;
}> = ({ currentSection, onGo }) => {
  const { t } = useTranslation();
  const [mode, setMode] = useState<ActiveMode | null>(null);
  // El planificador del sondeo necesita saber si hay un modo activo sin que eso
  // vuelva a crear el efecto (recrearlo reiniciaría el temporizador en cada
  // cambio de estado).
  const activeRef = useRef(false);
  activeRef.current = mode !== null;

  useEffect(() => {
    let alive = true;
    const poll = async () => {
      try {
        const [conv, interp, trans, free] = await Promise.all([
          commands.conversationStatus(),
          commands.interpreterStatus(),
          commands.translatorStatus(),
          commands.freeDictationStatus(),
        ]);
        if (!alive) return;
        // Prioridad = orden de los interceptores del backend: quien captura
        // primero el dictado es lo primero que hay que mostrar.
        if (free) {
          setMode({
            id: "free",
            labelKey: "activeMode.freeDictation",
            icon: Mic,
          });
        } else if (conv.listening) {
          setMode({
            id: "conversation",
            labelKey: "activeMode.conversation",
            icon: MessageCircle,
          });
        } else if (trans.listening) {
          setMode({
            id: "translator",
            labelKey: "activeMode.translator",
            icon: Languages,
          });
        } else if (interp.running) {
          setMode({
            id: "interpreter",
            labelKey: "activeMode.interpreter",
            icon: Radio,
          });
        } else {
          setMode(null);
        }
      } catch {
        /* backend aún no listo: sin banner */
      }
    };
    // Este banner está montado en toda la app, así que su sondeo es el costo
    // de fondo permanente: a 2,5 s fijos eran 4 llamadas IPC cada tick, ~96 por
    // minuto, incluso con la app minimizada a la bandeja (que es donde vive la
    // mayor parte del tiempo). Ahora el ritmo se adapta:
    //
    //   - ventana oculta: no se sondea, y se hace un sondeo al volver;
    //   - sin modo activo: cada 10 s, que es de sobra para un aviso pasivo;
    //   - con modo activo: 2,5 s, para que "Detener" refleje la realidad.
    let timer: number | undefined;

    const schedule = () => {
      if (!alive) return;
      const delay = activeRef.current ? 2500 : 10000;
      timer = window.setTimeout(tick, delay);
    };

    const tick = async () => {
      if (!document.hidden) await poll();
      schedule();
    };

    const onVisibility = () => {
      if (!document.hidden) void poll();
    };

    void poll();
    schedule();
    document.addEventListener("visibilitychange", onVisibility);

    return () => {
      alive = false;
      if (timer !== undefined) window.clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, []);

  // En la pantalla del propio modo ya hay UI de estado: el pill solo aparece
  // donde el usuario NO está viendo esa feature.
  if (!mode || mode.id === currentSection) return null;

  const stop = async () => {
    if (mode.id === "free") await commands.freeDictationSet(false);
    else if (mode.id === "conversation") await commands.conversationStop();
    else if (mode.id === "translator")
      await commands.translatorSetListening(false);
    else await commands.interpreterStop();
    setMode(null);
  };

  const Icon = mode.icon;

  return (
    // `role="status"` para que activar una sala o una sesión se anuncie: era
    // uno de los cambios de estado importantes que la app hacía en silencio.
    <div
      role="status"
      className="fixed bottom-5 left-1/2 z-40 flex -translate-x-1/2 items-center gap-3 rounded-full border border-white/15 bg-ink py-2 pe-2 ps-4 text-ink-fg shadow-lift"
    >
      <span className="relative flex h-2 w-2" aria-hidden="true">
        <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-500/60" />
        <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
      </span>
      <Icon width={14} height={14} aria-hidden="true" />
      <span className="text-xs font-medium">{t(mode.labelKey)}</span>
      {mode.id !== "free" && (
        <button
          type="button"
          onClick={() => onGo(mode.id as SidebarSection)}
          className="rounded-full px-2.5 py-1 text-xs text-ink-fg/80 transition-colors hover:bg-white/10 focus:outline-none focus:ring-1 focus:ring-white/40"
        >
          {t("activeMode.view")}
        </button>
      )}
      <button
        type="button"
        onClick={stop}
        className="flex items-center gap-1.5 rounded-full bg-white/10 px-2.5 py-1 text-xs font-medium transition-colors hover:bg-white/20 focus:outline-none focus:ring-1 focus:ring-white/40"
      >
        <Square width={10} height={10} />
        {t("activeMode.stop")}
      </button>
    </div>
  );
};
