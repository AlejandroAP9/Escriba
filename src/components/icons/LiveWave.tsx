import React from "react";

/**
 * La onda de voz de la marca, viva: barras SVG que "respiran" en bucle suave.
 * Complementa la marca de Flor (cuya onda vive dentro del PNG y no puede
 * animarse): mismo lenguaje visual, en movimiento. Hereda el color del texto
 * (currentColor) y respeta prefers-reduced-motion (queda estática).
 */

// Alturas relativas de las barras (0-1), espejo del gesto de la marca:
// crece hacia el centro y cae con una cola corta.
const BARS = [0.28, 0.5, 0.75, 1, 0.68, 0.42, 0.6, 0.34, 0.22];

const LiveWave: React.FC<{
  width?: number;
  className?: string;
  /** Segundos del ciclo (más alto = más calmo). */
  period?: number;
}> = ({ width = 72, className, period = 2.2 }) => {
  const H = 24;
  const barW = 3.4;
  const gap = (width - BARS.length * barW) / (BARS.length - 1);
  return (
    <svg
      width={width}
      height={H}
      viewBox={`0 0 ${width} ${H}`}
      fill="none"
      aria-hidden="true"
      className={className}
    >
      {BARS.map((h, i) => {
        const barH = h * (H - 4);
        return (
          <rect
            key={i}
            className="livewave-bar"
            x={i * (barW + gap)}
            y={(H - barH) / 2}
            width={barW}
            height={barH}
            rx={barW / 2}
            fill="currentColor"
            style={{
              // Desfase por barra: la onda ondula en vez de latir en bloque.
              animationDelay: `${(i * period) / BARS.length / 2}s`,
              animationDuration: `${period}s`,
            }}
          />
        );
      })}
    </svg>
  );
};

export default LiveWave;
