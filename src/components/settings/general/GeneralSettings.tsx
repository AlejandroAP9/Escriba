import React from "react";
import { useTranslation } from "react-i18next";
import { type } from "@tauri-apps/plugin-os";
import { ArrowRight, Keyboard, Mic, Sparkles } from "lucide-react";
import { AppearanceSettings } from "../AppearanceSettings";
import { PermissionsPanel } from "../PermissionsPanel";
import { MicrophoneSelector } from "../MicrophoneSelector";
import { ShortcutInput } from "../ShortcutInput";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { CollapsibleGroup } from "../../ui/CollapsibleGroup";
import { OutputDeviceSelector } from "../OutputDeviceSelector";
import { PushToTalk } from "../PushToTalk";
import { ReviewBeforePaste } from "../ReviewBeforePaste";
import { AudioFeedback } from "../AudioFeedback";
import { useSettings } from "../../../hooks/useSettings";
import { useModelStore } from "../../../stores/modelStore";
import { VolumeSlider } from "../VolumeSlider";
import { MuteWhileRecording } from "../MuteWhileRecording";
import { PauseMediaOnDictate } from "../PauseMediaOnDictate";
import { NoiseSuppression } from "../NoiseSuppression";
import { CustomWords } from "../CustomWords";
import { SpanishDictation } from "../SpanishDictation";
import { TextReplacements } from "../TextReplacements";
import { ObsidianVault } from "../ObsidianVault";
import { AutostartToggle } from "../AutostartToggle";
import { StartHidden } from "../StartHidden";
import { ShowTrayIcon } from "../ShowTrayIcon";
import { VoiceEngineSelector } from "../VoiceEngineSelector";
import { ModelSettingsCard } from "./ModelSettingsCard";

export const GeneralSettings: React.FC = () => {
  const { t } = useTranslation();
  const { audioFeedbackEnabled, getSetting } = useSettings();
  const { currentModel, models } = useModelStore();
  const pushToTalk = getSetting("push_to_talk");
  const isLinux = type() === "linux";
  const isMacos = type() === "macos";

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
                    ? "border-logo-primary/30 bg-logo-primary/10 text-gold-text"
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
            <span className="ml-auto font-mono text-2xs text-mid-gray">
              {t("settings.general.activeModel")}: {activeModelName}
            </span>
          )}
        </div>
      </div>

      {/* Atajo principal: el ajuste más importante, como protagonista. */}
      <div className="rounded-card border border-logo-primary/25 bg-logo-primary/5 p-1.5 shadow-card">
        <div className="flex items-center gap-2 px-3 pt-2.5 text-gold-text">
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
      {isMacos && (
        <SettingsGroup title={t("settings.general.permissions.title")}>
          <PermissionsPanel />
        </SettingsGroup>
      )}

      <SettingsGroup title={t("settings.general.appearance.title")}>
        <AppearanceSettings />
      </SettingsGroup>

      <SettingsGroup title={t("settings.general.dictationTitle")}>
        <PushToTalk descriptionMode="inline" grouped={true} />
        <ReviewBeforePaste descriptionMode="inline" grouped={true} />
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

      {/* Diccionario personal: los nombres que el modelo debe respetar
          ("Imperio Agéntico", apellidos del curso). Vivía enterrado en
          Avanzado → Transcripción, siendo de lo más pedido del aula. */}
      <SettingsGroup title={t("settings.general.dictionaryTitle")}>
        <CustomWords descriptionMode="inline" grouped />
        <TextReplacements descriptionMode="inline" grouped />
      </SettingsGroup>

      {/* Español profundo: correcciones deterministas del dictado en español.
          Las tildes inequívocas corren siempre (sin interruptor); esto agrupa
          lo que sí se elige. */}
      <SettingsGroup title={t("settings.general.spanishTitle")}>
        <SpanishDictation descriptionMode="inline" grouped />
      </SettingsGroup>

      {/* Audio: todo lo relacionado con el sonido, junto. */}
      <SettingsGroup title={t("settings.general.audioTitle")}>
        <MicrophoneSelector descriptionMode="inline" grouped={true} />
        <NoiseSuppression descriptionMode="inline" grouped={true} />
        <MuteWhileRecording descriptionMode="inline" grouped={true} />
        <PauseMediaOnDictate descriptionMode="inline" grouped={true} />
        <AudioFeedback descriptionMode="inline" grouped={true} />
        {/* Leer en voz alta es SALIDA de sonido, no escritura: vivía en la
            página de Escritura Inteligente por herencia, entre atajos que sí
            disparan el motor de IA. Este es su grupo. */}
        <ShortcutInput
          shortcutId="read_selection"
          descriptionMode="inline"
          grouped={true}
        />
        <VoiceEngineSelector
          setting="read_selection_voice_engine"
          titleKey="settings.general.readSelectionVoice.title"
          descriptionKey="settings.general.readSelectionVoice.description"
        />
      </SettingsGroup>

      {/* Obsidian: dónde aterrizan las notas. Lo configura hasta el asistente
          de bienvenida; tenerlo en Avanzado era esconderlo. */}
      <SettingsGroup title={t("settings.general.obsidianTitle")}>
        <ObsidianVault descriptionMode="inline" grouped={true} />
      </SettingsGroup>

      {/* Aplicación: arranque y bandeja. Ajustes de cualquier app, no de
          usuario avanzado. */}
      <SettingsGroup title={t("settings.general.appTitle")}>
        <AutostartToggle descriptionMode="inline" grouped={true} />
        <StartHidden descriptionMode="inline" grouped={true} />
        <ShowTrayIcon descriptionMode="inline" grouped={true} />
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
