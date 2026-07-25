import React from "react";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?:
    | "primary"
    | "primary-soft"
    | "secondary"
    | "danger"
    | "danger-ghost"
    | "ghost";
  size?: "sm" | "md" | "lg";
}

export const Button: React.FC<ButtonProps> = ({
  children,
  className = "",
  variant = "primary",
  size = "md",
  ...props
}) => {
  // Motion: transición corta (150 ms) que cubre color, brillo y transform, para
  // que hover y "pressed" (active) se sientan táctiles (Design Guide, cap. 09/11).
  // `focus:outline-none` va acompañado SIEMPRE de un anillo visible. Antes
  // estaba solo, y como esta clase vive en las clases base, anulaba el anillo
  // dorado global de App.css en todos los botones de la app: la variante
  // `ghost` se quedaba sin ninguna señal de foco.
  const baseClasses =
    "font-medium rounded-lg border focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary transition duration-150 disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer";

  const variantClasses = {
    primary:
      "gold-surface text-ink border-background-ui shadow-[0_1px_2px_rgba(27,20,38,0.18),inset_0_1px_0_rgba(255,255,255,0.25)] hover:brightness-105 active:brightness-95 focus:ring-1 focus:ring-background-ui",
    "primary-soft":
      "text-text bg-logo-primary/20 border-transparent hover:bg-logo-primary/30 active:bg-logo-primary/40 focus:ring-1 focus:ring-logo-primary",
    secondary:
      "bg-mid-gray/10 border-mid-gray/20 hover:bg-background-ui/30 hover:border-logo-primary active:bg-background-ui/40 focus:outline-none focus-visible:ring-1 focus-visible:ring-logo-primary",
    danger:
      "text-white bg-lacre border-lacre/30 hover:bg-lacre/90 active:bg-lacre/80 focus:ring-1 focus:ring-lacre",
    "danger-ghost":
      "text-lacre border-transparent hover:bg-lacre/10 active:bg-lacre/20 focus:ring-1 focus:ring-lacre",
    ghost:
      "text-current border-transparent hover:bg-mid-gray/10 hover:border-logo-primary active:bg-mid-gray/20 focus:bg-mid-gray/20",
  };

  const sizeClasses = {
    sm: "px-2 py-1 text-xs",
    md: "px-4 py-1.5 text-sm",
    lg: "px-4 py-2 text-base",
  };

  return (
    <button
      className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
      {...props}
    >
      {children}
    </button>
  );
};
