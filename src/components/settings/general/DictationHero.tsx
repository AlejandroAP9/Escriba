import React from "react";
import { useTranslation } from "react-i18next";
import { Mic, ArrowRight, ClipboardPaste } from "lucide-react";
import markInk from "../../../assets/escriba-mark-ink.png";
import markParchment from "../../../assets/escriba-mark-parchment.png";

// Apps donde el dictado aparece directo (proper nouns, no traducibles).
const APPS = ["Cursor", "Claude", "WhatsApp", "Terminal"];

/**
 * Cuadro de bienvenida arriba de Ajustes → General: recuerda de un vistazo qué
 * hace Escriba y sus tres promesas (local, sin nube, gratis). Debajo sigue el
 * formato normal de ajustes.
 */
export const DictationHero: React.FC = () => {
  const { t } = useTranslation();
  const badges = [
    t("settings.general.hero.local"),
    t("settings.general.hero.noCloud"),
    t("settings.general.hero.free"),
  ];

  return (
    <div
      className="relative overflow-hidden rounded-xl border border-logo-primary/25 shadow-[0_1px_2px_rgba(27,20,38,0.04),0_18px_40px_-22px_rgba(27,20,38,0.20)] p-5 sm:p-6"
      style={{
        background:
          "radial-gradient(130% 90% at 0% 0%, rgba(255,255,255,0.45), transparent 55%), linear-gradient(135deg, var(--color-background), var(--color-vitela))",
      }}
    >
      {/* Textura de papel casi imperceptible (grano de pergamino). */}
      <div
        className="pointer-events-none absolute inset-0 opacity-[0.05]"
        style={{
          backgroundImage:
            "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='140' height='140'%3E%3Cfilter id='p'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='2' stitchTiles='stitch'/%3E%3CfeColorMatrix type='saturate' values='0'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23p)'/%3E%3C/svg%3E\")",
        }}
      />
      {/* Marca de agua: la pluma como firma del panel (muy tenue). */}
      <img
        src={markInk}
        aria-hidden="true"
        className="escriba-mark--light pointer-events-none absolute -bottom-10 right-2 w-56 select-none opacity-[0.05]"
      />
      <img
        src={markParchment}
        aria-hidden="true"
        className="escriba-mark--dark pointer-events-none absolute -bottom-10 right-2 w-56 select-none opacity-[0.06]"
      />
      <div className="relative">
        <div className="mb-3 flex items-center justify-between gap-2">
          <span className="inline-flex items-center gap-2 text-xs font-semibold uppercase tracking-wider text-logo-primary">
            <span className="h-2 w-2 animate-pulse rounded-full bg-logo-primary" />
            {t("settings.general.hero.ready")}
          </span>
          <span className="rounded-full border border-mid-gray/30 px-2.5 py-1 text-[10px] font-semibold uppercase tracking-wider text-mid-gray">
            {t("settings.general.hero.private")}
          </span>
        </div>

        <div className="flex items-start justify-between gap-4">
          <h2
            className="max-w-md text-xl font-medium leading-snug text-text sm:text-[1.6rem]"
            style={{
              fontFamily: "var(--font-serif)",
              letterSpacing: "0.01em",
            }}
          >
            {t("settings.general.hero.headline")}
          </h2>
          <div className="flex shrink-0 items-center gap-1.5 text-logo-primary">

            <span className="flex h-9 w-9 items-center justify-center rounded-lg border border-logo-primary/30 bg-logo-primary/10">
              <Mic width={18} height={18} />
            </span>
            <ArrowRight width={14} height={14} className="text-mid-gray" />
            <span className="flex h-9 w-9 items-center justify-center rounded-lg border border-logo-primary/30 bg-logo-primary/10">
              <ClipboardPaste width={18} height={18} />
            </span>
          </div>
        </div>

        {/* Firma: un trazo de tinta dorado que se dibuja bajo el titular. */}
        <div
          aria-hidden="true"
          className="ink-stroke mt-3 h-0.5 w-20 rounded-full"
          style={{
            background:
              "linear-gradient(to right, var(--color-logo-primary), transparent)",
          }}
        />

        <p className="mt-3 max-w-lg text-sm text-mid-gray">
          {t("settings.general.hero.description")}
        </p>

        <div className="mt-4 flex flex-wrap gap-2">
          {APPS.map((app) => (
            <span
              key={app}
              className="rounded-full border border-mid-gray/25 px-3 py-1 text-xs text-mid-gray"
            >
              {app}
            </span>
          ))}
        </div>

        <div className="mt-4 flex flex-wrap gap-x-4 gap-y-1">
          {badges.map((label, i) => (
            <span
              key={i}
              className="inline-flex items-center gap-1.5 text-[10px] font-mono uppercase tracking-wide text-mid-gray"
            >
              <span className="h-1.5 w-1.5 rounded-full bg-logo-primary/70" />
              {label}
            </span>
          ))}
        </div>
      </div>
    </div>
  );
};
