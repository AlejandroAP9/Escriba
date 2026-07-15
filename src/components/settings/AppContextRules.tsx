import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, X } from "lucide-react";
import type { AppContextRule, LLMPrompt } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Dropdown } from "../ui/Dropdown";
import { ToggleSwitch } from "../ui/ToggleSwitch";

/**
 * Tonos por app: reglas "app activa → plantilla de IA". Al dictar con
 * post-proceso, si la app en primer plano coincide, esa plantilla pisa a la
 * global. Dictas igual en todas partes y cada app recibe su tono.
 */
export const AppContextRules: React.FC = React.memo(() => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = getSetting("app_context_enabled") || false;
  const prompts: LLMPrompt[] = getSetting("post_process_prompts") || [];
  const [rules, setRules] = useState<AppContextRule[]>(
    () => getSetting("app_context_rules") || [],
  );

  const persist = (next: AppContextRule[]) => {
    setRules(next);
    updateSetting("app_context_rules", next);
  };

  const addRule = () =>
    persist([...rules, { app_match: "", prompt_id: prompts[0]?.id ?? "" }]);
  const removeRule = (i: number) => persist(rules.filter((_, x) => x !== i));
  const editApp = (i: number, v: string) =>
    setRules(rules.map((r, x) => (x === i ? { ...r, app_match: v } : r)));
  const commit = () => persist(rules);
  const setPrompt = (i: number, id: string) =>
    persist(rules.map((r, x) => (x === i ? { ...r, prompt_id: id } : r)));

  const promptOptions = prompts.map((p) => ({ value: p.id, label: p.name }));

  return (
    <div className="space-y-3">
      <div className="rounded-card border border-line bg-background px-4 py-1 shadow-card">
        <ToggleSwitch
          checked={enabled}
          onChange={(v) => updateSetting("app_context_enabled", v)}
          isUpdating={isUpdating("app_context_enabled")}
          label={t("settings.postProcessing.appContext.enable")}
          description={t("settings.postProcessing.appContext.enableDesc")}
          descriptionMode="tooltip"
        />
      </div>

      {enabled && (
        <div className="space-y-2">
          {rules.length === 0 && (
            <p className="px-1 text-xs text-mid-gray">
              {t("settings.postProcessing.appContext.empty")}
            </p>
          )}
          {rules.map((rule, i) => (
            <div
              key={i}
              className="flex flex-wrap items-center gap-2 rounded-card border border-line bg-background p-2.5 shadow-card"
            >
              <Input
                type="text"
                value={rule.app_match}
                onChange={(e) => editApp(i, e.target.value)}
                onBlur={commit}
                placeholder={t(
                  "settings.postProcessing.appContext.appPlaceholder",
                )}
                variant="compact"
                className="min-w-[140px] flex-1"
              />
              <span className="text-xs text-mid-gray">→</span>
              <Dropdown
                selectedValue={rule.prompt_id || null}
                options={promptOptions}
                onSelect={(v) => setPrompt(i, v)}
                placeholder={t(
                  "settings.postProcessing.appContext.pickTemplate",
                )}
                className="min-w-[180px] flex-1"
              />
              <button
                type="button"
                onClick={() => removeRule(i)}
                title={t("settings.postProcessing.appContext.remove")}
                className="shrink-0 rounded-md p-1.5 text-mid-gray transition-colors hover:text-lacre"
              >
                <X width={15} height={15} />
              </button>
            </div>
          ))}

          <button
            type="button"
            onClick={addRule}
            className="flex items-center gap-1.5 rounded-card border border-dashed border-mid-gray/30 px-3 py-2 text-sm font-medium text-mid-gray transition-colors hover:border-logo-primary/50 hover:text-logo-primary"
          >
            <Plus width={15} height={15} />
            {t("settings.postProcessing.appContext.add")}
          </button>

          <p className="px-1 text-[11px] leading-relaxed text-mid-gray/80">
            {t("settings.postProcessing.appContext.hint")}
          </p>
        </div>
      )}
    </div>
  );
});
AppContextRules.displayName = "AppContextRules";
