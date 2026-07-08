import React from "react";

const BRAND = "Escriba";
const SERIF = "Georgia, 'Times New Roman', serif";

// Wordmark: pluma que escribe una onda de voz en tinta, terminando en punto,
// con "Escriba" en serif debajo. Versión vectorial del logo de Flor
// (brand/logo-escriba-flor). Usa currentColor para tema claro/oscuro.
const EscribaLogo = ({
  width,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  return (
    <svg
      width={width}
      height={height}
      className={className}
      viewBox="0 0 360 184"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* Pluma: silueta con espina central, la punta toca el inicio de la onda */}
      <path
        d="M 56 96
           C 64 66, 88 32, 128 10
           C 134 40, 122 72, 92 90
           C 80 97, 66 99, 56 96 Z"
        fill="currentColor"
        opacity="0.92"
      />
      <path
        d="M 58 94 C 76 66, 100 38, 126 14"
        stroke="currentColor"
        strokeWidth="2.5"
        strokeLinecap="round"
        opacity="0.35"
      />
      {/* Trazo de tinta: sale de la punta de la pluma y se vuelve onda de voz */}
      <path
        d="M 56 96
           C 84 106, 116 106, 146 99
           L 156 80 L 166 108 L 176 70 L 186 114 L 196 74 L 206 110 L 216 86 L 226 102
           C 258 108, 298 106, 324 99"
        stroke="currentColor"
        strokeWidth="6.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <ellipse
        cx="341"
        cy="100"
        rx="7"
        ry="4.2"
        fill="currentColor"
        transform="rotate(-10 341 100)"
      />
      <text
        x="180"
        y="168"
        textAnchor="middle"
        fill="currentColor"
        fontFamily={SERIF}
        fontSize="52"
        fontWeight="600"
        letterSpacing="1"
      >
        {BRAND}
      </text>
    </svg>
  );
};

export default EscribaLogo;
