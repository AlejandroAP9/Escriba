import React from "react";
import { AlertCircle, AlertTriangle, Info, CheckCircle } from "lucide-react";

type AlertVariant = "error" | "warning" | "info" | "success";

interface AlertProps {
  variant?: AlertVariant;
  /** When true, removes rounded corners for use inside containers */
  contained?: boolean;
  children: React.ReactNode;
  className?: string;
}

const variantStyles: Record<
  AlertVariant,
  { container: string; icon: string; text: string }
> = {
  error: {
    container: "bg-lacre/10",
    icon: "text-lacre",
    text: "text-lacre",
  },
  // Estas tres variantes usaban la paleta cruda de Tailwind, que está calibrada
  // para fondo blanco: sobre pergamino, text-yellow-600 daba 2,72:1 y
  // text-blue-500 3,41:1. Los tokens semánticos pasan AA en los dos temas.
  warning: {
    container: "bg-warning/10",
    icon: "text-warning",
    text: "text-warning",
  },
  info: {
    container: "bg-info/10",
    icon: "text-info",
    text: "text-info",
  },
  success: {
    container: "bg-success/10",
    icon: "text-success",
    text: "text-success",
  },
};

const variantIcons: Record<AlertVariant, React.ElementType> = {
  error: AlertCircle,
  warning: AlertTriangle,
  info: Info,
  success: CheckCircle,
};

export const Alert: React.FC<AlertProps> = ({
  variant = "error",
  contained = false,
  children,
  className = "",
}) => {
  const styles = variantStyles[variant];
  const Icon = variantIcons[variant];

  return (
    <div
      className={`flex items-start gap-3 p-4 ${styles.container} ${contained ? "" : "rounded-lg"} ${className}`}
    >
      <Icon className={`w-5 h-5 shrink-0 mt-0.5 ${styles.icon}`} />
      <p className={`text-sm ${styles.text}`}>{children}</p>
    </div>
  );
};
