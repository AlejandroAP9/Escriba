import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { ArrowRight, Keyboard, Mic, Sparkles } from "lucide-react";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { CollapsibleGroup } from "../../ui/CollapsibleGroup";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { PushToTalk } from "../PushToTalk";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";
import { useModelStore } from "../../../stores/modelStore";
import { VolumeSlider } from "../VolumeSlider";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { PauseMediaOnDictate } from "../PauseMediaOnDictate";
import { NoiseSuppression } from "../NoiseSuppression";
import { ModelSettingsCard } from "./ModelSettingsCard";

export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { audioFeedbackEnabled, getSetting } = useSettings();
  const { currentModel, models } = useModelStore();
  const pushToTalk = getSetting("push_to_talk");
  const isLinux = type() === "linux";

  const activeModelName = models.find((m) => m.id === currentModel)?.name;

  // Flujo educativo: en 5 segundos se entiende cómo funciona Escriba.
  const FLOW = [
    { key: "speak", icon: Mic },
    { key: "whisper", icon: Sparkles },
    { key: "text", icon: null },
    { key: "anywhere", icon: null },
  ];

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 py-2">
      {/* Encabezado contextual: orienta en vez de repetir el marketing. */}
      <div>
        <h1
          className="text-3xl leading-tight text-text sm:text-[2rem]"
          style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
        >
          {t("settings.general.contextTitle")}
        </h1>
        <p className="mt-2 text-sm leading-relaxed text-mid-gray">
          {t("settings.general.contextSubtitle")}
        </p>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          {FLOW.map(({ key, icon: Icon }, i) => (
            <React.Fragment key={key}>
              <span
                className={`flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs font-medium ${
                  key === "whisper"
                    ? "border-logo-primary/30 bg-logo-primary/10 text-logo-primary"
                    : "border-line bg-background text-mid-gray"
                }`}
              >
                {Icon && <Icon width={13} height={13} />}
                {t(`settings.general.flow.${key}`)}
              </span>
              {i < FLOW.length - 1 && (
                <ArrowRight
                  width={14}
                  height={14}
                  className="text-mid-gray/40"
                />
              )}
            </React.Fragment>
          ))}
          {activeModelName && (
            <span className="ml-auto font-mono text-[11px] text-mid-gray">
              {t("settings.general.activeModel")}: {activeModelName}
            </span>
          )}
        </div>
      </div>

      {/* Atajo principal: el ajuste más importante, como protagonista. */}
      <div className="rounded-card border border-logo-primary/25 bg-logo-primary/5 p-1.5 shadow-card">
        <div className="flex items-center gap-2 px-3 pt-2.5 text-logo-primary">
          <Keyboard width={15} height={15} />
          <span className="text-xs font-semibold uppercase tracking-[0.08em]">
            {t("settings.general.title")}
          </span>
        </div>
        <ShortcutInput
          shortcutId="transcribe"
          descriptionMode="inline"
          grouped={true}
        />
      </div>

      {/* Dictado: modo (push-to-talk) y cancelación. */}
      <SettingsGroup title={t("settings.general.dictationTitle")}>
        <PushToTalk descriptionMode="inline" grouped={true} />
        {/* El atajo de cancelar se oculta con push-to-talk (soltar cancela) y en Linux. */}
        {!isLinux && !pushToTalk && (
          <ShortcutInput
            shortcutId="cancel"
            descriptionMode="inline"
            grouped={true}
          />
        )}
      </SettingsGroup>

      {/* Transcripción: idioma + traducir al inglés. */}
      <ModelSettingsCard />

      {/* Audio: todo lo relacionado con el sonido, junto. */}
      <SettingsGroup title={t("settings.general.audioTitle")}>
        <MicrophoneSelector descriptionMode="inline" grouped={true} />
        <NoiseSuppression descriptionMode="inline" grouped={true} />
        <MuteWhileRecording descriptionMode="inline" grouped={true} />
        <PauseMediaOnDictate descriptionMode="inline" grouped={true} />
        <AudioFeedback descriptionMode="inline" grouped={true} />
      </SettingsGroup>

      {/* Avanzado: lo que el 90% no toca, plegado. */}
      <CollapsibleGroup title={t("settings.general.advancedTitle")}>
        <div className="space-y-1 p-2">
          <OutputDeviceSelector
            descriptionMode="inline"
            grouped={true}
            disabled={!audioFeedbackEnabled}
          />
          <VolumeSlider disabled={!audioFeedbackEnabled} />
        </div>
      </CollapsibleGroup>
    </div>
  );
};
