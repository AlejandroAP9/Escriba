import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Mic, Loader2 } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { commands } from "@/bindings";

/**
 * Botón de micrófono reusable para CUALQUIER campo de texto de la app: click
 * para grabar, click de nuevo para detener; el texto transcrito (100% local) se
 * inserta en el campo vía `onText`. Reusa el pipeline de dictado global, pero el
 * backend desvía el texto a este campo en vez de pegarlo en la app enfocada.
 *
 * Solo una grabación a la vez (lo impone el coordinador del backend). Un único
 * listener global reparte el resultado al botón activo.
 */

type Receiver = (text: string) => void;
let activeReceiver: Receiver | null = null;
let listenerReady = false;

function ensureGlobalListener() {
  if (listenerReady) return;
  listenerReady = true;
  listen<string>("field-dictation-result", (e) => {
    const receiver = activeReceiver;
    activeReceiver = null;
    receiver?.(e.payload);
  });
}

interface MicButtonProps {
  onText: (text: string) => void;
  disabled?: boolean;
  title?: string;
}

export const MicButton: React.FC<MicButtonProps> = ({
  onText,
  disabled,
  title,
}) => {
  const { t } = useTranslation();
  const [state, setState] = useState<"idle" | "recording" | "processing">(
    "idle",
  );
  const timeoutRef = useRef<number | null>(null);

  useEffect(() => {
    ensureGlobalListener();
    return () => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    };
  }, []);

  const reset = useCallback(() => {
    if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    setState("idle");
  }, []);

  const start = useCallback(async () => {
    if (await commands.isRecording()) return; // otra grabación en curso
    activeReceiver = (text: string) => {
      if (text.trim()) onText(text);
      reset();
    };
    setState("recording");
    await commands.fieldDictationToggle();
  }, [onText, reset]);

  const stop = useCallback(async () => {
    setState("processing");
    await commands.fieldDictationToggle();
    // Red de seguridad: si el backend no emitiera (p.ej. sin audio), vuelve a
    // idle tras un tiempo prudente.
    timeoutRef.current = window.setTimeout(() => {
      activeReceiver = null;
      setState("idle");
    }, 30000);
  }, []);

  const handleClick = () => {
    if (disabled) return;
    if (state === "idle") start();
    else if (state === "recording") stop();
  };

  const label =
    state === "recording"
      ? t("micButton.stop")
      : state === "processing"
        ? t("micButton.processing")
        : title || t("micButton.start");

  return (
    <button
      type="button"
      onClick={handleClick}
      disabled={disabled || state === "processing"}
      title={label}
      aria-label={label}
      className={`shrink-0 p-1.5 rounded-md flex items-center justify-center transition-colors disabled:opacity-40 ${
        state === "recording"
          ? "text-lacre bg-lacre/10 animate-pulse"
          : "text-mid-gray hover:text-text hover:bg-mid-gray/20"
      }`}
    >
      {state === "processing" ? (
        <Loader2 width={16} height={16} className="animate-spin" />
      ) : (
        <Mic width={16} height={16} />
      )}
    </button>
  );
};
