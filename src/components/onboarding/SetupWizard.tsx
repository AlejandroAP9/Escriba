import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Check, Cloud, KeyRound, Lock } from "lucide-react";
import { commands } from "@/bindings";
import { useSettingsStore } from "../../stores/settingsStore";
import { WizardShell } from "./WizardShell";
import { FirstDictationStep } from "./FirstDictationStep";
import { LocalLlmSetup } from "../settings/PostProcessingSettingsApi/LocalLlmSetup";
import { ObsidianVault } from "../settings/ObsidianVault";
import { ShortcutInput } from "../settings/ShortcutInput";
import { Card } from "../ui/Card";
import Onboarding from "./Onboarding";

/**
 * Asistente de primera vez, narrado por Plumín.
 *
 * Cinco de los siete pasos NO traen componentes nuevos: reusan el mismo que
 * vive en Ajustes (modelo, motor local, atajo, Obsidian). Eso mantiene una
 * sola implementación de cada cosa — si el selector de modelos mejora, mejora
 * en los dos sitios — y deja el trabajo nuevo donde de verdad aporta: la
 * bienvenida y la primera dictada.
 *
 * El orden no es casual. El modelo de transcripción se empieza a descargar en
 * el paso 2 y sigue bajando mientras la persona configura motor, atajo y
 * vault: **el propio asistente es la barra de progreso**, y para cuando llega
 * a dictar, el modelo ya está.
 */

export type WizardStep =
  "welcome" | "model" | "engine" | "shortcut" | "obsidian" | "test";

const ORDER: WizardStep[] = [
  "welcome",
  "model",
  "engine",
  "shortcut",
  "obsidian",
  "test",
];

interface SetupWizardProps {
  /** Atajo de dictado ya formateado, para enseñarlo en el último paso. */
  shortcut: string;
  onComplete: () => void;
}

export const SetupWizard: React.FC<SetupWizardProps> = ({
  shortcut,
  onComplete,
}) => {
  const { t } = useTranslation();
  const [index, setIndex] = useState(0);
  const current = ORDER[index];
  const total = ORDER.length;
  const stepNumber = index + 1;

  const next = () => setIndex((i) => Math.min(ORDER.length - 1, i + 1));
  const back = () => setIndex((i) => Math.max(0, i - 1));

  const common = {
    step: stepNumber,
    totalSteps: total,
    onBack: index > 0 ? back : undefined,
  };

  if (current === "welcome") {
    const PROMISES = [
      { icon: Lock, key: "local" },
      { icon: Cloud, key: "noCloud" },
      { icon: KeyRound, key: "noAccount" },
    ] as const;

    return (
      <WizardShell
        {...common}
        pose="guia"
        title={t("wizard.welcome.title")}
        narration={t("wizard.welcome.narration")}
        onNext={next}
        nextLabel={t("wizard.welcome.cta")}
      >
        <div className="flex flex-col gap-2.5">
          {PROMISES.map(({ icon: Icon, key }) => (
            <Card key={key} variant="config" className="px-4 py-3">
              <div className="flex items-start gap-3">
                <span className="mt-0.5 rounded-full bg-logo-primary/15 p-2 text-gold-text">
                  <Icon width={16} height={16} />
                </span>
                <div className="min-w-0">
                  <p className="text-sm font-medium text-text">
                    {t(`wizard.welcome.${key}.title`)}
                  </p>
                  <p className="mt-0.5 text-xs leading-relaxed text-mid-gray">
                    {t(`wizard.welcome.${key}.body`)}
                  </p>
                </div>
              </div>
            </Card>
          ))}
        </div>

        {/* El argumento más concreto que tenemos, y el que la landing pone en
            el centro: lo mismo que otros cobran al mes, aquí no cuesta. Va con
            cifras y con nombre, no como eslogan. */}
        <div className="mt-3 rounded-card border border-line bg-vitela/25 px-4 py-3">
          <p className="text-xs font-medium text-mid-gray">
            {t("wizard.welcome.priceTitle")}
          </p>
          <div className="mt-2 flex flex-wrap items-baseline gap-x-5 gap-y-1.5 text-sm">
            {/* Los tres nombres son marcas: no se traducen, y por eso van como
                datos y no como literales sueltos en el JSX. */}
            {[
              {
                name: "Typeless",
                price: t("wizard.welcome.priceTypeless"),
                ours: false,
              },
              {
                name: "Wispr Flow",
                price: t("wizard.welcome.priceWispr"),
                ours: false,
              },
              {
                name: "Escriba",
                price: t("wizard.welcome.priceEscriba"),
                ours: true,
              },
            ].map((row) => (
              <span
                key={row.name}
                className={
                  row.ours ? "font-medium text-gold-text" : "text-mid-gray"
                }
              >
                {row.name} <span className="tabular-nums">{row.price}</span>
              </span>
            ))}
          </div>
        </div>
      </WizardShell>
    );
  }

  if (current === "model") {
    return (
      <Onboarding
        onModelSelected={next}
        wizard={{
          step: stepNumber,
          totalSteps: total,
          onBack: back,
          onNext: next,
        }}
      />
    );
  }

  if (current === "engine") {
    return <EngineStep {...common} onNext={next} onSkip={next} />;
  }

  if (current === "shortcut") {
    return (
      <WizardShell
        {...common}
        pose="escucha"
        title={t("wizard.shortcut.title")}
        narration={t("wizard.shortcut.narration")}
        onNext={next}
        nextLabel={t("wizard.next")}
      >
        <div className="flex flex-col gap-2">
          <ShortcutInput
            shortcutId="transcribe"
            descriptionMode="inline"
            grouped
          />
          <ShortcutInput
            shortcutId="transcribe_with_post_process"
            descriptionMode="inline"
            grouped
          />
          <ShortcutInput
            shortcutId="voice_edit"
            descriptionMode="inline"
            grouped
          />

          {/* Lo que de verdad hay que llevarse de esta pantalla no es la
              combinación de teclas, sino que el texto aterriza en la app donde
              estés, sin copiar ni pegar. */}
          <p className="mt-1 text-xs leading-relaxed text-mid-gray">
            {t("wizard.shortcut.note")}
          </p>
        </div>
      </WizardShell>
    );
  }

  if (current === "obsidian") {
    return (
      <WizardShell
        {...common}
        pose="escribe"
        title={t("wizard.obsidian.title")}
        narration={t("wizard.obsidian.narration")}
        onNext={next}
        nextLabel={t("wizard.next")}
        onSkip={next}
        skipLabel={t("wizard.later")}
      >
        <ObsidianVault descriptionMode="inline" grouped />
      </WizardShell>
    );
  }

  return (
    <FirstDictationStep
      step={stepNumber}
      totalSteps={total}
      onBack={back}
      onDone={onComplete}
      shortcut={shortcut}
    />
  );
};

