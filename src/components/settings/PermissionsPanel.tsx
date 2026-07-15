import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { platform } from "@tauri-apps/plugin-os";
import {
  checkAccessibilityPermission,
  checkMicrophonePermission,
} from "tauri-plugin-macos-permissions-api";
import { commands } from "@/bindings";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

/**
 * Panel central de permisos (solo macOS): los tres que Escriba necesita, con
 * su estado REAL y el botón directo a Ajustes del Sistema. Antes el usuario
 * descubría los permisos a golpes, feature por feature (auditoría premium +
 * QA de Flor). Se refresca al volver el foco a la ventana: conceder un
 * permiso ocurre fuera de la app.
 */

const PANE = "x-apple.systempreferences:com.apple.preference.security?Privacy_";

type Perm = {
  key: "accessibility" | "microphone" | "screen";
  pane: string;
  check: () => Promise<boolean>;
};

const PERMS: Perm[] = [
  {
    key: "accessibility",
    pane: "Accessibility",
    check: () => checkAccessibilityPermission(),
  },
  {
    key: "microphone",
    pane: "Microphone",
    check: () => checkMicrophonePermission(),
  },
  {
    key: "screen",
    pane: "ScreenCapture",
    check: () => commands.systemAudioPermission(),
  },
];

export const PermissionsPanel: React.FC = () => {
  const { t } = useTranslation();
  const [status, setStatus] = useState<Record<string, boolean | null>>({});

  const refresh = useCallback(() => {
    PERMS.forEach((p) => {
      p.check()
        .then((ok) => setStatus((s) => ({ ...s, [p.key]: ok })))
        .catch(() => {});
    });
  }, []);

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [refresh]);

  if (platform() !== "macos") return null;

  return (
    <>
      {PERMS.map((p) => {
        const ok = status[p.key];
        return (
          <SettingContainer
            key={p.key}
            title={t(`settings.general.permissions.${p.key}.title`)}
            description={t(`settings.general.permissions.${p.key}.description`)}
            grouped
            layout="horizontal"
          >
            <div className="flex items-center gap-2.5">
              <span
                className={`flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-medium ${
                  ok === false
                    ? "bg-lacre/10 text-lacre"
                    : ok
                      ? "bg-green-600/10 text-green-600"
                      : "bg-mid-gray/10 text-mid-gray"
                }`}
              >
                <span
                  className={`h-1.5 w-1.5 rounded-full ${
                    ok === false
                      ? "bg-lacre"
                      : ok
                        ? "bg-green-600"
                        : "bg-mid-gray/50"
                  }`}
                />
                {ok === false
                  ? t("settings.general.permissions.missing")
                  : ok
                    ? t("settings.general.permissions.granted")
                    : "…"}
              </span>
              {ok === false && (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => openUrl(PANE + p.pane)}
                >
                  {t("settings.general.permissions.open")}
                </Button>
              )}
            </div>
          </SettingContainer>
        );
      })}
    </>
  );
};
