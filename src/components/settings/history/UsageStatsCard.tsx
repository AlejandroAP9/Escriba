import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands, type UsageStats } from "@/bindings";

/**
 * Escriba: estadísticas derivadas del historial (sin telemetría, todo local).
 * El "tiempo ahorrado" asume 40 palabras/min tecleando vs ~200 dictando,
 * y el supuesto se muestra en el tooltip para ser honestos con el número.
 */
export const UsageStatsCard: React.FC = () => {
  const { t } = useTranslation();
  const [stats, setStats] = useState<UsageStats | null>(null);

  useEffect(() => {
    commands.getUsageStats().then((result) => {
      if (result.status === "ok") setStats(result.data);
    });
  }, []);

  if (!stats || stats.total_transcriptions === 0) return null;

  // Métricas con sentido: lo primero es el valor (tiempo ahorrado), no un dato
  // suelto como "55 palabras".
  const savedMin = stats.minutes_saved;
  const items: { value: string; unit?: string; label: string; title?: string }[] =
    [
      {
        value: savedMin >= 60 ? `${Math.round(savedMin / 60)}` : `${savedMin}`,
        unit: savedMin >= 60 ? "h" : "min",
        label: t("home.timeSaved"),
        title: t("historyStats.minutesSavedHint"),
      },
      {
        value: stats.total_transcriptions.toLocaleString(),
        label: t("home.audios"),
      },
      {
        value: stats.total_words.toLocaleString(),
        label: t("historyStats.words"),
      },
      {
        value: `${stats.current_streak_days}`,
        label: t("historyStats.streak"),
      },
    ];

  return (
    <div className="grid grid-cols-4 gap-2 mb-4">
      {items.map((item) => (
        <div
          key={item.label}
          title={item.title}
          className="rounded-xl border border-mid-gray/15 p-3 text-center shadow-[0_1px_2px_rgba(27,20,38,0.04)]"
        >
          <div className="flex items-baseline justify-center gap-0.5">
            <span
              className="text-xl text-text"
              style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
            >
              {item.value}
            </span>
            {item.unit && (
              <span className="text-xs text-mid-gray">{item.unit}</span>
            )}
          </div>
          <div className="text-xs text-mid-gray mt-1">{item.label}</div>
        </div>
      ))}
    </div>
  );
};
