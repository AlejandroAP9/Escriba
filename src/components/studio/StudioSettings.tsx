import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import {
  open as openDialog,
  save as saveDialog,
} from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { commands, type StudioJob } from "@/bindings";
import { Button } from "../ui/Button";
import { Alert } from "../ui/Alert";

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
          extensions: ["mp3", "m4a", "mp4", "mov", "aac", "flac", "ogg", "wav"],
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
    const result = await commands.studioExport(job.id, format);
    if (result.status === "error") return;
    const base = job.file_name.replace(/\.[^.]+$/, "");
    const path = await saveDialog({ defaultPath: `${base}.${format}` });
    if (path) await writeTextFile(path, result.data);
  };

  const summarize = async (job: StudioJob) => {
    setSummarizing(job.id);
    await commands.studioSummarize(job.id);
    setSummarizing(null);
    refresh();
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <button
        type="button"
        onClick={pickFiles}
        className={`w-full rounded-xl border-2 border-dashed p-8 text-center transition-colors ${
          dragOver
            ? "border-logo-primary bg-logo-primary/10"
            : "border-mid-gray/40 hover:border-logo-primary/60"
        }`}
      >
        <p className="text-text font-medium">{t("studio.dropZoneTitle")}</p>
        <p className="text-sm text-mid-gray mt-1">{t("studio.dropZoneHint")}</p>
      </button>

      {jobs.length === 0 && (
        <p className="text-sm text-mid-gray text-center">{t("studio.empty")}</p>
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
              </div>
              {job.summary && (
                <div className="text-sm text-text/90 whitespace-pre-wrap border-l-2 border-logo-primary pl-3">
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
