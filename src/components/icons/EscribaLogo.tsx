import React from "react";
import markInk from "../../assets/escriba-mark-ink.png";
import markParchment from "../../assets/escriba-mark-parchment.png";

const BRAND = "Escriba";
const SERIF = "var(--font-serif)";

// Logo de Escriba (diseno de Flor): pluma + onda de voz + firma, con "Escriba"
// en serif debajo. La marca viene en dos versiones (tinta y pergamino) que se
// intercambian segun el tema del sistema, asi se ve bien en claro y oscuro.
// Las clases escriba-mark--light/--dark viven en App.css.
const EscribaLogo = ({
  width = 120,
  height,
  className,
  onDark = false,
}: {
  width?: number;
  height?: number;
  className?: string;
  // En superficies oscuras (barra lateral tinta) fija la marca pergamino y el
  // wordmark crema, sin depender del tema del sistema.
  onDark?: boolean;
}) => {
  // La marca de Flor es casi cuadrada; se muestra a ~2/3 del ancho nominal para
  // que no domine junto al wordmark.
  const imgStyle = {
    width: Math.round(width * 0.66),
    height: height ?? "auto",
  } as const;
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
      {onDark ? (
        <img src={markParchment} alt="" style={imgStyle} />
      ) : (
        <>
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
        </>
      )}
      <span
        className={onDark ? "text-ink-fg" : "text-text"}
        style={{
          fontFamily: SERIF,
          fontWeight: 600,
          fontSize: Math.round(width * 0.15),
          letterSpacing: "0.02em",
          lineHeight: 1,
          opacity: 0.92,
        }}
      >
        {BRAND}
      </span>
    </div>
  );
};

export default EscribaLogo;
