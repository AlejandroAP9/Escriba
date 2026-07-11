import React from "react";
import markInk from "../../assets/escriba-mark-ink.png";
import markParchment from "../../assets/escriba-mark-parchment.png";

const BRAND = "Escriba";
const SERIF = "Georgia, 'Times New Roman', serif";

// Logo de Escriba (diseno de Flor): pluma + onda de voz + firma, con "Escriba"
// en serif debajo. La marca viene en dos versiones (tinta y pergamino) que se
// intercambian segun el tema del sistema, asi se ve bien en claro y oscuro.
// Las clases escriba-mark--light/--dark viven en App.css.
const EscribaLogo = ({
  width = 120,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  const imgStyle = { width, height: height ?? "auto" } as const;
  return (
    <div
      className={className}
      style={{
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        gap: Math.round(width * 0.06),
      }}
    >
      <img
        className="escriba-mark--light"
        src={markInk}
        alt=""
        style={imgStyle}
      />
      <img
        className="escriba-mark--dark"
        src={markParchment}
        alt=""
        style={imgStyle}
      />
      <span
        className="text-text"
        style={{
          fontFamily: SERIF,
          fontWeight: 600,
          fontSize: Math.round(width * 0.3),
          letterSpacing: 1,
          lineHeight: 1,
        }}
      >
        {BRAND}
      </span>
    </div>
  );
};

export default EscribaLogo;
