import React from "react";
import { useTranslation } from "react-i18next";
import ResetIcon from "../icons/ResetIcon";

interface ResetButtonProps {
  onClick: () => void;
  disabled?: boolean;
  className?: string;
  /**
   * Nombre accesible. Si se omite se usa uno genérico ("Restablecer al valor
   * por omisión"): 6 de los 7 usos no lo pasaban y quedaban como botones de
   * solo icono sin nombre. Vale la pena pasar uno específico cuando hay varios
   * en la misma pantalla, para distinguir qué restablece cada uno.
   */
  ariaLabel?: string;
  children?: React.ReactNode;
}

export const ResetButton: React.FC<ResetButtonProps> = React.memo(
  ({ onClick, disabled = false, className = "", ariaLabel, children }) => {
    const { t } = useTranslation();
    return (
      <button
        type="button"
        aria-label={ariaLabel ?? t("a11y.resetSetting")}
        className={`p-1 rounded-md border border-transparent transition-all duration-150 focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary ${
          disabled
            ? "opacity-50 cursor-not-allowed text-text/40"
            : "hover:bg-logo-primary/30 active:bg-logo-primary/50 active:translate-y-[1px] hover:cursor-pointer hover:border-logo-primary text-text/80"
        } ${className}`}
        onClick={onClick}
        disabled={disabled}
      >
        {children ?? <ResetIcon />}
      </button>
    );
  },
);

ResetButton.displayName = "ResetButton";
