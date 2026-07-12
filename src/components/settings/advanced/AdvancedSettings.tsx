import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { AlertTriangle, FolderOpen, Terminal } from "lucide-react";
import { commands, type McpStatus } from "@/bindings";
import { ShowOverlay } from "../ShowOverlay";
import { ModelUnloadTimeoutSetting } from "../ModelUnloadTimeout";
import { CustomWords } from "../CustomWords";
import { TextReplacements } from "../TextReplacements";
import { CollapsibleGroup } from "../../ui/CollapsibleGroup";
import { StartHidden } from "../StartHidden";
import { AutostartToggle } from "../AutostartToggle";
import { ShowTrayIcon } from "../ShowTrayIcon";
import { PasteMethodSetting } from "../PasteMethod";
import { TypingToolSetting } from "../TypingTool";
import { ClipboardHandlingSetting } from "../ClipboardHandling";
import { AutoSubmit } from "../AutoSubmit";
import { AppendTrailingSpace } from "../AppendTrailingSpace";
import { HistoryLimit } from "../HistoryLimit";
import { RecordingRetentionPeriodSelector } from "../RecordingRetentionPeriod";
import { ExperimentalToggle } from "../ExperimentalToggle";
import { useSettings } from "../../../hooks/useSettings";
import { useModelStore } from "../../../stores/modelStore";
import { KeyboardImplementationSelector } from "../debug/KeyboardImplementationSelector";
import { VoiceActivityDetection } from "../VoiceActivityDetection";
import { AccelerationSelector } from "../AccelerationSelector";
import { LazyStreamClose } from "../LazyStreamClose";

/**
 * Panel de estado interno (estilo Activity Monitor): muestra datos reales del
 * sistema —modelo cargado, servidor MCP, overlay, inicio automático,
 * experimental— para que el usuario avanzado vea de un vistazo cómo está
 * corriendo Escriba. La memoria ocupada no se muestra: no hay una fuente fiable.
 */
const SystemStatusPanel: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const { models } = useModelStore();
  const [modelLoaded, setModelLoaded] = useState(false);
  const [modelId, setModelId] = useState<string | null>(null);
  const [mcp, setMcp] = useState<McpStatus | null>(null);

  useEffect(() => {
    let alive = true;
    const load = async () => {
      const ls = await commands.getModelLoadStatus();
      if (alive && ls.status === "ok") {
        setModelLoaded(ls.data.is_loaded);
        setModelId(ls.data.current_model);
      }
      const m = await commands.mcpStatus();
      if (alive) setMcp(m);
    };
    load();
    const id = window.setInterval(load, 4000);
    return () => {
      alive = false;
      window.clearInterval(id);
    };
  }, []);

  const modelName =
    models.find((m) => m.id === modelId)?.name ?? modelId ?? "—";
  const overlayStyle = (getSetting("overlay_style") || "live") as string;
  const overlayLabel = t(
    `settings.advanced.overlay.style.options.${overlayStyle}`,
  );
  const autostart = getSetting("autostart_enabled") ?? false;
  const experimental = getSetting("experimental_enabled") ?? false;

  const rows: { label: string; value: string; ok: boolean }[] = [
    {
      label: t("settings.advanced.status.model"),
      value: `${modelName} · ${modelLoaded ? t("settings.advanced.status.active") : t("settings.advanced.status.inactive")}`,
      ok: modelLoaded,
    },
    {
      label: t("settings.advanced.status.mcp"),
      value: mcp?.running
        ? `${t("settings.advanced.status.running")} · ${mcp.port}`
        : t("settings.advanced.status.stopped"),
      ok: !!mcp?.running,
    },
    {
      label: t("settings.advanced.status.overlay"),
      value: overlayLabel,
      ok: overlayStyle !== "none",
    },
    {
      label: t("settings.advanced.status.autostart"),
      value: autostart
        ? t("settings.advanced.status.yes")
        : t("settings.advanced.status.no"),
      ok: autostart,
    },
    {
      label: t("settings.advanced.status.experimental"),
      value: experimental
        ? t("settings.advanced.status.on")
        : t("settings.advanced.status.off"),
      ok: experimental,
    },
  ];

  return (
    <div className="rounded-2xl border border-mid-gray/15 bg-background shadow-[0_1px_2px_rgba(27,20,38,0.04)]">
      <p className="border-b border-mid-gray/10 px-4 py-2.5 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-mid-gray">
        {t("settings.advanced.status.title")}
      </p>
      <dl className="grid grid-cols-1 sm:grid-cols-2">
        {rows.map((r, i) => (
          <div
            key={r.label}
            className={`flex items-center gap-2 px-4 py-2.5 ${
              i % 2 === 0 ? "sm:border-r sm:border-mid-gray/10" : ""
            } ${i >= 2 ? "border-t border-mid-gray/10" : ""}`}
          >
            <span
              className={`h-1.5 w-1.5 shrink-0 rounded-full ${
                r.ok ? "bg-green-600" : "bg-mid-gray/40"
              }`}
            />
            <dt className="text-xs text-mid-gray">{r.label}</dt>
            <dd className="ml-auto truncate font-mono text-xs text-text">
              {r.value}
            </dd>
          </div>
        ))}
      </dl>
    </div>
  );
};

