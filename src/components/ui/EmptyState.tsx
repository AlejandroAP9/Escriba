import React from "react";
import { Loader2, type LucideIcon } from "lucide-react";
import { Plumin, type PluminPose } from "../shared/Plumin";

/**
 * Estados reutilizables (Escriba Design Guide, cap. 09). Nunca una lista vacía
 * sin mensaje: un icono discreto, un título cálido y, si aplica, una acción.
 * `compact` para usarlo dentro de tarjetas pequeñas (feeds, paneles).
 */
interface EmptyStateProps {
  icon?: LucideIcon;
  /** Plumín acompaña el estado vacío (tiene prioridad sobre el icono). */
  plumin?: PluminPose;
  title: string;
  description?: string;
  action?: React.ReactNode;
  compact?: boolean;
  className?: string;
}

export const EmptyState: React.FC<EmptyStateProps> = ({
  icon: Icon,
  plumin,
  title,
  description,
  action,
  compact = false,
  className = "",
}) => (
  <div
    className={`flex flex-col items-center justify-center text-center ${
      compact ? "gap-1.5 py-6" : "gap-2 py-10"
    } ${className}`}
  >
    {plumin && <Plumin pose={plumin} size={compact ? 104 : 136} />}
    {!plumin && Icon && (
      <span
        className={`flex items-center justify-center rounded-card bg-vitela/50 text-mid-gray ${
          compact ? "h-9 w-9" : "h-12 w-12"
        }`}
      >
        <Icon
          width={compact ? 16 : 20}
          height={compact ? 16 : 20}
          strokeWidth={1.6}
        />
      </span>
    )}
    <p className="text-sm font-medium text-text">{title}</p>
    {description && (
      <p className="max-w-xs text-xs leading-relaxed text-mid-gray">
        {description}
      </p>
    )}
    {action && <div className="mt-1">{action}</div>}
  </div>
);

/** Espera discreta: un spinner suave, nunca una pantalla en blanco. */
export const LoadingState: React.FC<{ label: string; compact?: boolean }> = ({
  label,
  compact = false,
}) => (
  <div
    className={`flex items-center justify-center gap-2 text-mid-gray ${
      compact ? "py-6" : "py-10"
    }`}
  >
    <Loader2 width={16} height={16} className="animate-spin" />
    <span className="text-sm">{label}</span>
  </div>
);
