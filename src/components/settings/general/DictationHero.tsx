import React from "react";
import { useTranslation } from "react-i18next";
import { Mic, ArrowRight, ClipboardPaste } from "lucide-react";

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
    <div className="relative overflow-hidden rounded-xl border border-logo-primary/25 bg-logo-primary/5 p-5 sm:p-6">
      <div className="pointer-events-none absolute -right-10 -top-16 h-56 w-56 rounded-full bg-logo-primary/10 blur-3xl" />
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
            className="max-w-md text-2xl font-semibold leading-snug text-text sm:text-3xl"
            style={{ fontFamily: "Georgia, 'Times New Roman', serif" }}
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

        <p className="mt-2 max-w-lg text-sm text-mid-gray">
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
