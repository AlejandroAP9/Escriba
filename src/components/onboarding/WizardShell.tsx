import React from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft } from "lucide-react";
import { Plumin, type PluminPose } from "../shared/Plumin";
import { Button } from "../ui/Button";
import EscribaLogo from "../icons/EscribaLogo";

interface WizardShellProps {
  /** Paso actual, 1-indexado, solo para el indicador de progreso. */
  step: number;
  totalSteps: number;
  pose: PluminPose;
  title: string;
  /** Una frase de Plumín. Habla él, en primera persona. */
  narration: string;
  children: React.ReactNode;
  onBack?: () => void;
  onNext?: () => void;
  /** Texto del botón principal; sin él, el paso se avanza solo desde dentro. */
  nextLabel?: string;
  nextDisabled?: boolean;
  /** Salida secundaria ("Ahora no"), para pasos que no son obligatorios. */
  onSkip?: () => void;
  skipLabel?: string;
}

/**
 * Marco compartido del asistente de primera vez.
 *
 * Plumín no es un adorno en la esquina: es quien narra. Cada paso elige su pose
 * y habla en primera persona, así que la app se presenta como alguien que te
 * acompaña en vez de como un formulario con iconos. Es la diferencia entre
 * "Transcripción 100% local" y "yo escucho aquí mismo, tu voz no sale de tu
 * computador".
 *
 * El armazón solo pone el marco, la narración y la navegación; cada paso trae
 * su propio contenido, casi siempre reusando el componente que ya vive en
 * Ajustes.
 */
export const WizardShell: React.FC<WizardShellProps> = ({
  step,
  totalSteps,
  pose,
  title,
  narration,
  children,
  onBack,
  onNext,
  nextLabel,
  nextDisabled = false,
  onSkip,
  skipLabel,
}) => {
  const { t } = useTranslation();

  return (
    <div className="h-screen w-screen overflow-y-auto">
      <div className="mx-auto flex min-h-full max-w-2xl flex-col gap-5 px-6 py-8">
        {/* Cabecera: marca + progreso. El progreso es una fila de trazos, no
            un porcentaje: son pasos contables, no una descarga. */}
        <div className="flex items-center justify-between gap-4">
          {onBack ? (
            <button
              type="button"
              onClick={onBack}
              className="flex items-center gap-1.5 text-xs text-mid-gray transition-colors hover:text-text focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
            >
              <ArrowLeft width={14} height={14} />
              {t("common.back")}
            </button>
          ) : (
            <EscribaLogo width={116} className="text-text" />
          )}

          <div
            className="flex items-center gap-1"
            role="progressbar"
            aria-valuemin={1}
            aria-valuemax={totalSteps}
            aria-valuenow={step}
            aria-label={t("wizard.progress", { step, total: totalSteps })}
          >
            {Array.from({ length: totalSteps }, (_, i) => (
              <span
                key={i}
                className={`h-1 w-5 rounded-full transition-colors ${
                  i < step ? "bg-logo-primary" : "bg-mid-gray/25"
                }`}
              />
            ))}
          </div>
        </div>

        {/* Plumín narrando. El texto va en un bocadillo a su lado: es él quien
            habla, no una descripción impersonal del sistema. */}
        <div className="flex items-start gap-4">
          <Plumin pose={pose} size={88} className="shrink-0" />
          <div className="min-w-0 flex-1 pt-1">
            <h1
              className="text-2xl leading-tight text-text"
              style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
            >
              {title}
            </h1>
            <p className="mt-1.5 text-sm leading-relaxed text-mid-gray">
              {narration}
            </p>
          </div>
        </div>

        <div className="flex-1">{children}</div>

        {(onNext || onSkip) && (
          <div className="flex flex-wrap items-center justify-end gap-2 pt-1">
            {onSkip && (
              <Button variant="ghost" size="sm" onClick={onSkip}>
                {skipLabel ?? t("wizard.skip")}
              </Button>
            )}
            {onNext && (
              <Button
                variant="primary"
                onClick={onNext}
                disabled={nextDisabled}
              >
                {nextLabel ?? t("wizard.next")}
              </Button>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

WizardShell.displayName = "WizardShell";
