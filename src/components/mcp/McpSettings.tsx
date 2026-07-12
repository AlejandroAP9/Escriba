import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Check, Copy } from "lucide-react";
import { commands, type McpStatus } from "@/bindings";
import { Button } from "../ui/Button";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

/**
 * Sección "Agentes (MCP)": levanta un servidor MCP local para que agentes de IA
 * (Claude Code, Cursor, Cline) usen las capacidades de Escriba —transcribir,
 * traducir, resumir— como tools. Escucha solo en 127.0.0.1: nada sale del equipo.
 */
export const McpSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [status, setStatus] = useState<McpStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);

  const refresh = useCallback(async () => {
    setStatus(await commands.mcpStatus());
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const toggle = async () => {
    setBusy(true);
    try {
      if (status?.running) {
        await commands.mcpStop();
      } else {
        const result = await commands.mcpStart();
        if (result.status === "error") {
          toast.error(t("mcp.startError"), { description: result.error });
        }
      }
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const connectCmd = status?.url
    ? `claude mcp add escriba --transport http --url ${status.url}`
    : "";

  const copyCmd = async () => {
    if (!connectCmd) return;
    try {
      await navigator.clipboard.writeText(connectCmd);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error(t("mcp.copyError"));
    }
  };

  const TOOLS = [
    "transcribe",
    "translate",
    "summarize",
    "polish",
    "list_dictations",
    "usage_stats",
  ] as const;

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <div className="rounded-lg border border-mid-gray/30 p-5 space-y-4">
        <div>
          <h3 className="font-semibold text-text">{t("mcp.title")}</h3>
          <p className="text-sm text-mid-gray mt-1">{t("mcp.subtitle")}</p>
        </div>

        <div className="flex items-center justify-between">
          <span
            className={`text-sm font-medium ${
              status?.running ? "text-logo-primary" : "text-mid-gray"
            }`}
          >
            {status?.running ? t("mcp.running") : t("mcp.stopped")}
          </span>
          <Button
            variant={status?.running ? "secondary" : "primary"}
            size="md"
            onClick={toggle}
            disabled={busy}
          >
            {status?.running ? t("mcp.stop") : t("mcp.start")}
          </Button>
        </div>

        <ToggleSwitch
          checked={getSetting("mcp_autostart") || false}
          onChange={(value) => {
            updateSetting("mcp_autostart", value);
            // Al activarlo, el backend levanta el servidor: refresca el estado.
            if (value) window.setTimeout(refresh, 600);
          }}
          isUpdating={isUpdating("mcp_autostart")}
          label={t("mcp.autostart.label")}
          description={t("mcp.autostart.description")}
          descriptionMode="tooltip"
        />

        {status?.running && status.url && (
          <div className="space-y-2">
            <p className="text-xs text-mid-gray">{t("mcp.connectHint")}</p>
            <div className="flex items-center gap-2 rounded-lg border border-mid-gray/30 bg-background p-2">
              <code className="flex-1 text-xs text-text break-all font-mono">
                {connectCmd}
              </code>
              <button
                type="button"
                onClick={copyCmd}
                title={t("mcp.copy")}
                className="shrink-0 p-1.5 rounded-md text-mid-gray hover:text-text"
              >
                {copied ? (
                  <Check width={16} height={16} />
                ) : (
                  <Copy width={16} height={16} />
                )}
              </button>
            </div>
          </div>
        )}

        <div className="rounded-lg bg-logo-primary/5 border border-logo-primary/20 p-3">
          <p className="text-xs font-medium text-text mb-2">
            {t("mcp.toolsTitle")}
          </p>
          <ul className="space-y-1">
            {TOOLS.map((tool) => (
              <li key={tool} className="text-xs text-mid-gray">
                <span className="font-mono text-text">{tool}</span>
                {" — "}
                {t(`mcp.tool.${tool}`)}
              </li>
            ))}
          </ul>
        </div>

        <p className="text-xs text-mid-gray italic">{t("mcp.localNote")}</p>
      </div>
    </div>
  );
};
