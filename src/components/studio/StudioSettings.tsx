import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { EngineRequiredCard } from "../shared/EngineRequiredCard";
import { listen } from "@tauri-apps/api/event";
import {
  open as openDialog,
  save as saveDialog,
} from "@tauri-apps/plugin-dialog";
import { Lock, Globe, Cpu, UploadCloud } from "lucide-react";
import { commands, type StudioJob } from "@/bindings";
import { requestObsidianExport } from "@/stores/obsidianStore";
import { Button } from "../ui/Button";
import { Alert } from "../ui/Alert";
import { AudioPlayer } from "../ui/AudioPlayer";
import { RetranscribeMenu } from "../shared/RetranscribeMenu";

const SUPPORTED_FORMATS = [
  "MP3",
  "WAV",
  "M4A",
  "MP4",
  "MOV",
  "FLAC",
  "OPUS",
  "OGG",
  "AAC",
  "AIFF",
  "CAF",
];

type Progress = {
  id: number;
  status: StudioJob["status"];
  progress: number;
  error: string | null;
};

const EXPORT_FORMATS = ["srt", "vtt", "txt", "json"] as const;

export const StudioSettings: React.FC = () => {
  const { t } = useTranslation();
  const [jobs, setJobs] = useState<StudioJob[]>([]);
  const [dragOver, setDragOver] = useState(false);
  const [summarizing, setSummarizing] = useState<number | null>(null);
  const [timestampedJobs, setTimestampedJobs] = useState<Set<number>>(
    () => new Set(),
  );

  const refresh = useCallback(async () => {
    setJobs(await commands.studioJobs());
  }, []);

  useEffect(() => {
    refresh();
    const unlisten = listen<Progress>("studio-progress", () => refresh());
    // Algún archivo quedó fuera por su ubicación. Se dice cuántos y no cuáles:
    // detallarlo convertiría el aviso en la vía para averiguar qué hay en el
    // disco, que es justo lo que se cerró en el backend.
    const unRejected = listen<number>("studio-paths-rejected", (e) => {
      toast.warning(t("studio.pathsRejected", { count: e.payload }));
    });
    return () => {
      unlisten.then((fn) => fn());
      unRejected.then((fn) => fn());
    };
  }, [refresh, t]);

  const enqueue = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      await commands.studioEnqueue(paths);
      refresh();
    },
    [refresh],
  );

  const pickFiles = async () => {
    const selected = await openDialog({
      multiple: true,
      filters: [
        {
          name: t("studio.audioVideo"),
          extensions: [
            "mp3",
            "m4a",
            "opus",
            "ogg",
            "oga",
            "mp4",
            "mov",
            "aac",
            "flac",
            "wav",
            "aiff",
            "aif",
            "caf",
          ],
        },
      ],
    });
    if (Array.isArray(selected)) await enqueue(selected);
    else if (selected) await enqueue([selected]);
  };

  useEffect(() => {
    const unlisten = listen<{ paths: string[] }>(
      "tauri://drag-drop",
      (event) => {
        setDragOver(false);
        if (event.payload?.paths) enqueue(event.payload.paths);
      },
    );
    const enter = listen("tauri://drag-enter", () => setDragOver(true));
    const leave = listen("tauri://drag-leave", () => setDragOver(false));
    return () => {
      unlisten.then((fn) => fn());
      enter.then((fn) => fn());
      leave.then((fn) => fn());
    };
  }, [enqueue]);

  const exportJob = async (job: StudioJob, format: string) => {
    const base = job.file_name.replace(/\.[^.]+$/, "");
    const path = await saveDialog({ defaultPath: `${base}.${format}` });
    if (!path) return;
    // El backend genera y ESCRIBE el archivo en la ruta elegida: así la UI no
    // necesita permiso de escritura al home (endurecido en capabilities).
    const result = await commands.studioExportTo(job.id, format, path);
    if (result.status === "error") {
      toast.error(t("studio.exportError"), { description: result.error });
      return;
    }
    toast.success(t("studio.exportSaved"));
  };

  const copyText = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(t("studio.copied"));
    } catch {
      toast.error(t("studio.copyError"));
    }
  };

  const getPlaybackUrl = async (id: number) => {
    const result = await commands.studioPlaybackUrl(id);
    return result.status === "ok" ? result.data : null;
  };

  const summarize = async (job: StudioJob) => {
    setSummarizing(job.id);
    const result = await commands.studioSummarize(job.id);
    setSummarizing(null);
    if (result.status === "error") {
      // Motor local no instalado / sin proveedor: sin esto el botón "no hace nada".
      toast.error(t("studio.summaryError"), { description: result.error });
      return;
    }
    refresh();
  };

  const retranscribe = async (job: StudioJob, modelId: string | null) => {
    const result = await commands.studioRetranscribe(job.id, modelId);
    if (result.status === "error") {
      toast.error(t("studio.genericError"), { description: result.error });
      return;
    }
    refresh();
  };

  const capabilities = [
    { icon: Lock, label: t("studio.capabilities.local") },
    { icon: Globe, label: t("studio.capabilities.languages") },
    { icon: Cpu, label: t("studio.capabilities.engine") },
  ];

  const completedCount = jobs.filter((job) => job.status === "done").length;

  const toggleTimestamps = (id: number) => {
    setTimestampedJobs((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const statusClasses: Record<StudioJob["status"], string> = {
    pending: "border-mid-gray/25 bg-mid-gray/10 text-mid-gray",
    processing: "border-logo-primary/30 bg-logo-primary/10 text-gold-text",
    done: "border-success/25 bg-success/10 text-success",
    error: "border-lacre/25 bg-lacre/10 text-lacre",
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      {/* Héroe editorial: vende el valor antes de arrastrar nada. */}
      <div>
        <h1
          className="text-3xl leading-tight text-text sm:text-[2rem]"
          style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
        >
          {t("studio.heroTitle")}
        </h1>
        <p className="mt-2 text-sm text-mid-gray">{t("studio.heroSubtitle")}</p>
      </div>

      <EngineRequiredCard />

      {/* Dropzone sólido (no punteado), con ícono grande y chips de formato. */}
      <button
        type="button"
        onClick={pickFiles}
        className={`relative w-full overflow-hidden rounded-card border p-10 text-center transition-all duration-200 ${
          dragOver
            ? "scale-[1.01] border-logo-primary bg-logo-primary/8 shadow-lift"
            : "border-mid-gray/25 bg-vitela/40 hover:border-logo-primary/50 hover:bg-logo-primary/4"
        }`}
      >
        <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-card bg-logo-primary/10 text-gold-text">
          <UploadCloud width={32} height={32} strokeWidth={1.5} />
        </div>
        <p className="mt-4 text-base font-semibold text-text">
          {t("studio.dropZoneTitle")}
        </p>
        <p className="mt-1 text-sm text-mid-gray">
          {t("studio.dropZoneClick")}
        </p>
        <div className="mt-4 flex flex-wrap justify-center gap-1.5">
          {SUPPORTED_FORMATS.map((fmt) => (
            <span
              key={fmt}
              className="rounded-md border border-mid-gray/20 bg-background px-2 py-0.5 font-mono text-3xs tracking-wide text-mid-gray"
            >
              {fmt}
            </span>
          ))}
        </div>
      </button>

      {/* Capacidades: la ventaja competitiva, siempre visible. */}
      <div className="grid grid-cols-3 gap-3">
        {capabilities.map(({ icon: Icon, label }) => (
          <div
            key={label}
            className="flex items-center gap-2.5 rounded-card border border-line bg-background px-3.5 py-3 shadow-card"
          >
            <Icon width={17} height={17} className="shrink-0 text-gold-text" />
            <span className="text-sm text-text">{label}</span>
          </div>
        ))}
      </div>

      {jobs.length === 0 && (
        <p className="pt-2 text-center text-sm text-mid-gray">
          {t("studio.empty")}
        </p>
      )}

      {jobs.length > 0 && (
        <div className="flex items-center justify-between gap-3 px-1 font-mono text-3xs font-semibold uppercase tracking-[0.14em] text-mid-gray">
          <p>{t("studio.recent")}</p>
          <p aria-live="polite">
            {t("studio.queueSummary", {
              done: completedCount,
              total: jobs.length,
            })}
          </p>
        </div>
      )}

      {jobs.map((job) => (
        <div
          key={job.id}
          className="rounded-lg border border-mid-gray/30 p-4 space-y-3"
        >
          <div className="flex items-center justify-between gap-3">
            <span className="min-w-0 flex-1 truncate font-medium text-text">
              {job.file_name}
            </span>
            <span
              className={`shrink-0 rounded-full border px-2 py-0.5 text-2xs font-medium ${statusClasses[job.status]}`}
              aria-live="polite"
            >
              {job.status === "processing"
                ? t("studio.status.processingProgress", {
                    progress: Math.round(job.progress * 100),
                  })
                : t(`studio.status.${job.status}`)}
            </span>
            <button
              type="button"
              onClick={() => commands.studioRemoveJob(job.id).then(refresh)}
              className="text-xs text-mid-gray hover:text-text"
            >
              {t("studio.remove")}
            </button>
          </div>

          {job.status === "processing" || job.status === "pending" ? (
            <div
              className="h-2 w-full overflow-hidden rounded-full bg-mid-gray/30"
              role="progressbar"
              aria-label={t("studio.progress")}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-valuenow={Math.round(job.progress * 100)}
            >
              <div
                className="h-full rounded-full bg-logo-primary transition-all"
                style={{ width: `${Math.round(job.progress * 100)}%` }}
              />
            </div>
          ) : null}

          {job.status === "error" && (
            <Alert variant="error" contained>
              {job.error ?? t("studio.genericError")}
            </Alert>
          )}

          {job.status === "done" && (
            <>
              {/*
                El motor reporta confianza por token; si hubo tramos donde
                estaba adivinando conviene releer ANTES de exportar, que es
                cuando todavía se puede corregir. Sin este aviso, la
                alucinación se descubre en el subtítulo ya publicado.
              */}
              {job.low_confidence && (
                <Alert variant="warning" contained>
                  {t("studio.lowConfidence")}
                </Alert>
              )}
              <AudioPlayer
                compact
                className="rounded-lg border border-mid-gray/20 bg-background px-3 py-2"
                onLoadRequest={() => getPlaybackUrl(job.id)}
              />
              <div className="max-h-40 overflow-y-auto whitespace-pre-wrap rounded bg-mid-gray/10 p-2 text-sm text-text/90">
                {timestampedJobs.has(job.id)
                  ? job.timestamped_text.join("\n")
                  : job.paragraphs.join("\n\n")}
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() =>
                    copyText(
                      timestampedJobs.has(job.id)
                        ? job.timestamped_text.join("\n")
                        : job.paragraphs.join("\n\n"),
                    )
                  }
                >
                  {t("studio.copy")}
                </Button>
                {job.timestamped_text.length > 0 && (
                  <Button
                    variant={
                      timestampedJobs.has(job.id) ? "primary-soft" : "secondary"
                    }
                    size="sm"
                    aria-pressed={timestampedJobs.has(job.id)}
                    onClick={() => toggleTimestamps(job.id)}
                  >
                    {timestampedJobs.has(job.id)
                      ? t("studio.hideTimestamps")
                      : t("studio.showTimestamps")}
                  </Button>
                )}
                {EXPORT_FORMATS.map((fmt) => (
                  <Button
                    key={fmt}
                    variant="secondary"
                    size="sm"
                    onClick={() => exportJob(job, fmt)}
                  >
                    {fmt.toUpperCase()}
                  </Button>
                ))}
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() =>
                    requestObsidianExport(
                      job.file_name.replace(/\.[^.]+$/, ""),
                      job.paragraphs.join("\n\n"),
                    )
                  }
                >
                  {t("obsidian.send")}
                </Button>
                <Button
                  variant="primary"
                  size="sm"
                  onClick={() => summarize(job)}
                  disabled={summarizing === job.id}
                >
                  {summarizing === job.id
                    ? t("studio.summarizing")
                    : t("studio.summarize")}
                </Button>
                <RetranscribeMenu
                  onRetranscribe={(modelId) => retranscribe(job, modelId)}
                  currentModelId={job.model_id}
                />
              </div>
              {job.summary && (
                <div className="text-sm text-text/90 whitespace-pre-wrap border-l-2 border-logo-primary pl-3 group relative">
                  <button
                    type="button"
                    onClick={() => copyText(job.summary || "")}
                    className="absolute top-0 right-0 text-xs text-mid-gray hover:text-text"
                  >
                    {t("studio.copy")}
                  </button>
                  {job.summary}
                </div>
              )}
            </>
          )}
        </div>
      ))}
    </div>
  );
};
