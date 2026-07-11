import React from "react";

const BRAND = "Escriba";
const SERIF = "Georgia, 'Times New Roman', serif";

// Wordmark: pluma de line-art (contorno + espina + barbas) que escribe una onda
// de voz en tinta, terminando en punto, con "Escriba" en serif debajo.
// Monocromo via currentColor: se adapta a tema claro/oscuro y se ve nitido a
// cualquier tamano (antes era una silueta rellena que a tamano chico se veia
// como una mancha).
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
      viewBox="0 0 380 250"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      {/* Pluma: contorno */}
      <path
        d="M300 18 C 244 36, 194 78, 164 144 C 209 133, 249 102, 273 58 C 285 44, 294 31, 300 18 Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="4.5"
        strokeLinejoin="round"
      />
      {/* Espina central */}
      <path
        d="M300 18 C 244 62, 198 104, 140 150"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
      />
      {/* Barbas */}
      <g stroke="currentColor" strokeWidth="2.6" strokeLinecap="round">
        <path d="M266 56 L 247 42" />
        <path d="M245 80 L 223 66" />
        <path d="M224 102 L 200 90" />
        <path d="M203 124 L 178 114" />
      </g>
      {/* Canon hasta la punta (nib) */}
      <path
        d="M140 150 L 116 170"
        stroke="currentColor"
        strokeWidth="4.5"
        strokeLinecap="round"
      />
      <path d="M116 170 l -9 12 l 13 -3 Z" fill="currentColor" />
      {/* Trazo de tinta que se vuelve onda de voz, termina en punto */}
      <path
        d="M122 168 C 164 178, 204 175, 230 167 L 240 150 L 250 178 L 260 141 L 270 182 L 280 147 L 290 174 L 300 160 L 312 170 C 336 176, 352 173, 362 168"
        stroke="currentColor"
        strokeWidth="5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <ellipse cx="374" cy="168" rx="5" ry="3" fill="currentColor" />
      <text
        x="190"
        y="238"
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
