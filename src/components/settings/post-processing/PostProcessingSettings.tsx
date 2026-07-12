import React, { useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import {
  ArrowRight,
  Check,
  Languages,
  Mic,
  Plus,
  RefreshCcw,
  Sparkles,
} from "lucide-react";
import { commands } from "@/bindings";

import { Alert } from "../../ui/Alert";
import {
  Dropdown,
  SettingContainer,
  SettingsGroup,
  Textarea,
} from "@/components/ui";
import { Button } from "../../ui/Button";
import { CollapsibleGroup } from "../../ui/CollapsibleGroup";
import { ResetButton } from "../../ui/ResetButton";
import { Input } from "../../ui/Input";
import { MicButton } from "../../shared/MicButton";

import { ProviderSelect } from "../PostProcessingSettingsApi/ProviderSelect";
import { LocalLlmSetup } from "../PostProcessingSettingsApi/LocalLlmSetup";
import { BaseUrlField } from "../PostProcessingSettingsApi/BaseUrlField";
import { ApiKeyField } from "../PostProcessingSettingsApi/ApiKeyField";
import { ModelSelect } from "../PostProcessingSettingsApi/ModelSelect";
import { usePostProcessProviderState } from "../PostProcessingSettingsApi/usePostProcessProviderState";
import { ShortcutInput } from "../ShortcutInput";
import { TranslationTargetLanguage } from "../TranslationTargetLanguage";
import { useSettings } from "../../../hooks/useSettings";

// Ids de las plantillas semilla: para ellas mostramos una descripción curada;
// las personalizadas muestran un extracto de su propio prompt.
const KNOWN_TEMPLATE_IDS = new Set([
  "escriba_dictado_natural",
  "escriba_prompt_maestro",
  "escriba_whatsapp",
  "escriba_email",
  "escriba_apuntes",
  "default_improve_transcriptions",
]);

const PostProcessingSettingsApiComponent: React.FC = () => {
  const { t } = useTranslation();
  const state = usePostProcessProviderState();

  return (
    <>
      <SettingContainer
        title={t("settings.postProcessing.api.provider.title")}
        description={t("settings.postProcessing.api.provider.description")}
        descriptionMode="tooltip"
        layout="horizontal"
        grouped={true}
      >
        <div className="flex items-center gap-2">
          <ProviderSelect
            options={state.providerOptions}
            value={state.selectedProviderId}
            onChange={state.handleProviderSelect}
          />
        </div>
      </SettingContainer>

      {state.selectedProvider?.id === "local_llm" ? (
        <LocalLlmSetup />
      ) : state.isAppleProvider ? (
        state.appleIntelligenceUnavailable ? (
          <Alert variant="error" contained>
            {t("settings.postProcessing.api.appleIntelligence.unavailable")}
          </Alert>
        ) : null
      ) : (
        <>
          {state.selectedProvider?.id === "custom" && (
            <SettingContainer
              title={t("settings.postProcessing.api.baseUrl.title")}
              description={t("settings.postProcessing.api.baseUrl.description")}
              descriptionMode="tooltip"
              layout="horizontal"
              grouped={true}
            >
              <div className="flex items-center gap-2">
                <BaseUrlField
                  value={state.baseUrl}
                  onBlur={state.handleBaseUrlChange}
                  placeholder={t(
                    "settings.postProcessing.api.baseUrl.placeholder",
                  )}
                  disabled={state.isBaseUrlUpdating}
                  className="min-w-[380px]"
                />
              </div>
            </SettingContainer>
          )}

          <SettingContainer
            title={t("settings.postProcessing.api.apiKey.title")}
            description={t("settings.postProcessing.api.apiKey.description")}
            descriptionMode="tooltip"
            layout="horizontal"
            grouped={true}
          >
            <div className="flex items-center gap-2">
              <ApiKeyField
                value={state.apiKey}
                onBlur={state.handleApiKeyChange}
                placeholder={t(
                  "settings.postProcessing.api.apiKey.placeholder",
                )}
                disabled={state.isApiKeyUpdating}
                className="min-w-[320px]"
              />
            </div>
          </SettingContainer>
        </>
      )}

      {!state.isAppleProvider && (
        <SettingContainer
          title={t("settings.postProcessing.api.model.title")}
          description={
            state.isCustomProvider
              ? t("settings.postProcessing.api.model.descriptionCustom")
              : t("settings.postProcessing.api.model.descriptionDefault")
          }
          descriptionMode="tooltip"
          layout="stacked"
          grouped={true}
        >
          <div className="flex items-center gap-2">
            <ModelSelect
              value={state.model}
              options={state.modelOptions}
              disabled={state.isModelUpdating}
              isLoading={state.isFetchingModels}
              placeholder={
                state.modelOptions.length > 0
                  ? t(
                      "settings.postProcessing.api.model.placeholderWithOptions",
                    )
                  : t("settings.postProcessing.api.model.placeholderNoOptions")
              }
              onSelect={state.handleModelSelect}
              onCreate={state.handleModelCreate}
              onBlur={() => {}}
              className="flex-1 min-w-[380px]"
            />
            <ResetButton
              onClick={state.handleRefreshModels}
              disabled={state.isFetchingModels}
              ariaLabel={t("settings.postProcessing.api.model.refreshModels")}
              className="flex h-10 w-10 items-center justify-center"
            >
              <RefreshCcw
                className={`h-4 w-4 ${state.isFetchingModels ? "animate-spin" : ""}`}
              />
            </ResetButton>
          </div>
        </SettingContainer>
      )}
    </>
  );
};

/**
 * Biblioteca de plantillas: presenta los prompts como tarjetas seleccionables
 * (los "modos" de la IA), con un editor plegable para la seleccionada y el
 * flujo de creación. Reutiliza el sistema real de prompts de Escriba.
 */
const TemplateLibraryComponent: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating, refreshSettings } =
    useSettings();
  const [isCreating, setIsCreating] = useState(false);
  const [draftName, setDraftName] = useState("");
  const [draftText, setDraftText] = useState("");

  const prompts = getSetting("post_process_prompts") || [];
  const selectedPromptId = getSetting("post_process_selected_prompt_id") || "";
  const selectedPrompt =
    prompts.find((prompt) => prompt.id === selectedPromptId) || null;

  useEffect(() => {
    if (isCreating) return;
    if (selectedPrompt) {
      setDraftName(selectedPrompt.name);
      setDraftText(selectedPrompt.prompt);
    } else {
      setDraftName("");
      setDraftText("");
    }
  }, [isCreating, selectedPromptId, selectedPrompt?.name, selectedPrompt?.prompt]);

  const selectTemplate = (promptId: string) => {
    if (isUpdating("post_process_selected_prompt_id")) return;
    updateSetting("post_process_selected_prompt_id", promptId);
    setIsCreating(false);
  };

  const handleCreatePrompt = async () => {
    if (!draftName.trim() || !draftText.trim()) return;
    try {
      const result = await commands.addPostProcessPrompt(
        draftName.trim(),
        draftText.trim(),
      );
      if (result.status === "ok") {
        await refreshSettings();
        updateSetting("post_process_selected_prompt_id", result.data.id);
        setIsCreating(false);
      }
    } catch (error) {
      console.error("Failed to create prompt:", error);
    }
  };

  const handleUpdatePrompt = async () => {
    if (!selectedPromptId || !draftName.trim() || !draftText.trim()) return;
    try {
      await commands.updatePostProcessPrompt(
        selectedPromptId,
        draftName.trim(),
        draftText.trim(),
      );
      await refreshSettings();
    } catch (error) {
      console.error("Failed to update prompt:", error);
    }
  };

  const handleDeletePrompt = async (promptId: string) => {
    if (!promptId) return;
    try {
      await commands.deletePostProcessPrompt(promptId);
      await refreshSettings();
      setIsCreating(false);
    } catch (error) {
      console.error("Failed to delete prompt:", error);
    }
  };

  const handleStartCreate = () => {
    setIsCreating(true);
    setDraftName("");
    setDraftText("");
  };

  const handleCancelCreate = () => {
    setIsCreating(false);
    if (selectedPrompt) {
      setDraftName(selectedPrompt.name);
      setDraftText(selectedPrompt.prompt);
    }
  };

  // Descripción legible por plantilla: curada para las semilla, extracto del
  // prompt para las personalizadas.
  const describe = (id: string, prompt: string) =>
    KNOWN_TEMPLATE_IDS.has(id)
      ? t(`settings.postProcessing.templateDesc.${id}`)
      : prompt.replace(/\$\{output\}/g, "").replace(/\s+/g, " ").trim().slice(0, 90) + "…";

  const isDirty =
    !!selectedPrompt &&
    (draftName.trim() !== selectedPrompt.name ||
      draftText.trim() !== selectedPrompt.prompt.trim());

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-sm font-semibold text-text">
          {t("settings.postProcessing.templatesTitle")}
        </h2>
        <p className="text-xs text-mid-gray">
          {t("settings.postProcessing.templatesSubtitle")}
        </p>
      </div>

      {/* Rejilla de plantillas como tarjetas seleccionables. */}
      <div className="grid gap-2.5 sm:grid-cols-2">
        {prompts.map((p) => {
          const active = p.id === selectedPromptId && !isCreating;
          return (
            <button
              key={p.id}
              type="button"
              onClick={() => selectTemplate(p.id)}
              className={`group relative rounded-xl border p-3.5 text-left transition-all ${
                active
                  ? "border-logo-primary bg-logo-primary/5 shadow-[0_1px_2px_rgba(27,20,38,0.06)]"
                  : "border-mid-gray/15 bg-background hover:-translate-y-0.5 hover:border-mid-gray/30"
              }`}
            >
              <div className="flex items-start justify-between gap-2">
                <span className="text-sm font-semibold text-text">
                  {p.name}
                </span>
                {active && (
                  <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-logo-primary text-ink">
                    <Check width={13} height={13} />
                  </span>
                )}
              </div>
              <p className="mt-1 text-xs leading-relaxed text-mid-gray">
                {describe(p.id, p.prompt)}
              </p>
              {!KNOWN_TEMPLATE_IDS.has(p.id) && (
                <span className="mt-2 inline-block rounded-full border border-mid-gray/25 px-2 py-0.5 text-[10px] font-medium text-mid-gray">
                  {t("settings.postProcessing.customBadge")}
                </span>
              )}
            </button>
          );
        })}

        {/* Tarjeta para crear una plantilla nueva. */}
        <button
          type="button"
          onClick={handleStartCreate}
          className={`flex items-center justify-center gap-2 rounded-xl border border-dashed p-3.5 text-sm font-medium transition-colors ${
            isCreating
              ? "border-logo-primary bg-logo-primary/5 text-logo-primary"
              : "border-mid-gray/30 text-mid-gray hover:border-logo-primary/50 hover:text-logo-primary"
          }`}
        >
          <Plus width={16} height={16} />
          {t("settings.postProcessing.prompts.createNew")}
        </button>
      </div>

      {/* Editor de la plantilla seleccionada (plegado por defecto). */}
      {!isCreating && selectedPrompt && (
        <CollapsibleGroup title={t("settings.postProcessing.editTemplate")}>
          <div className="space-y-3 p-4">
            <div className="flex flex-col space-y-2">
              <label className="text-sm font-semibold">
                {t("settings.postProcessing.prompts.promptLabel")}
              </label>
              <Input
                type="text"
                value={draftName}
                onChange={(e) => setDraftName(e.target.value)}
                placeholder={t(
                  "settings.postProcessing.prompts.promptLabelPlaceholder",
                )}
                variant="compact"
              />
            </div>
            <div className="flex flex-col space-y-2">
              <div className="flex items-center justify-between">
                <label className="text-sm font-semibold">
                  {t("settings.postProcessing.prompts.promptInstructions")}
                </label>
                <MicButton
                  onText={(text) =>
                    setDraftText((prev) => (prev ? `${prev} ${text}` : text))
                  }
                  title={t("micButton.dictatePrompt")}
                />
              </div>
              <Textarea
                value={draftText}
                onChange={(e) => setDraftText(e.target.value)}
                placeholder={t(
                  "settings.postProcessing.prompts.promptInstructionsPlaceholder",
                )}
              />
              <p className="text-xs text-mid-gray/70">
                <Trans
                  i18nKey="settings.postProcessing.prompts.promptTip"
                  components={{ code: <code /> }}
                />
              </p>
            </div>
            <div className="flex gap-2 pt-1">
              <Button
                onClick={handleUpdatePrompt}
                variant="primary"
                size="md"
                disabled={!draftName.trim() || !draftText.trim() || !isDirty}
              >
                {t("settings.postProcessing.prompts.updatePrompt")}
              </Button>
              <Button
                onClick={() => handleDeletePrompt(selectedPromptId)}
                variant="secondary"
                size="md"
                disabled={!selectedPromptId || prompts.length <= 1}
              >
                {t("settings.postProcessing.prompts.deletePrompt")}
              </Button>
            </div>
          </div>
        </CollapsibleGroup>
      )}

      {/* Flujo de creación. */}
      {isCreating && (
        <div className="space-y-3 rounded-xl border border-logo-primary/30 bg-logo-primary/5 p-4">
          <div className="flex flex-col space-y-2">
            <label className="text-sm font-semibold text-text">
              {t("settings.postProcessing.prompts.promptLabel")}
            </label>
            <Input
              type="text"
              value={draftName}
              onChange={(e) => setDraftName(e.target.value)}
              placeholder={t(
                "settings.postProcessing.prompts.promptLabelPlaceholder",
              )}
              variant="compact"
            />
          </div>
          <div className="flex flex-col space-y-2">
            <div className="flex items-center justify-between">
              <label className="text-sm font-semibold">
                {t("settings.postProcessing.prompts.promptInstructions")}
              </label>
              <MicButton
                onText={(text) =>
                  setDraftText((prev) => (prev ? `${prev} ${text}` : text))
                }
                title={t("micButton.dictatePrompt")}
              />
            </div>
            <Textarea
              value={draftText}
              onChange={(e) => setDraftText(e.target.value)}
              placeholder={t(
                "settings.postProcessing.prompts.promptInstructionsPlaceholder",
              )}
            />
          </div>
          <div className="flex gap-2 pt-1">
            <Button
              onClick={handleCreatePrompt}
              variant="primary"
              size="md"
              disabled={!draftName.trim() || !draftText.trim()}
            >
              {t("settings.postProcessing.prompts.createPrompt")}
            </Button>
            <Button onClick={handleCancelCreate} variant="secondary" size="md">
              {t("settings.postProcessing.prompts.cancel")}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};

// Un par entrada → salida ilustrativo (no es un resultado en vivo).
const ExampleCard: React.FC<{ id: string }> = ({ id }) => {
  const { t } = useTranslation();
  return (
    <div className="rounded-xl border border-mid-gray/15 bg-background p-4 shadow-[0_1px_2px_rgba(27,20,38,0.04)]">
      <p className="mb-2.5 flex items-center gap-1.5 text-xs font-semibold text-logo-primary">
        <Sparkles width={13} height={13} />
        {t(`settings.postProcessing.example.${id}.label`)}
      </p>
      <p className="rounded-lg bg-mid-gray/5 px-3 py-2 text-xs italic leading-relaxed text-mid-gray">
        {t(`settings.postProcessing.example.${id}.in`)}
      </p>
      <div className="my-1.5 flex justify-center text-mid-gray/50">
        <ArrowRight width={14} height={14} className="rotate-90" />
      </div>
      <p
        className="whitespace-pre-line rounded-lg border border-logo-primary/20 bg-logo-primary/5 px-3 py-2 text-sm leading-relaxed text-text"
        style={{ fontFamily: "var(--font-serif)" }}
      >
        {t(`settings.postProcessing.example.${id}.out`)}
      </p>
    </div>
  );
};

export const PostProcessingSettingsApi = React.memo(
  PostProcessingSettingsApiComponent,
);
PostProcessingSettingsApi.displayName = "PostProcessingSettingsApi";

export const PostProcessingSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = getSetting("post_process_enabled") || false;

  const FLOW = [
    { key: "speak", icon: Mic },
    { key: "transcribe", icon: null },
    { key: "ai", icon: Sparkles },
    { key: "ready", icon: null },
  ];

  return (
    <div className="mx-auto w-full max-w-3xl space-y-8 py-2">
      {/* Bloque 1 — Héroe: qué es y por qué importa. */}
      <div>
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="max-w-xl">
            <h1
              className="text-3xl leading-tight text-text sm:text-[2rem]"
              style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
            >
              {t("settings.postProcessing.heroTitle")}
            </h1>
            <p className="mt-2 text-sm leading-relaxed text-mid-gray">
              {t("settings.postProcessing.heroSubtitle")}
            </p>
          </div>
          {/* Interruptor compacto, sin ocupar una tarjeta entera. */}
          <label className="flex shrink-0 cursor-pointer items-center gap-2.5 rounded-full border border-mid-gray/20 bg-background px-3.5 py-2 shadow-[0_1px_2px_rgba(27,20,38,0.04)]">
            <span className="text-xs font-medium text-text">
              {enabled
                ? t("settings.postProcessing.enabledBadge")
                : t("settings.postProcessing.disabledBadge")}
            </span>
            <input
              type="checkbox"
              className="h-4 w-4 accent-logo-primary"
              checked={enabled}
              disabled={isUpdating("post_process_enabled")}
              onChange={(e) =>
                updateSetting("post_process_enabled", e.target.checked)
              }
            />
          </label>
        </div>

        {/* Diagrama de flujo en una línea: Hablas → Transcripción → IA → Listo. */}
        <div className="mt-5 flex flex-wrap items-center gap-2">
          {FLOW.map(({ key, icon: Icon }, i) => (
            <React.Fragment key={key}>
              <span
                className={`flex items-center gap-1.5 rounded-lg border px-3 py-1.5 text-xs font-medium ${
                  key === "ai"
                    ? "border-logo-primary/30 bg-logo-primary/10 text-logo-primary"
                    : "border-mid-gray/15 bg-background text-mid-gray"
                }`}
              >
                {Icon && <Icon width={13} height={13} />}
                {t(`settings.postProcessing.flow.${key}`)}
              </span>
              {i < FLOW.length - 1 && (
                <ArrowRight width={14} height={14} className="text-mid-gray/40" />
              )}
            </React.Fragment>
          ))}
        </div>
      </div>

      {/* Ejemplos: vende el valor con pares entrada → salida. */}
      <section className="space-y-3">
        <h2 className="text-sm font-semibold text-text">
          {t("settings.postProcessing.examplesTitle")}
        </h2>
        <div className="grid gap-3 sm:grid-cols-2">
          {["natural", "email", "notes", "translate"].map((id) => (
            <ExampleCard key={id} id={id} />
          ))}
        </div>
      </section>

      {/* Bloque 2 — Qué quieres que haga la IA: plantillas, traducción, atajos. */}
      <TemplateLibraryComponent />

      <SettingsGroup title={t("settings.postProcessing.translation.title")}>
        <ShortcutInput
          shortcutId="transcribe_translate"
          descriptionMode="tooltip"
          grouped={true}
        />
        <TranslationTargetLanguage />
      </SettingsGroup>

      <SettingsGroup title={t("settings.postProcessing.shortcutsTitle")}>
        <ShortcutInput
          shortcutId="transcribe_with_post_process"
          descriptionMode="tooltip"
          grouped={true}
        />
        <ShortcutInput
          shortcutId="voice_edit"
          descriptionMode="tooltip"
          grouped={true}
        />
      </SettingsGroup>

      {/* Bloque 3 — Opciones avanzadas: el 95% no lo toca. */}
      <div className="pt-2">
        <CollapsibleGroup title={t("settings.postProcessing.advancedTitle")}>
          <div className="p-4">
            <PostProcessingSettingsApi />
          </div>
        </CollapsibleGroup>
      </div>
    </div>
  );
};
