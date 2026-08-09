import React, { useState, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { ProgressBar } from "../shared";
import { Button } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { useSettings } from "../../hooks/useSettings";
import { commands } from "../../bindings";

interface UpdateCheckerProps {
  className?: string;
}

const UpdateChecker: React.FC<UpdateCheckerProps> = ({ className = "" }) => {
  const { t } = useTranslation();
  // Update checking state
  const [isChecking, setIsChecking] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [isInstalling, setIsInstalling] = useState(false);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const [showUpToDate, setShowUpToDate] = useState(false);
  const [showPortableUpdateDialog, setShowPortableUpdateDialog] =
    useState(false);

  const { settings, isLoading } = useSettings();
  const settingsLoaded = !isLoading && settings !== null;
  const updateChecksEnabled = settings?.update_checks_enabled ?? false;

  const upToDateTimeoutRef = useRef<ReturnType<typeof setTimeout>>();
  const isManualCheckRef = useRef(false);
  const isCheckingRef = useRef(false);
  const downloadedBytesRef = useRef(0);
  const contentLengthRef = useRef(0);

  useEffect(() => {
    // Wait for settings to load before doing anything
    if (!settingsLoaded) return;

    // Este ajuste controla solo la comprobación AUTOMÁTICA. La búsqueda manual
    // debe seguir disponible: antes apagar el chequeo de fondo inutilizaba
    // también el botón y la opción del menú de bandeja.
    if (updateChecksEnabled) {
      void checkForUpdates(false);
    }

    // Listen for update check events
    const updateUnlisten = listen("check-for-updates", () => {
      handleManualUpdateCheck();
    });

    return () => {
      if (upToDateTimeoutRef.current) {
        clearTimeout(upToDateTimeoutRef.current);
      }
      updateUnlisten.then((fn) => fn());
    };
  }, [settingsLoaded, updateChecksEnabled]);

  // Update checking functions
  const checkForUpdates = async (manual: boolean) => {
    if ((!manual && !updateChecksEnabled) || isCheckingRef.current) return;

    try {
      isManualCheckRef.current = manual;
      isCheckingRef.current = true;
      setIsChecking(true);
      const update = await check();

      if (update) {
        setUpdateAvailable(true);
        setShowUpToDate(false);
        // Solo conservamos el metadato. `installUpdate` vuelve a comprobar para
        // descargar el artefacto correcto y este recurso nativo ya no hace falta.
        await update.close().catch((error) => {
          // El resultado ya es útil; fallar al liberar el handle no convierte
          // una comprobación correcta en un error de red para la persona.
          console.warn("Failed to close updater metadata resource:", error);
        });
      } else {
        setUpdateAvailable(false);

        if (isManualCheckRef.current) {
          setShowUpToDate(true);
          if (upToDateTimeoutRef.current) {
            clearTimeout(upToDateTimeoutRef.current);
          }
          upToDateTimeoutRef.current = setTimeout(() => {
            setShowUpToDate(false);
          }, 3000);
        }
      }
    } catch (error) {
      console.error("Failed to check for updates:", error);
      // Solo en chequeos manuales: el chequeo de fondo no debe interrumpir.
      if (isManualCheckRef.current) {
        toast.error(t("footer.updateCheckFailed"));
      }
    } finally {
      isCheckingRef.current = false;
      setIsChecking(false);
      isManualCheckRef.current = false;
    }
  };

  const handleManualUpdateCheck = () => {
    void checkForUpdates(true);
  };

  const installUpdate = async () => {
    let update: Awaited<ReturnType<typeof check>> = null;
    try {
      const portable = await commands.isPortable();
      if (portable) {
        setShowPortableUpdateDialog(true);
        return;
      }

      setIsInstalling(true);
      setDownloadProgress(0);
      downloadedBytesRef.current = 0;
      contentLengthRef.current = 0;
      update = await check();

      if (!update) {
        console.log("No update available during install attempt");
        setUpdateAvailable(false);
        setShowUpToDate(true);
        return;
      }

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            downloadedBytesRef.current = 0;
            contentLengthRef.current = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloadedBytesRef.current += event.data.chunkLength;
            const progress =
              contentLengthRef.current > 0
                ? Math.round(
                    (downloadedBytesRef.current / contentLengthRef.current) *
                      100,
                  )
                : 0;
            setDownloadProgress(Math.min(progress, 100));
            break;
        }
      });
      await relaunch();
    } catch (error) {
      console.error("Failed to install update:", error);
      toast.error(t("footer.updateInstallFailed"));
    } finally {
      if (update) {
        await update.close().catch((error) => {
          console.warn("Failed to close updater resource:", error);
        });
      }
      setIsInstalling(false);
      setDownloadProgress(0);
      downloadedBytesRef.current = 0;
      contentLengthRef.current = 0;
    }
  };

  // Update status functions
  const getUpdateStatusText = () => {
    if (isInstalling) {
      return downloadProgress > 0 && downloadProgress < 100
        ? t("footer.downloading", {
            progress: downloadProgress.toString().padStart(3),
          })
        : downloadProgress === 100
          ? t("footer.installing")
          : t("footer.preparing");
    }
    if (isChecking) return t("footer.checkingUpdates");
    if (showUpToDate) return t("footer.upToDate");
    if (updateAvailable) return t("footer.updateAvailableShort");
    return t("footer.checkForUpdates");
  };

  const getUpdateStatusAction = () => {
    if (updateAvailable && !isInstalling) return installUpdate;
    if (!isChecking && !isInstalling && !updateAvailable)
      return handleManualUpdateCheck;
    return undefined;
  };

  const isUpdateDisabled = isChecking || isInstalling;
  const isUpdateClickable =
    !isUpdateDisabled && (updateAvailable || (!isChecking && !showUpToDate));

  return (
    <>
      {/*
        Este diálogo estaba escrito a mano con `bg-bg border-border`, clases que
        no corresponden a ningún token del tema: se renderizaba sin fondo ni
        borde, como texto suelto sobre el backdrop. Además no atrapaba el foco ni
        cerraba con Escape. `Dialog` ya resuelve las tres cosas.
      */}
      <Dialog
        open={showPortableUpdateDialog}
        onOpenChange={setShowPortableUpdateDialog}
        title={t("footer.portableUpdateTitle")}
        closeLabel={t("common.close")}
        footer={
          <>
            <Button
              variant="secondary"
              onClick={() => setShowPortableUpdateDialog(false)}
            >
              {t("common.close")}
            </Button>
            <Button
              variant="primary"
              onClick={() => {
                openUrl(
                  "https://github.com/AlejandroAP9/Escriba/releases/latest",
                );
                setShowPortableUpdateDialog(false);
              }}
            >
              {t("footer.portableUpdateButton")}
            </Button>
          </>
        }
      >
        <p className="text-sm text-text">{t("footer.portableUpdateMessage")}</p>
      </Dialog>
      <div className={`flex items-center gap-3 ${className}`}>
        {isUpdateClickable ? (
          <Button
            type="button"
            variant={updateAvailable ? "primary" : "secondary"}
            size="sm"
            onClick={getUpdateStatusAction()}
            disabled={isUpdateDisabled}
            className="tabular-nums"
          >
            {getUpdateStatusText()}
          </Button>
        ) : (
          <span className="text-text/60 tabular-nums">
            {getUpdateStatusText()}
          </span>
        )}

        {isInstalling && downloadProgress > 0 && downloadProgress < 100 && (
          <ProgressBar
            progress={[
              {
                id: "update",
                percentage: downloadProgress,
              },
            ]}
            size="large"
          />
        )}
      </div>
    </>
  );
};

export default UpdateChecker;