/** Herramientas de usuario avanzado: versión + accesos a carpetas reales. */
const AdvancedTools: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");
  const [logPath, setLogPath] = useState("");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => {});
    commands.getLogDirPath().then((r) => {
      if (r.status === "ok") setLogPath(r.data);
    });
  }, []);

  return (
    <div className="rounded-2xl border border-mid-gray/15 bg-background p-4 shadow-[0_1px_2px_rgba(27,20,38,0.04)]">
      <p className="mb-3 flex items-center gap-2 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-mid-gray">
        <Terminal width={12} height={12} />
        {t("settings.advanced.tools.title")}
      </p>
      <div className="flex items-center justify-between py-1.5 text-xs">
        <span className="text-mid-gray">{t("settings.advanced.tools.version")}</span>
        <span className="font-mono text-text">{version || "—"}</span>
      </div>
      <div className="flex items-center justify-between gap-3 border-t border-mid-gray/10 py-2 text-xs">
        <span className="min-w-0 truncate font-mono text-mid-gray" title={logPath}>
          {logPath || t("settings.advanced.tools.logsFolder")}
        </span>
        <button
          type="button"
          onClick={() => commands.openLogDir()}
          className="flex shrink-0 items-center gap-1.5 rounded-lg border border-mid-gray/20 px-3 py-1.5 font-medium text-text transition-colors hover:border-mid-gray/40"
        >
          <FolderOpen width={13} height={13} />
          {t("settings.advanced.tools.logsFolder")}
        </button>
      </div>
      <div className="flex items-center justify-end border-t border-mid-gray/10 py-2">
        <button
          type="button"
          onClick={() => commands.openAppDataDir()}
          className="flex items-center gap-1.5 rounded-lg border border-mid-gray/20 px-3 py-1.5 text-xs font-medium text-text transition-colors hover:border-mid-gray/40"
        >
          <FolderOpen width={13} height={13} />
          {t("settings.advanced.tools.dataFolder")}
        </button>
      </div>
    </div>
  );
};

export const AdvancedSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const experimentalEnabled = getSetting("experimental_enabled") || false;

  return (
    <div className="mx-auto w-full max-w-3xl space-y-3 py-2">
      {/* Encabezado: marca claramente que entraste a una zona técnica. */}
      <div className="pb-1">
        <span className="font-mono text-[10px] font-semibold uppercase tracking-[0.2em] text-logo-primary">
          {t("settings.advanced.hero.badge")}
        </span>
        <h1
          className="mt-1 text-3xl leading-tight text-text"
          style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
        >
          {t("settings.advanced.hero.title")}
        </h1>
        <p className="mt-1.5 text-sm leading-relaxed text-mid-gray">
          {t("settings.advanced.hero.subtitle")}
        </p>
        <p className="mt-2 flex items-center gap-1.5 text-xs text-mid-gray/80">
          <AlertTriangle width={13} height={13} className="text-lacre" />
          {t("settings.advanced.hero.warning")}
        </p>
      </div>

      <SystemStatusPanel />

      <div className="space-y-3 pt-1">
        <CollapsibleGroup
          title={t("settings.advanced.groups.startup")}
          defaultOpen
        >
          <AutostartToggle descriptionMode="inline" grouped={true} />
          <StartHidden descriptionMode="inline" grouped={true} />
          <ShowTrayIcon descriptionMode="inline" grouped={true} />
        </CollapsibleGroup>

        <CollapsibleGroup title={t("settings.advanced.groups.interface")}>
          <ShowOverlay descriptionMode="inline" grouped={true} />
        </CollapsibleGroup>

        <CollapsibleGroup title={t("settings.advanced.groups.performance")}>
          <ModelUnloadTimeoutSetting descriptionMode="inline" grouped={true} />
        </CollapsibleGroup>

        {/* Funciones experimentales: destacadas con advertencia, no un switch más. */}
        <div className="rounded-2xl border border-lacre/25 bg-lacre/5 p-1.5">
          <p className="flex items-center gap-1.5 px-3 pt-2.5 text-xs font-medium text-lacre">
            <AlertTriangle width={13} height={13} />
            {t("settings.advanced.experimentalWarning")}
          </p>
          <ExperimentalToggle descriptionMode="inline" grouped={true} />
        </div>

        <CollapsibleGroup title={t("settings.advanced.groups.output")}>
          <PasteMethodSetting descriptionMode="inline" grouped={true} />
          <TypingToolSetting descriptionMode="inline" grouped={true} />
          <ClipboardHandlingSetting descriptionMode="inline" grouped={true} />
          <AutoSubmit descriptionMode="inline" grouped={true} />
        </CollapsibleGroup>

        <CollapsibleGroup title={t("settings.advanced.groups.transcription")}>
          <VoiceActivityDetection descriptionMode="inline" grouped={true} />
          <CustomWords descriptionMode="inline" grouped />
          <TextReplacements descriptionMode="inline" grouped />
          <AppendTrailingSpace descriptionMode="inline" grouped={true} />
        </CollapsibleGroup>

        <CollapsibleGroup title={t("settings.advanced.groups.history")}>
          <HistoryLimit descriptionMode="inline" grouped={true} />
          <RecordingRetentionPeriodSelector
            descriptionMode="inline"
            grouped={true}
          />
        </CollapsibleGroup>

        {experimentalEnabled && (
          <CollapsibleGroup title={t("settings.advanced.groups.experimental")}>
            <KeyboardImplementationSelector
              descriptionMode="inline"
              grouped={true}
            />
            <AccelerationSelector descriptionMode="inline" grouped={true} />
            <LazyStreamClose descriptionMode="inline" grouped={true} />
          </CollapsibleGroup>
        )}
      </div>

      <AdvancedTools />
    </div>
  );
};
