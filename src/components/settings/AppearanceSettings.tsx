import React from "react";
import { useTranslation } from "react-i18next";
import { Monitor, Moon, Sun } from "lucide-react";
import { useSettings } from "../../hooks/useSettings";
import { SettingContainer } from "../ui/SettingContainer";
import { ToggleSwitch } from "../ui/ToggleSwitch";

/**
 * Apariencia (tanda de inclusión): tema Día/Noche/Sistema, tamaño del texto y
 * modos de accesibilidad visual. Nacen de feedback de la comunidad (15-jul):
 * ayudas visuales para ojos cansados o vista reducida, y modo nocturno
 * controlable sin tocar macOS. Los tres modos visuales (alto contraste,
 * daltonismo, Modo Calma) son tokens CSS sobre el sistema de tema; ver
 * styles/a11y.css.
 */

const THEMES = [
  { id: "system", icon: Monitor },
  { id: "light", icon: Sun },
  { id: "dark", icon: Moon },
] as const;

// Escalas curadas (no un slider): saltos perceptibles y probados.
const SCALES = [90, 100, 115, 130] as const;

export const AppearanceSettings: React.FC<{ grouped?: boolean }> = ({
  grouped = true,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();

  const theme = (getSetting("ui_theme") ?? "system") as string;
  const scale = (getSetting("ui_scale") ?? 100) as number;
  const highContrast = (getSetting("high_contrast") ?? false) as boolean;
  const colorblind = (getSetting("colorblind_assist") ?? false) as boolean;
  const calmMode = (getSetting("calm_mode") ?? false) as boolean;

  return (
    <>
      <SettingContainer
        title={t("settings.general.appearance.themeTitle")}
        description={t("settings.general.appearance.themeDescription")}
        grouped={grouped}
        layout="horizontal"
      >
        <div className="flex items-center gap-1 rounded-lg border border-line bg-background p-0.5">
          {THEMES.map(({ id, icon: Icon }) => (
            <button
              key={id}
              type="button"
              onClick={() => updateSetting("ui_theme", id)}
              className={`flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-xs font-medium transition-colors focus:outline-none focus:ring-1 focus:ring-logo-primary ${
                theme === id
                  ? "bg-logo-primary/15 text-gold-text"
                  : "text-mid-gray hover:text-text"
              }`}
            >
              <Icon width={13} height={13} />
              {t(`settings.general.appearance.theme.${id}`)}
            </button>
          ))}
        </div>
      </SettingContainer>

      <SettingContainer
        title={t("settings.general.appearance.textSizeTitle")}
        description={t("settings.general.appearance.textSizeDescription")}
        grouped={grouped}
        layout="horizontal"
      >
        <div className="flex items-center gap-1 rounded-lg border border-line bg-background p-0.5">
          {SCALES.map((pct) => (
            <button
              key={pct}
              type="button"
              onClick={() => updateSetting("ui_scale", pct)}
              aria-label={t("settings.general.appearance.textSizeOption", {
                pct,
              })}
              className={`rounded-md px-2.5 py-1 font-serif transition-colors focus:outline-none focus:ring-1 focus:ring-logo-primary ${
                scale === pct
                  ? "bg-logo-primary/15 text-gold-text"
                  : "text-mid-gray hover:text-text"
              }`}
              // El propio botón muestra el tamaño: la A crece con la escala.
              style={{ fontSize: `${(pct / 100) * 0.95}rem`, lineHeight: 1 }}
            >
              A
            </button>
          ))}
        </div>
      </SettingContainer>

      <ToggleSwitch
        checked={calmMode}
        onChange={(v) => updateSetting("calm_mode", v)}
        label={t("settings.general.appearance.calmModeTitle")}
        description={t("settings.general.appearance.calmModeDescription")}
        grouped={grouped}
      />

      <ToggleSwitch
        checked={highContrast}
        onChange={(v) => updateSetting("high_contrast", v)}
        label={t("settings.general.appearance.highContrastTitle")}
        description={t("settings.general.appearance.highContrastDescription")}
        grouped={grouped}
      />

      <ToggleSwitch
        checked={colorblind}
        onChange={(v) => updateSetting("colorblind_assist", v)}
        label={t("settings.general.appearance.colorblindTitle")}
        description={t("settings.general.appearance.colorblindDescription")}
        grouped={grouped}
      />
    </>
  );
};
