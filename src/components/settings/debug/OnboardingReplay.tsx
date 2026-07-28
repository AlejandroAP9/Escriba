import React from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../ui/Button";
import { SettingContainer } from "../../ui/SettingContainer";
import { replayOnboarding } from "@/lib/navigation";

interface OnboardingReplayProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Volver a ver la bienvenida de Plumín.
 *
 * El asistente solo aparece en instalación nueva, así que quien ya usa la app
 * no tenía forma de verlo salvo borrando su configuración entera. Esto lo
 * relanza sin tocar ningún ajuste: sirve para grabar la presentación tantas
 * veces como haga falta, y para quien quiera repasar cómo era.
 */
export const OnboardingReplay: React.FC<OnboardingReplayProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();

  return (
    <SettingContainer
      title={t("settings.debug.replayOnboarding.title")}
      description={t("settings.debug.replayOnboarding.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    >
      <Button variant="secondary" size="md" onClick={replayOnboarding}>
        {t("settings.debug.replayOnboarding.button")}
      </Button>
    </SettingContainer>
  );
};

OnboardingReplay.displayName = "OnboardingReplay";
