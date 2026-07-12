import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type UsageStats } from "@/bindings";
import i18n from "@/i18n";
import { useSettings } from "../../hooks/useSettings";
import markInk from "../../assets/escriba-mark-ink.png";
import markParchment from "../../assets/escriba-mark-parchment.png";

// Atajo -> teclas legibles (símbolos de macOS + nombres cortos).
const KEY_SYMBOLS: Record<string, string> = {
  cmd: "⌘",
  command: "⌘",
  meta: "⌘",
  super: "⌘",
  win: "⌘",
  alt: "⌥",
  option: "⌥",
  ctrl: "⌃",
  control: "⌃",
  shift: "⇧",
  space: "Space",
  enter: "↵",
  return: "↵",
};

function formatKeys(binding: string | undefined): string[] {
  if (!binding) return [];
  return binding.split("+").map((raw) => {
    const k = raw.trim().toLowerCase();
    if (KEY_SYMBOLS[k]) return KEY_SYMBOLS[k];
    return raw.trim().length === 1
      ? raw.trim().toUpperCase()
      : raw.trim().charAt(0).toUpperCase() + raw.trim().slice(1);
  });
}

// Tiempo relativo localizado ("hace 4 minutos") sin cadenas propias por idioma.
function relativeTime(tsSeconds: number, locale: string): string {
  const diffSec = Math.round((tsSeconds * 1000 - Date.now()) / 1000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const abs = Math.abs(diffSec);
  if (abs < 60) return rtf.format(Math.round(diffSec), "second");
  if (abs < 3600) return rtf.format(Math.round(diffSec / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diffSec / 3600), "hour");
  return rtf.format(Math.round(diffSec / 86400), "day");
}

const SERIF = "var(--font-serif)";

export const HomeScreen: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const [stats, setStats] = useState<UsageStats | null>(null);
  const [lastTs, setLastTs] = useState<number | null>(null);

  useEffect(() => {
    commands.getUsageStats().then((r) => {
      if (r.status === "ok") setStats(r.data);
    });
    commands.getHistoryEntries(null, 1).then((r) => {
      if (r.status === "ok" && r.data.entries.length > 0) {
        setLastTs(r.data.entries[0].timestamp);
      }
    });
  }, []);

  const bindings = getSetting("bindings") as
    | Record<string, { current_binding?: string }>
    | undefined;
  const keys = useMemo(
    () => formatKeys(bindings?.transcribe?.current_binding),
    [bindings],
  );

  // Tiempo ahorrado: horas si supera la hora, si no minutos.
  const savedMin = stats?.minutes_saved ?? 0;
  const savedValue = savedMin >= 60 ? Math.round(savedMin / 60) : savedMin;
  const savedUnit = savedMin >= 60 ? "h" : "min";

  const promises = [
    t("settings.general.hero.private"),
    t("settings.general.hero.local"),
    t("settings.general.hero.noCloud"),
    t("settings.general.hero.free"),
  ];

  const statCards = [
    {
      value: `${savedValue}`,
      unit: savedUnit,
      label: t("home.timeSaved"),
    },
    {
      value: (stats?.total_words ?? 0).toLocaleString(),
      unit: "",
      label: t("historyStats.words"),
    },
    {
      value: `${stats?.current_streak_days ?? 0}`,
      unit: "",
      label: t("historyStats.streak"),
    },
  ];

  return (
    <div className="relative mx-auto flex min-h-full max-w-2xl flex-col justify-center px-6 py-14">
      {/* Marca de agua: la pluma, enorme y tenue, como sello del panel. */}
      <img
        src={markInk}
        aria-hidden="true"
        className="escriba-mark--light pointer-events-none absolute -right-10 top-4 w-72 select-none opacity-[0.04]"
      />
      <img
        src={markParchment}
        aria-hidden="true"
        className="escriba-mark--dark pointer-events-none absolute -right-10 top-4 w-72 select-none opacity-[0.05]"
      />

      <div className="relative">
        {/* Estado */}
        <span className="inline-flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.14em] text-logo-primary">
          <span className="h-2 w-2 animate-pulse rounded-full bg-logo-primary" />
          {t("settings.general.hero.ready")}
        </span>

        {/* Titular editorial */}
        <h1
          className="mt-5 text-5xl leading-[1.05] text-text sm:text-6xl"
          style={{ fontFamily: SERIF, fontWeight: 600, letterSpacing: "0.005em" }}
        >
          {t("home.headline1")}
          <br />
          <span className="text-mid-gray">{t("home.headline2")}</span>
        </h1>

        {/* Firma de tinta bajo el titular */}
        <div
          aria-hidden="true"
          className="ink-stroke mt-5 h-0.5 w-24 rounded-full"
          style={{
            background:
              "linear-gradient(to right, var(--color-logo-primary), transparent)",
          }}
        />

        {/* Promesas */}
        <div className="mt-6 flex flex-wrap gap-x-5 gap-y-2 font-mono text-[11px] uppercase tracking-[0.08em] text-mid-gray">
          {promises.map((p) => (
            <span key={p} className="inline-flex items-center gap-2">
              <span className="h-1.5 w-1.5 rounded-full bg-logo-primary/80" />
              {p}
            </span>
          ))}
        </div>

        {/* Inicio rápido */}
        <div className="mt-9 flex items-center gap-4">
          <div className="flex items-center gap-1.5">
            {keys.length > 0 ? (
              keys.map((k, i) => (
                <kbd
                  key={i}
                  className="min-w-8 rounded-lg border border-mid-gray/25 bg-background px-2.5 py-1.5 text-center text-sm font-medium text-text shadow-[0_1px_2px_rgba(27,20,38,0.06),inset_0_1px_0_rgba(255,255,255,0.4)]"
                >
                  {k}
                </kbd>
              ))
            ) : (
              <span className="text-sm text-mid-gray">{t("home.setShortcut")}</span>
            )}
          </div>
          <span className="text-sm text-mid-gray">{t("home.start")}</span>
        </div>

        {/* Métricas */}
        <div className="mt-10 grid grid-cols-2 gap-3 sm:grid-cols-3">
          {statCards.map((s) => (
            <div
              key={s.label}
              className="rounded-xl border border-mid-gray/15 bg-background p-4 shadow-[0_1px_2px_rgba(27,20,38,0.04),0_12px_28px_-18px_rgba(27,20,38,0.12)]"
            >
              <div className="flex items-baseline gap-1">
                <span
                  className="text-2xl text-text"
                  style={{ fontFamily: SERIF, fontWeight: 600 }}
                >
                  {s.value}
                </span>
                {s.unit && (
                  <span className="text-sm text-mid-gray">{s.unit}</span>
                )}
              </div>
              <div className="mt-1 text-xs text-mid-gray">{s.label}</div>
            </div>
          ))}
        </div>

        {/* Último dictado */}
        <div className="mt-4 text-xs text-mid-gray">
          {t("home.lastDictation")}
          {": "}
          <span className="text-text">
            {lastTs ? relativeTime(lastTs, i18n.language) : t("home.noneYet")}
          </span>
        </div>
      </div>
    </div>
  );
};
