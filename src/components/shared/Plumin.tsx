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
 * Recortado a transparencia real: vive sobre cualquier fondo, y flota con una
 * respiración suave (quieta si el sistema pide reducir movimiento).
 */

export type PluminPose =
  "neutral" | "escucha" | "escribe" | "celebra" | "disculpa" | "guia";

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
  /** Altura en px; el ancho se ajusta solo (las poses no son cuadradas). */
  size?: number;
  className?: string;
  /** Flotación suave de reposo (off = quieto, p. ej. junto a texto denso). */
  animated?: boolean;
}> = ({ pose = "neutral", size = 120, className = "", animated = true }) => (
  <img
    src={POSES[pose]}
    alt=""
    aria-hidden="true"
    style={{ height: size, width: "auto" }}
    className={`select-none ${animated ? "plumin-float" : ""} ${className}`}
    draggable={false}
  />
);
