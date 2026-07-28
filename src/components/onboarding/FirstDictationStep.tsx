import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, Mic, Square } from "lucide-react";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { WizardShell } from "./WizardShell";
import { Button } from "../ui/Button";

interface FirstDictationStepProps {
  step: number;
  totalSteps: number;
  onBack: () => void;
  onDone: () => void;
  /** Atajo real de dictado, para enseñarlo mientras se prueba. */
  shortcut: string;
}

type Phase = "idle" | "recording" | "processing";

/**
 * El paso que nos diferencia: la persona termina el asistente habiendo dictado
 * de verdad y visto SUS palabras, no una maqueta.
 *
 * Hace dos cosas a la vez:
 *
 * 1. **Prueba la cadena completa** —micrófono, modelo, transcripción— delante
 *    de quien instala. Hasta ahora, si algo de eso estaba mal, se enteraba solo
 *    y más tarde, sin saber qué pieza falló.
 * 2. **Descubre el resto con su propio texto.** Traducir la frase que acabas de
 *    decir enseña más que cualquier captura, porque el material es tuyo.
 *
 * Usa el camino de dictado en campo (`fieldDictationToggle`) y NO el atajo
 * global: los atajos no se inicializan hasta que el asistente termina, y en
 * macOS dependen del permiso de Accesibilidad que quizá se acaba de conceder.
 * El atajo real se enseña en pantalla, pero la prueba no depende de él.
 */
export const FirstDictationStep: React.FC<FirstDictationStepProps> = ({
  step,
  totalSteps,
  onBack,
  onDone,
  shortcut,
}) => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  // Traducir necesita el motor local, y el paso anterior deja saltarlo a
  // propósito (son 2,5 GB). Sin esto, quien lo salta pulsaba "Tradúcelo" y no
  // pasaba NADA: el comando devuelve POST_PROCESS_DISABLED y el botón solo
  // dejaba de girar. Ofrecer algo que no puede funcionar es peor que no
  // ofrecerlo.
  const canTranslate = (getSetting("post_process_enabled") ?? false) as boolean;
  const [phase, setPhase] = useState<Phase>("idle");
  const [text, setText] = useState("");
  const [translation, setTranslation] = useState("");
  const [translating, setTranslating] = useState(false);
  const timeoutRef = useRef<number | null>(null);

  useEffect(() => {
    const unlisten = listen<string>("field-dictation-result", (e) => {
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
      setPhase("idle");
      if (e.payload.trim()) setText(e.payload.trim());
    });
    return () => {
      unlisten.then((fn) => fn());
      if (timeoutRef.current) window.clearTimeout(timeoutRef.current);
    };
  }, []);

  const toggle = useCallback(async () => {
    if (phase === "idle") {
      if (await commands.isRecording()) return;
      setTranslation("");
      setPhase("recording");
      await commands.fieldDictationToggle();
    } else if (phase === "recording") {
      setPhase("processing");
      await commands.fieldDictationToggle();
      // Red de seguridad: si el backend no emite (sin audio, modelo aún
      // cargando), no dejamos el botón atrapado en "procesando".
      timeoutRef.current = window.setTimeout(() => setPhase("idle"), 30000);
    }
  }, [phase]);

  const translate = async () => {
    setTranslating(true);
    try {
      const r = await commands.processTypedText(text, "translate");
      if (r.status === "ok") {
        setTranslation(r.data);
      } else {
        // Aunque el motor esté encendido puede fallar (caído, a medio
        // instalar). Que se diga, en vez de que el botón vuelva a su sitio
        // como si nada.
        toast.error(t("wizard.test.translateFailed"));
      }
    } finally {
      setTranslating(false);
    }
  };

  const hasText = text.trim().length > 0;

  return (
    <WizardShell
      step={step}
      totalSteps={totalSteps}
      pose={hasText ? "celebra" : "escucha"}
      title={hasText ? t("wizard.test.titleDone") : t("wizard.test.title")}
      narration={
        hasText ? t("wizard.test.narrationDone") : t("wizard.test.narration")
      }
      onBack={onBack}
      onNext={onDone}
      nextLabel={
        hasText ? t("wizard.test.finish") : t("wizard.test.finishEmpty")
      }
    >
      <div className="flex flex-col gap-4">
        {/* El lienzo. Vacío invita; con texto, es la voz de quien instala,
            en la serif de la marca y en grande: "tu voz en tinta", literal. */}
        <div
          className="flex min-h-[168px] flex-col justify-center rounded-card border border-line bg-vitela/25 px-6 py-5"
          role="status"
          aria-live="polite"
        >
          {hasText ? (
            <p
              className="text-xl leading-relaxed text-text"
              style={{ fontFamily: "var(--font-serif)" }}
            >
              {text}
            </p>
          ) : (
            <p className="text-center text-sm text-mid-gray">
              {phase === "recording"
                ? t("wizard.test.listening")
                : phase === "processing"
                  ? t("wizard.test.writing")
                  : t("wizard.test.placeholder")}
            </p>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            variant={phase === "recording" ? "danger" : "primary"}
            onClick={toggle}
            disabled={phase === "processing"}
          >
            <span className="flex items-center gap-2">
              {phase === "processing" ? (
                <Loader2 width={16} height={16} className="animate-spin" />
              ) : phase === "recording" ? (
                <Square width={14} height={14} fill="currentColor" />
              ) : (
                <Mic width={16} height={16} />
              )}
              {phase === "recording"
                ? t("wizard.test.stop")
                : phase === "processing"
                  ? t("wizard.test.writing")
                  : hasText
                    ? t("wizard.test.again")
                    : t("wizard.test.start")}
            </span>
          </Button>

          {hasText && canTranslate && (
            <Button
              variant="secondary"
              onClick={translate}
              disabled={translating}
            >
              {translating
                ? t("wizard.test.translating")
                : t("wizard.test.translate")}
            </Button>
          )}
        </div>

        {/* Sin motor, se explica qué falta en vez de dejar un botón muerto. */}
        {hasText && !canTranslate && (
          <p className="text-xs text-mid-gray">
            {t("wizard.test.needsEngine")}
          </p>
        )}

        {translation && (
          <div
            className="rounded-card border border-line bg-background px-5 py-4"
            role="status"
            aria-live="polite"
          >
            <p className="font-mono text-3xs uppercase tracking-[0.14em] text-mid-gray">
              {t("wizard.test.translationLabel")}
            </p>
            <p
              className="mt-1.5 text-lg leading-relaxed text-text"
              style={{ fontFamily: "var(--font-serif)" }}
            >
              {translation}
            </p>
            <p className="mt-2 text-2xs text-mid-gray">
              {t("wizard.test.translationNote")}
            </p>
          </div>
        )}

        {/* El atajo real, para que se lo lleven aprendido aunque la prueba de
            aquí no lo use. */}
        <p className="text-center text-xs text-mid-gray">
          {t("wizard.test.shortcutHint")}{" "}
          <kbd className="rounded border border-line bg-mid-gray/10 px-1.5 py-0.5 font-mono text-2xs text-text">
            {shortcut}
          </kbd>
        </p>
      </div>
    </WizardShell>
  );
};

FirstDictationStep.displayName = "FirstDictationStep";
