import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingContainer } from "../../ui/SettingContainer";
import { commands } from "@/bindings";

interface DebugPathsProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

/**
 * Rutas internas reales de la app.
 *
 * Antes mostraba tres literales fijos con formato de Windows
 * (`%APPDATA%/handy`, ...), que estaban mal por partida triple: decían "handy"
 * cuando el identificador es `com.escriba.app`, se mostraban igual en macOS y
 * Linux donde esa sintaxis no existe, y al ser fijos no reflejaban el modo
 * portátil, que cambia el directorio de datos por completo. Un panel de
 * depuración que miente es peor que no tenerlo. Ahora sale del backend con
 * `get_app_dir_path`, que ya resuelve el modo portátil.
 */
export const DebugPaths: React.FC<DebugPathsProps> = ({
  descriptionMode = "inline",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const [appDir, setAppDir] = useState<string | null>(null);
  const [logDir, setLogDir] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    void (async () => {
      const [app, log] = await Promise.all([
        commands.getAppDirPath(),
        commands.getLogDirPath(),
      ]);
      if (!alive) return;
      if (app.status === "ok") setAppDir(app.data);
      if (log.status === "ok") setLogDir(log.data);
    })();
    return () => {
      alive = false;
    };
  }, []);

  // Separador de rutas según lo que devuelva el backend, para componer las
  // subcarpetas sin asumir la plataforma.
  const sep = appDir?.includes("\\") ? "\\" : "/";
  const join = (base: string | null, child: string) =>
    base ? `${base}${sep}${child}` : null;

  const rows: { label: string; path: string | null }[] = [
    { label: t("settings.debug.paths.appData"), path: appDir },
    { label: t("settings.debug.paths.models"), path: join(appDir, "models") },
    {
      label: t("settings.debug.paths.settings"),
      path: join(appDir, "settings_store.json"),
    },
    { label: t("settings.debug.paths.logs"), path: logDir },
  ];

  return (
    <SettingContainer
      title={t("a11y.debugPaths")}
      description={t("settings.debug.paths.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    >
      <div className="text-sm text-mid-gray space-y-2">
        {rows.map((row) => (
          <div key={row.label}>
            <span className="font-medium">{row.label}</span>{" "}
            <span className="font-mono text-xs select-text break-all">
              {row.path ?? t("common.loading")}
            </span>
          </div>
        ))}
      </div>
    </SettingContainer>
  );
};
