import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  AlignLeft,
  BarChart3,
  Check,
  Copy,
  History,
  Languages,
  Lock,
  Mic,
  Power,
  RotateCw,
  Sparkles,
  Terminal,
} from "lucide-react";
import { commands, type McpStatus } from "@/bindings";
import { Button } from "../ui/Button";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

/**
 * Sección "Agentes (MCP)": un panel de control del servidor MCP local, no un
 * formulario. Muestra estado en vivo (uptime, llamadas, actividad, agentes
 * conectados) con datos reales del backend, la configuración por cliente, las
 * tools expuestas y las garantías de privacidad. Escucha solo en 127.0.0.1.
 */

// Iconos y metadatos por tool (el orden es el del servidor).
const TOOLS = [
  { id: "transcribe", icon: Mic },
  { id: "translate", icon: Languages },
  { id: "summarize", icon: AlignLeft },
  { id: "polish", icon: Sparkles },
  { id: "list_dictations", icon: History },
  { id: "usage_stats", icon: BarChart3 },
] as const;

// Clientes soportados con su forma real de configuración.
type ClientId = "claude" | "cursor" | "vscode" | "other";
const CLIENTS: ClientId[] = ["claude", "cursor", "vscode", "other"];

function snippetFor(client: ClientId, url: string): string {
  switch (client) {
    case "claude":
      return `claude mcp add escriba --transport http --url ${url}`;
    case "cursor":
      return `{
  "mcpServers": {
    "escriba": {
      "url": "${url}"
    }
  }
}`;
    case "vscode":
      return `{
  "servers": {
    "escriba": {
      "type": "http",
      "url": "${url}"
    }
  }
}`;
    case "other":
      return url;
  }
}

// "3 h 18 min" / "12 min" / "45 s"
function formatDuration(secs: number): string {
  if (secs < 60) return `${secs} s`;
  if (secs < 3600) return `${Math.floor(secs / 60)} min`;
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  return m > 0 ? `${h} h ${m} min` : `${h} h`;
}

// "hace 5 s" style, sin plurales complejos.
function formatAgo(secs: number): string {
  if (secs < 60) return `${secs} s`;
  if (secs < 3600) return `${Math.floor(secs / 60)} min`;
  return `${Math.floor(secs / 3600)} h`;
}

