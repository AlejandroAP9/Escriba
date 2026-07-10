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

  const items: { value: string; label: string; title?: string }[] = [
    {
      value: stats.total_words.toLocaleString(),
      label: t("historyStats.words"),
    },
    {
      value: `${stats.minutes_saved}`,
      label: t("historyStats.minutesSaved"),
      title: t("historyStats.minutesSavedHint"),
    },
    {
      value: `${stats.current_streak_days}`,
      label: t("historyStats.streak"),
    },
    {
      value: `${stats.active_days_last_30}/30`,
      label: t("historyStats.activeDays"),
    },
  ];

  return (
    <div className="grid grid-cols-4 gap-2 mb-4">
      {items.map((item) => (
        <div
          key={item.label}
          title={item.title}
          className="rounded-lg border border-mid-gray/30 p-3 text-center"
        >
          <div className="text-xl font-semibold text-text">{item.value}</div>
          <div className="text-xs text-mid-gray mt-1">{item.label}</div>
        </div>
      ))}
    </div>
  );
};
