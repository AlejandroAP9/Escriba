import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Download } from "lucide-react";
import { commands } from "@/bindings";
import { Button } from "../ui/Button";
import { navigateTo } from "@/lib/navigation";

/**
 * Aviso reutilizable para las features que dependen del motor de IA local
 * (Traductor, Intérprete, Sesiones, resumen del Estudio). Sin el motor esas
 * pantallas fallaban EN SILENCIO para todo usuario nuevo (hallazgo de Flor,
 * 15-jul): el usuario merece saber qué falta y dónde conseguirlo.
 * Se oculta solo cuando el motor (runtime + modelo) está instalado.
 */
export const EngineRequiredCard: React.FC<{ className?: string }> = ({
  className = "",
}) => {
  const { t } = useTranslation();
  // null = aún no sabemos (no mostrar nada para evitar parpadeo).
  const [ready, setReady] = useState<boolean | null>(null);

  useEffect(() => {
    let alive = true;
    const check = () =>
      commands
        .getLocalLlmStatus()
        .then((s) => {
          if (alive) setReady(s.runtime_installed && s.model_installed);
        })
        .catch(() => {});
    check();
    // Si el usuario instala el motor en Post Proceso y vuelve, la tarjeta
    // desaparece sola.
    const id = window.setInterval(check, 5000);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, []);

  if (ready !== false) return null;

  return (
    <div
      className={`rounded-card border border-logo-primary/25 bg-logo-primary/5 p-4 ${className}`}
    >
      <div className="flex flex-wrap items-start gap-3">
        <Download
          width={18}
          height={18}
          className="mt-0.5 shrink-0 text-logo-primary"
        />
        <div className="min-w-[200px] flex-1">
          <p className="text-sm font-semibold text-text">
            {t("engine.requiredTitle")}
          </p>
          <p className="mt-1 text-xs leading-relaxed text-mid-gray">
            {t("engine.requiredBody")}
          </p>
        </div>
        <Button
          variant="primary-soft"
          size="sm"
          onClick={() => navigateTo("postprocessing")}
        >
          {t("engine.requiredCta")}
        </Button>
      </div>
    </div>
  );
};
