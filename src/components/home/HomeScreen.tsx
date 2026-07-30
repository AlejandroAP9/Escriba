import React, { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, Check } from "lucide-react";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { commands, type UsageStats } from "@/bindings";
import { navigateTo } from "@/lib/navigation";
import i18n from "@/i18n";
import { useSettings } from "../../hooks/useSettings";
import { Card } from "../ui/Card";
import LiveWave from "../icons/LiveWave";
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
  const [modelName, setModelName] = useState<string>("");

  useEffect(() => {
    commands.getUsageStats().then((r) => {
      if (r.status === "ok") setStats(r.data);
    });
    commands.getHistoryEntries(null, 1).then((r) => {
      if (r.status === "ok" && r.data.entries.length > 0) {
        setLastTs(r.data.entries[0].timestamp);
      }
    });
    // Nombre amigable del modelo actual (id -> name vía catálogo).
    Promise.all([commands.getCurrentModel(), commands.getAvailableModels()])
      .then(([cur, list]) => {
        if (cur.status === "ok" && list.status === "ok") {
          const m = list.data.find((x) => x.id === cur.data);
          if (m) setModelName(m.name);
        }
      })
      .catch(() => {});
  }, []);

  const bindings = getSetting("bindings") as
    Record<string, { current_binding?: string }> | undefined;
  const keys = useMemo(
    () => formatKeys(bindings?.transcribe?.current_binding),
    [bindings],
  );

  const savedMin = stats?.minutes_saved ?? 0;
  const savedValue = savedMin >= 60 ? Math.round(savedMin / 60) : savedMin;
  const savedUnit = savedMin >= 60 ? "h" : "min";

  const promises = [
    t("settings.general.hero.private"),
    t("settings.general.hero.local"),
    t("settings.general.hero.noCloud"),
    t("settings.general.hero.free"),
  ];

  // Palabras por día: el backend entrega 7 baldes (el último es hoy). Las
  // barras se escalan contra el máximo de la semana; 4px de mínimo visible
  // para que un día con pocas palabras no desaparezca.
  const wordsByDay = stats?.words_by_day ?? [];
  const weekTotal = wordsByDay.reduce((a, b) => a + b, 0);
  const weekMax = Math.max(...wordsByDay, 1);
  const dayFormatter = new Intl.DateTimeFormat(i18n.language, {
    weekday: "narrow",
  });
  const weekBars = wordsByDay.map((words, i) => {
    // Se retrocede por FECHA, no restando 24h fijas: en los dos días del año con
    // cambio de hora el día local no dura 86.400 s, y restar esa cantidad deja
    // la etiqueta un día corrida respecto al balde que el backend cubicó (que
    // también es por fecha local, ver `local_day` en history.rs).
    const date = new Date();
    date.setDate(date.getDate() - (wordsByDay.length - 1 - i));
    return {
      key: i,
      words,
      dayLabel: dayFormatter.format(date),
      isToday: i === wordsByDay.length - 1,
      heightPx:
        words === 0 ? 2 : Math.max(4, Math.round((words / weekMax) * 44)),
    };
  });

  const statCards = [
    { value: `${savedValue}`, unit: savedUnit, label: t("home.timeSaved") },
    {
      // Cuenta DICTADOS, no archivos: el agregado sobrevive a propósito al
      // recorte de grabaciones (es justo para lo que existe). La etiqueta decía
      // "Audios", que invita a contrastarlo con la carpeta de grabaciones y no
      // cuadra nunca en cuanto la retención borra un .wav.
      value: (stats?.total_transcriptions ?? 0).toLocaleString(),
      unit: "",
      label: t("home.audios"),
    },
    {
      value: (stats?.total_words ?? 0).toLocaleString(),
      unit: "",
      label: t("historyStats.words"),
    },
  ];

  // Estado REAL del sistema (antes eran checks decorativos siempre en verde):
  // modelo seleccionado, permiso de micrófono y de accesibilidad. Cada uno se
  // consulta de verdad; si algo falta, se marca y lleva a arreglarlo.
  const [micOk, setMicOk] = useState<boolean | null>(null);
  const [accOk, setAccOk] = useState<boolean | null>(null);
  useEffect(() => {
    if (platform() !== "macos") return;
    checkMicrophonePermission()
      .then(setMicOk)
      .catch(() => {});
    checkAccessibilityPermission()
      .then(setAccOk)
      .catch(() => {});
  }, []);

  const statusItems: {
    key: string;
    label: string;
    ok: boolean | null;
    goTo?: () => void;
  }[] = [
    {
      key: "model",
      label: modelName || t("home.status.modelMissing"),
      ok: modelName ? true : false,
      goTo: modelName ? undefined : () => navigateTo("models"),
    },
    ...(platform() === "macos"
      ? [
          {
            key: "mic",
            label:
              micOk === false
                ? t("home.status.micMissing")
                : t("home.micReady"),
            ok: micOk,
            goTo: micOk === false ? () => navigateTo("general") : undefined,
          },
          {
            key: "acc",
            label:
              accOk === false
                ? t("home.status.accessibilityMissing")
                : t("home.status.accessibility"),
            ok: accOk,
            goTo: accOk === false ? () => navigateTo("general") : undefined,
          },
        ]
      : []),
  ];

  return (
    <div className="relative mx-auto flex min-h-full max-w-5xl items-center px-6 py-12">
      {/* Marca de agua: la pluma, enorme y tenue, como sello del panel. */}
      <img
        src={markInk}
        aria-hidden="true"
        className="escriba-mark--light pointer-events-none absolute right-0 top-6 w-80 select-none opacity-[0.035]"
      />
      <img
        src={markParchment}
        aria-hidden="true"
        className="escriba-mark--dark pointer-events-none absolute right-0 top-6 w-80 select-none opacity-[0.05]"
      />

      <div className="relative grid w-full gap-8 lg:grid-cols-[1.45fr_1fr] lg:items-center">
        {/* Portada editorial */}
        <div>
          <span className="inline-flex items-center gap-2 text-xs font-semibold uppercase tracking-[0.14em] text-gold-text">
            <span className="h-2 w-2 animate-pulse rounded-full bg-logo-primary" />
            {t("settings.general.hero.ready")}
          </span>

          <h1
            className="mt-5 text-5xl leading-[1.05] text-text sm:text-6xl"
            style={{
              fontFamily: SERIF,
              fontWeight: 600,
              letterSpacing: "0.005em",
            }}
          >
            {t("home.headline1")}
            <br />
            <span className="text-mid-gray">{t("home.headline2")}</span>
          </h1>

          {/* La onda de la marca, viva: reemplaza al divisor estático. */}
          <LiveWave width={110} className="mt-5 text-gold-text" />

          <div className="mt-6 flex flex-wrap gap-x-5 gap-y-2 font-mono text-2xs uppercase tracking-[0.08em] text-mid-gray">
            {promises.map((p) => (
              <span key={p} className="inline-flex items-center gap-2">
                <span className="h-1.5 w-1.5 rounded-full bg-logo-primary/80" />
                {p}
              </span>
            ))}
          </div>

          {/* CTA: enseña el atajo como acción principal (casi un botón). */}
          <div className="mt-9 inline-flex items-center gap-3 rounded-card border border-logo-primary/40 bg-logo-primary/5 px-5 py-3.5 shadow-[0_1px_2px_rgba(27,20,38,0.05),inset_0_1px_0_rgba(255,255,255,0.35)]">
            {keys.length > 0 ? (
              <span className="flex items-center gap-1.5">
                {keys.map((k, i) => (
                  <kbd
                    key={i}
                    className="min-w-8 rounded-lg border border-mid-gray/25 bg-background px-2.5 py-1.5 text-center text-sm font-semibold text-text shadow-[0_1px_2px_rgba(27,20,38,0.06),inset_0_1px_0_rgba(255,255,255,0.4)]"
                  >
                    {k}
                  </kbd>
                ))}
              </span>
            ) : (
              <span className="text-sm text-mid-gray">
                {t("home.setShortcut")}
              </span>
            )}
            <span className="text-sm font-semibold text-text">
              {t("home.start")}
            </span>
          </div>
        </div>

        {/* Panel de estado + actividad (equilibra la composición). */}
        <aside className="space-y-3">
          <div className="rounded-card border border-line bg-background p-5 shadow-lift">
            <p className="font-mono text-3xs font-semibold uppercase tracking-[0.14em] text-mid-gray">
              {t("home.systemStatus")}
            </p>
            <ul className="mt-3 space-y-2.5">
              {statusItems.map((item) => (
                <li
                  key={item.key}
                  className="flex items-center gap-2.5 text-sm"
                >
                  {item.ok === false ? (
                    <AlertTriangle
                      width={15}
                      height={15}
                      className="shrink-0 text-lacre"
                    />
                  ) : (
                    <Check
                      width={15}
                      height={15}
                      className="shrink-0 text-gold-text"
                    />
                  )}
                  {item.goTo ? (
                    <button
                      type="button"
                      onClick={item.goTo}
                      className="truncate text-start text-lacre underline-offset-2 hover:underline focus:outline-none focus:underline"
                    >
                      {item.label}
                    </button>
                  ) : (
                    <span className="truncate text-text">{item.label}</span>
                  )}
                </li>
              ))}
            </ul>
            <div className="mt-4 border-t border-line pt-3 text-xs text-mid-gray">
              {t("home.lastDictation")}
              {" · "}
              <span className="text-text">
                {lastTs
                  ? relativeTime(lastTs, i18n.language)
                  : t("home.noneYet")}
              </span>
            </div>
          </div>

          <div className="grid grid-cols-3 gap-2">
            {statCards.map((s) => (
              <Card key={s.label} variant="metric" className="p-3">
                <div className="flex items-baseline justify-center gap-0.5">
                  <span
                    className="text-xl text-text"
                    style={{ fontFamily: SERIF, fontWeight: 600 }}
                  >
                    {s.value}
                  </span>
                  {s.unit && (
                    <span className="text-xs text-mid-gray">{s.unit}</span>
                  )}
                </div>
                <div className="mt-1 text-2xs leading-tight text-mid-gray">
                  {s.label}
                </div>
              </Card>
            ))}
          </div>

          {/* Palabras por día (últimos 7). Un solo color: la altura ya codifica
              el dato, así que la gráfica no depende del color y funciona igual
              en alto contraste y con daltonismo. Para un lector de pantalla es
              una imagen con resumen; el detalle por día va en el title. */}
          {weekTotal > 0 && (
            <Card variant="metric" className="p-3">
              <div className="mb-2 flex items-baseline justify-between">
                <span className="text-2xs leading-tight text-mid-gray">
                  {t("home.wordsByDay")}
                </span>
                <span className="text-2xs text-mid-gray">
                  {weekTotal.toLocaleString()}
                </span>
              </div>
              <div
                role="img"
                aria-label={t("home.wordsByDaySummary", {
                  count: weekTotal,
                })}
                className="flex items-end justify-between gap-1"
              >
                {weekBars.map((bar) => (
                  <div
                    key={bar.key}
                    title={`${bar.dayLabel}: ${bar.words.toLocaleString()}`}
                    className="flex flex-1 flex-col items-center gap-1"
                  >
                    <div
                      className={`w-full rounded-t-sm ${
                        bar.isToday ? "bg-logo-primary" : "bg-logo-primary/45"
                      }`}
                      style={{ height: `${bar.heightPx}px` }}
                    />
                    <span
                      aria-hidden="true"
                      className="text-3xs uppercase text-mid-gray"
                    >
                      {bar.dayLabel}
                    </span>
                  </div>
                ))}
              </div>
            </Card>
          )}
        </aside>
      </div>
    </div>
  );
};