/**
 * Motor local (Qwen). Son 2,5 GB, así que se ofrece con salida clara en vez de
 * bloquear: quien lo salte puede instalarlo después en Ajustes.
 *
 * Al quedar listo, además de instalar, ENCIENDE el post-proceso y selecciona
 * el proveedor local. Sin eso —y es lo que pasa hoy en Ajustes— alguien
 * descarga 2,5 GB y no ocurre nada, porque el interruptor que lo activa vive
 * en otra pantalla.
 */
const EngineStep: React.FC<{
  step: number;
  totalSteps: number;
  onBack?: () => void;
  onNext: () => void;
  onSkip: () => void;
}> = ({ step, totalSteps, onBack, onNext, onSkip }) => {
  const { t } = useTranslation();
  const updateSetting = useSettingsStore((s) => s.updateSetting);
  const setProvider = useSettingsStore((s) => s.setPostProcessProvider);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    let cancelled = false;
    // El sondeo corre cada 2 s mientras esta pantalla esté abierta, así que la
    // activación tiene que ocurrir UNA vez, en el flanco de "aún no" a "listo".
    // Antes se reescribían proveedor y post-proceso en cada vuelta: decenas de
    // escrituras a los ajustes por minuto, y si el usuario apagaba el
    // post-proceso sin salir del paso, el siguiente tick se lo volvía a
    // encender (auditoría externa E-10, 30-jul-2026).
    let armed = false;
    const check = async () => {
      const s = await commands.getLocalLlmStatus();
      if (cancelled) return;
      const isReady = Boolean(s.runtime_installed && s.model_installed);
      setReady(isReady);
      if (isReady && !armed) {
        armed = true;
        // Dejarlo utilizable, no solo instalado.
        await setProvider("local_llm");
        await updateSetting("post_process_enabled", true);
      }
    };
    check();
    const id = window.setInterval(check, 2000);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [setProvider, updateSetting]);

  const UNLOCKS = ["polish", "translate", "sessions"] as const;

  return (
    <WizardShell
      step={step}
      totalSteps={totalSteps}
      pose="escribe"
      title={t("wizard.engine.title")}
      narration={t("wizard.engine.narration")}
      onBack={onBack}
      onNext={onNext}
      nextLabel={t("wizard.next")}
      onSkip={ready ? undefined : onSkip}
      skipLabel={t("wizard.later")}
    >
      <div className="flex flex-col gap-3">
        <Card variant="config" className="px-4 py-3">
          <p className="text-xs font-medium text-mid-gray">
            {t("wizard.engine.unlocksTitle")}
          </p>
          <ul className="mt-2 flex flex-col gap-1.5">
            {UNLOCKS.map((key) => (
              <li key={key} className="flex items-start gap-2 text-sm">
                <Check
                  width={14}
                  height={14}
                  className="mt-1 shrink-0 text-gold-text"
                />
                <span className="text-text">
                  {t(`wizard.engine.unlocks.${key}`)}
                </span>
              </li>
            ))}
          </ul>
        </Card>

        <LocalLlmSetup />

        <p className="text-xs text-mid-gray">{t("wizard.engine.sizeNote")}</p>
      </div>
    </WizardShell>
  );
};

SetupWizard.displayName = "SetupWizard";
