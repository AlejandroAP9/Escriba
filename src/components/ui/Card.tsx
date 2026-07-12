import React from "react";

/**
 * Primitiva de superficie única (Escriba Design Guide, cap. 05–06). Toda tarjeta
 * nace de aquí, así que radio, borde y sombra quedan consolidados en un solo
 * lugar. Tres variantes:
 *  - config:  ajustes y paneles (la superficie por defecto).
 *  - hero:    encabezado de identidad (degradado cálido, sombra elevada).
 *  - metric:  tile de KPI (centrado, para números grandes).
 */
export type CardVariant = "config" | "hero" | "metric";

interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: CardVariant;
}

const VARIANTS: Record<CardVariant, string> = {
  config: "rounded-card border border-line bg-background shadow-card",
  metric:
    "rounded-card border border-line bg-background shadow-card text-center",
  hero: "rounded-card border border-logo-primary/25 shadow-lift",
};

export const Card: React.FC<CardProps> = ({
  variant = "config",
  className = "",
  children,
  ...props
}) => (
  <div className={`${VARIANTS[variant]} ${className}`} {...props}>
    {children}
  </div>
);
