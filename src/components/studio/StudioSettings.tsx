import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { listen } from "@tauri-apps/api/event";
import {
  open as openDialog,
  save as saveDialog,
} from "@tauri-apps/plugin-dialog";
import { Lock, Globe, Cpu, UploadCloud } from "lucide-react";
import { commands, type StudioJob } from "@/bindings";
import { Button } from "../ui/Button";
import { Alert } from "../ui/Alert";
import { RetranscribeMenu } from "../shared/RetranscribeMenu";

const SUPPORTED_FORMATS = ["MP3", "WAV", "M4A", "MP4", "MOV", "FLAC"];

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

  const refresh = useCallback(async () => {
    setJobs(await commands.studioJobs());
  }, []);

  useEffect(() => {
    refresh();
    const unlisten = listen<Progress>("studio-progress", () => refresh());
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

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
            "mp4",
            "mov",
            "aac",
            "flac",
            "wav",
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
        <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-card bg-logo-primary/10 text-logo-primary">
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
              className="rounded-md border border-mid-gray/20 bg-background px-2 py-0.5 font-mono text-[10px] tracking-wide text-mid-gray"
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
            <Icon
              width={17}
              height={17}
              className="shrink-0 text-logo-primary"
            />
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
        <p className="px-1 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-mid-gray">
          {t("studio.recent")}
        </p>
      )}

      {jobs.map((job) => (
        <div
          key={job.id}
          className="rounded-lg border border-mid-gray/30 p-4 space-y-3"
        >
          <div className="flex items-center justify-between gap-2">
            <span className="font-medium text-text truncate">
              {job.file_name}
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
            <div className="h-2 w-full rounded-full bg-mid-gray/30 overflow-hidden">
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
              <div className="max-h-40 overflow-y-auto text-sm text-text/90 whitespace-pre-wrap bg-mid-gray/10 rounded p-2">
                {job.paragraphs.join("\n\n")}
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => copyText(job.paragraphs.join("\n\n"))}
                >
                  {t("studio.copy")}
                </Button>
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
