import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { commands, type LocalLlmStatus } from "@/bindings";
import { Alert } from "../../ui/Alert";
import { Button } from "../../ui/Button";
import { SettingContainer } from "@/components/ui";

type SetupProgress = {
  stage: string;
  downloaded: number;
  total: number;
  message: string;
};

const GB = 1024 * 1024 * 1024;

export const LocalLlmSetup: React.FC = () => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<LocalLlmStatus | null>(null);
  const [progress, setProgress] = useState<SetupProgress | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setStatus(await commands.getLocalLlmStatus());
  }, []);

  useEffect(() => {
    refresh();
    const unlisten = listen<SetupProgress>(
      "local-llm-setup-progress",
      (event) => {
        setProgress(event.payload);
        if (event.payload.stage === "done") {
          setInstalling(false);
          setProgress(null);
          refresh();
        }
        if (event.payload.stage === "error") {
          setInstalling(false);
          setError(event.payload.message);
        }
      },
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [refresh]);

  const handleInstall = async () => {
    setError(null);
    setInstalling(true);
    const result = await commands.setupLocalLlm();
    if (result.status === "error") {
      setError(result.error);
      setInstalling(false);
    }
    await refresh();
  };

  const ready = Boolean(status?.runtime_installed && status?.model_installed);
  const inProgress = installing || Boolean(status?.setup_in_progress);
  const percent =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.downloaded / progress.total) * 100))
      : 0;
  const downloadedGb = progress ? (progress.downloaded / GB).toFixed(2) : "0";
  const totalGb =
    progress && progress.total > 0 ? (progress.total / GB).toFixed(2) : "?";

  return (
    <SettingContainer
      title={t("settings.postProcessing.localLlm.title")}
      description={t("settings.postProcessing.localLlm.description")}
      descriptionMode="tooltip"
      layout="stacked"
      grouped={true}
    >
      <div className="flex flex-col gap-2 w-full">
        {ready ? (
          <Alert variant="success" contained>
            {t("settings.postProcessing.localLlm.ready")}
          </Alert>
        ) : inProgress ? (
          <div className="flex flex-col gap-1 w-full">
            <div className="h-2 w-full rounded-full bg-mid-gray/30 overflow-hidden">
              <div
                className="h-full rounded-full bg-logo-primary transition-all"
                style={{ width: `${percent}%` }}
              />
            </div>
            <span className="text-xs text-mid-gray">
              {progress?.stage === "runtime"
                ? t("settings.postProcessing.localLlm.downloadingRuntime")
                : t("settings.postProcessing.localLlm.downloadingModel")}
              {t("settings.postProcessing.localLlm.progressLine", {
                percent,
                downloaded: downloadedGb,
                total: totalGb,
              })}
            </span>
          </div>
        ) : (
          <div className="flex items-center gap-3">
            <Button
              variant="primary"
              size="md"
              onClick={handleInstall}
              disabled={inProgress}
            >
              {t("settings.postProcessing.localLlm.installButton")}
            </Button>
            <span className="text-xs text-mid-gray">
              {t("settings.postProcessing.localLlm.installHint")}
            </span>
          </div>
        )}
        {error && (
          <Alert variant="error" contained>
            {error}
          </Alert>
        )}
      </div>
    </SettingContainer>
  );
};