export const McpSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const [status, setStatus] = useState<McpStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const [client, setClient] = useState<ClientId>("claude");

  const refresh = useCallback(async () => {
    setStatus(await commands.mcpStatus());
  }, []);

  // Estado inicial + sondeo en vivo mientras el servidor está activo, para que
  // la actividad y el uptime se sientan reales.
  useEffect(() => {
    refresh();
    const id = window.setInterval(refresh, 3500);
    return () => window.clearInterval(id);
  }, [refresh]);

  const running = !!status?.running;

  const start = async () => {
    setBusy(true);
    try {
      const result = await commands.mcpStart();
      if (result.status === "error") {
        toast.error(t("mcp.startError"), { description: result.error });
      }
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const stop = async () => {
    setBusy(true);
    try {
      await commands.mcpStop();
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const restart = async () => {
    setBusy(true);
    try {
      await commands.mcpStop();
      const result = await commands.mcpStart();
      if (result.status === "error") {
        toast.error(t("mcp.startError"), { description: result.error });
      }
      await refresh();
    } finally {
      setBusy(false);
    }
  };

  const snippet = useMemo(
    () => (status?.url ? snippetFor(client, status.url) : ""),
    [client, status?.url],
  );

  const copySnippet = async () => {
    if (!snippet) return;
    try {
      await navigator.clipboard.writeText(snippet);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      toast.error(t("mcp.copyError"));
    }
  };

  const address = status?.port ? `127.0.0.1:${status.port}` : "127.0.0.1:5151";
  const clientLabel = (id: ClientId) => t(`mcp.client.${id}`);

  return (
    <div className="mx-auto w-full max-w-4xl space-y-8 py-2">
      {/* Héroe: qué es y por qué importa. */}
      <div>
        <h1
          className="text-3xl leading-tight text-text sm:text-[2rem]"
          style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
        >
          {t("mcp.heroTitle")}
        </h1>
        <p className="mt-2 max-w-2xl text-sm leading-relaxed text-mid-gray">
          {t("mcp.heroSubtitle")}
        </p>
      </div>

      {/* Barra de estado del servidor + métricas en vivo. */}
      <div className="overflow-hidden rounded-card border border-line bg-background shadow-card">
        <div className="flex flex-wrap items-center justify-between gap-4 border-b border-line px-5 py-4">
          <div className="flex items-center gap-3">
            <span
              className={`relative flex h-2.5 w-2.5 ${running ? "" : "opacity-40"}`}
            >
              {running && (
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-500/60" />
              )}
              <span
                className={`relative inline-flex h-2.5 w-2.5 rounded-full ${
                  running ? "bg-green-600" : "bg-mid-gray"
                }`}
              />
            </span>
            <div>
              <p className="text-sm font-semibold text-text">
                {running ? t("mcp.running") : t("mcp.stopped")}
              </p>
              <p className="font-mono text-xs text-mid-gray">{address}</p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            {running ? (
              <>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={restart}
                  disabled={busy}
                >
                  <RotateCw width={14} height={14} className="mr-1.5" />
                  {t("mcp.restart")}
                </Button>
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={stop}
                  disabled={busy}
                >
                  <Power width={14} height={14} className="mr-1.5" />
                  {t("mcp.stop")}
                </Button>
              </>
            ) : (
              <Button
                variant="primary"
                size="md"
                onClick={start}
                disabled={busy}
              >
                <Power width={16} height={16} className="mr-2" />
                {t("mcp.start")}
              </Button>
            )}
          </div>
        </div>

        {running ? (
          <div className="grid grid-cols-2 divide-mid-gray/10 sm:grid-cols-4 sm:divide-x">
            {[
              { label: t("mcp.stat.uptime"), value: formatDuration(status!.uptime_seconds) },
              { label: t("mcp.stat.calls"), value: String(status!.total_calls) },
              { label: t("mcp.stat.tools"), value: String(TOOLS.length) },
              { label: t("mcp.stat.agents"), value: String(status!.clients.length) },
            ].map((s) => (
              <div key={s.label} className="px-5 py-4">
                <p
                  className="text-2xl text-text"
                  style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
                >
                  {s.value}
                </p>
                <p className="mt-0.5 font-mono text-[10px] uppercase tracking-[0.14em] text-mid-gray">
                  {s.label}
                </p>
              </div>
            ))}
          </div>
        ) : (
          <p className="px-5 py-4 text-sm text-mid-gray">{t("mcp.stoppedHint")}</p>
        )}
      </div>

      {/* Conectar: pestañas por cliente + snippet real. */}
      {running && status?.url && (
        <section className="space-y-3">
          <div>
            <h2 className="text-sm font-semibold text-text">
              {t("mcp.connectTitle")}
            </h2>
            <p className="text-xs text-mid-gray">{t("mcp.connectSubtitle")}</p>
          </div>

          <div className="flex flex-wrap gap-1.5">
            {CLIENTS.map((id) => (
              <button
                key={id}
                type="button"
                onClick={() => setClient(id)}
                className={`rounded-lg border px-3 py-1.5 text-xs font-medium transition-colors ${
                  client === id
                    ? "border-logo-primary/50 bg-logo-primary/10 text-text"
                    : "border-mid-gray/20 text-mid-gray hover:border-mid-gray/40 hover:text-text"
                }`}
              >
                {clientLabel(id)}
              </button>
            ))}
          </div>

          <div className="rounded-card border border-line bg-ink shadow-card">
            <div className="flex items-center justify-between border-b border-white/5 px-4 py-2">
              <span className="flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.14em] text-ink-muted">
                <Terminal width={12} height={12} />
                {t(`mcp.fileHint.${client}`)}
              </span>
              <button
                type="button"
                onClick={copySnippet}
                className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium text-ink-fg/80 transition-colors hover:text-ink-fg"
              >
                {copied ? (
                  <>
                    <Check width={13} height={13} className="text-green-400" />
                    {t("mcp.copied")}
                  </>
                ) : (
                  <>
                    <Copy width={13} height={13} />
                    {t("mcp.copyCmd")}
                  </>
                )}
              </button>
            </div>
            <pre className="overflow-x-auto px-4 py-3">
              <code className="font-mono text-xs leading-relaxed text-ink-fg">
                {snippet}
              </code>
            </pre>
          </div>
          <p className="font-mono text-[10px] text-mid-gray">
            {t("mcp.tokenNote")}
          </p>
        </section>
      )}

      {/* Herramientas como tarjetas. */}
      <section className="space-y-3">
        <h2 className="text-sm font-semibold text-text">
          {t("mcp.toolsTitle")}
        </h2>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {TOOLS.map(({ id, icon: Icon }) => (
            <div
              key={id}
              className="rounded-card border border-line bg-background p-4 shadow-card transition-transform hover:-translate-y-0.5"
            >
              <div className="flex items-center gap-2.5">
                <span className="flex h-8 w-8 items-center justify-center rounded-lg bg-logo-primary/10 text-logo-primary">
                  <Icon width={16} height={16} />
                </span>
                <div>
                  <p className="font-mono text-sm font-semibold text-text">
                    {id}
                  </p>
                  <p className="font-mono text-[10px] text-mid-gray">
                    {t(`mcp.toolIo.${id}`)}
                  </p>
                </div>
              </div>
              <p className="mt-2.5 text-xs leading-relaxed text-mid-gray">
                {t(`mcp.tool.${id}`)}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Actividad + agentes: dos columnas de "vida" del servidor. */}
      {running && (
        <div className="grid gap-4 lg:grid-cols-2">
          {/* Actividad reciente. */}
          <section className="rounded-card border border-line bg-background p-5 shadow-card">
            <h2 className="mb-3 text-sm font-semibold text-text">
              {t("mcp.activityTitle")}
            </h2>
            {status!.activity.length === 0 ? (
              <p className="text-xs leading-relaxed text-mid-gray">
                {t("mcp.activityEmpty")}
              </p>
            ) : (
              <ul className="space-y-2.5">
                {status!.activity.map((a, i) => (
                  <li
                    key={i}
                    className="flex items-center gap-3 text-xs"
                  >
                    <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-logo-primary" />
                    <span className="font-mono font-semibold text-text">
                      {a.tool}
                    </span>
                    <span className="ml-auto font-mono text-mid-gray">
                      {`${a.ms} ms`}
                    </span>
                    <span className="w-14 text-right font-mono text-mid-gray/70">
                      {formatAgo(a.seconds_ago)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </section>

          {/* Agentes conectados. */}
          <section className="rounded-card border border-line bg-background p-5 shadow-card">
            <h2 className="mb-3 text-sm font-semibold text-text">
              {t("mcp.agentsTitle")}
            </h2>
            {status!.clients.length === 0 ? (
              <p className="text-xs leading-relaxed text-mid-gray">
                {t("mcp.agentsEmpty")}
              </p>
            ) : (
              <ul className="space-y-2.5">
                {status!.clients.map((c, i) => (
                  <li key={i} className="flex items-center gap-2.5 text-sm">
                    <span className="h-2 w-2 shrink-0 rounded-full bg-green-600" />
                    <span className="font-medium text-text">{c.name}</span>
                    {c.version && (
                      <span className="font-mono text-[10px] text-mid-gray">
                        {`v${c.version}`}
                      </span>
                    )}
                    <span className="ml-auto font-mono text-[10px] text-mid-gray/70">
                      {formatAgo(c.seconds_ago)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      )}

      {/* Privacidad: la garantía que vende. */}
      <section className="rounded-card border border-logo-primary/20 bg-logo-primary/5 p-5">
        <h2 className="mb-3 flex items-center gap-2 text-sm font-semibold text-text">
          <Lock width={15} height={15} className="text-logo-primary" />
          {t("mcp.privacyTitle")}
        </h2>
        <ul className="grid gap-2 sm:grid-cols-2">
          {["localhost", "noInternet", "audioLocal", "noTelemetry"].map((k) => (
            <li key={k} className="flex items-start gap-2 text-xs text-text/80">
              <Check
                width={14}
                height={14}
                className="mt-0.5 shrink-0 text-logo-primary"
              />
              {t(`mcp.privacy.${k}`)}
            </li>
          ))}
        </ul>
      </section>

      {/* Preferencias al final. */}
      <section className="space-y-2">
        <h2 className="text-sm font-semibold text-text">
          {t("mcp.prefsTitle")}
        </h2>
        <div className="rounded-card border border-line bg-background px-4 py-1 shadow-card">
          <ToggleSwitch
            checked={getSetting("mcp_autostart") || false}
            onChange={(value) => {
              updateSetting("mcp_autostart", value);
              if (value) window.setTimeout(refresh, 600);
            }}
            isUpdating={isUpdating("mcp_autostart")}
            label={t("mcp.autostart.label")}
            description={t("mcp.autostart.description")}
            descriptionMode="tooltip"
          />
        </div>
      </section>
    </div>
  );
};
