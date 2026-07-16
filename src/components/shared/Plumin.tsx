import React from "react";
import neutral from "@/assets/plumin/plumin-neutral.png";
import escucha from "@/assets/plumin/plumin-escucha.png";
import escribe from "@/assets/plumin/plumin-escribe.png";
import celebra from "@/assets/plumin/plumin-celebra.png";
import disculpa from "@/assets/plumin/plumin-disculpa.png";
import guia from "@/assets/plumin/plumin-guia.png";

/**
 * Plumín, el aprendiz de escriba (diseño de la dupla, bautizado por el jurado
 * infantil de QA). Aparece solo donde aporta: onboarding, estados vacíos y
 * celebraciones. Nunca flota sobre el trabajo del usuario: es guía, no Clippy.
 * Los PNG traen fondo pergamino (#fbf6ea): en tema claro se funden con la app
 * y en oscuro el borde redondeado los convierte en una estampa de manuscrito.
 */

export type PluminPose =
  | "neutral"
  | "escucha"
  | "escribe"
  | "celebra"
  | "disculpa"
  | "guia";

const POSES: Record<PluminPose, string> = {
  neutral,
  escucha,
  escribe,
  celebra,
  disculpa,
  guia,
};

export const Plumin: React.FC<{
  pose?: PluminPose;
  size?: number;
  className?: string;
}> = ({ pose = "neutral", size = 112, className = "" }) => (
  <img
    src={POSES[pose]}
    alt=""
    aria-hidden="true"
    width={size}
    height={size}
    className={`select-none rounded-2xl ${className}`}
    draggable={false}
  />
);
