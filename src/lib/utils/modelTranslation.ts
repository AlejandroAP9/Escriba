import i18n from "i18next";
import type { TFunction } from "i18next";
import type { ModelInfo } from "@/bindings";
import { MODEL_DESCRIPTIONS_ES } from "./modelDescriptions.es";

/**
 * Get the translated name for a model
 * @param model - The model info object
 * @param t - The translation function from useTranslation
 * @returns The translated model name, or the original name if no translation exists
 */
export function getTranslatedModelName(model: ModelInfo, t: TFunction): string {
  const translationKey = `onboarding.models.${model.id}.name`;
  const translated = t(translationKey, { defaultValue: "" });
  return translated !== "" ? translated : model.name;
}

/**
 * Get the translated description for a model
 * @param model - The model info object
 * @param t - The translation function from useTranslation
 * @returns The translated model description, or the original description if no translation exists
 */
export function getTranslatedModelDescription(
  model: ModelInfo,
  t: TFunction,
): string {
  // Custom models use a generic translation key
  if (model.is_custom) {
    return t("onboarding.customModelDescription");
  }
  const translationKey = `onboarding.models.${model.id}.description`;
  const translated = t(translationKey, { defaultValue: "" });
  if (translated !== "") {
    return translated;
  }
  // Escriba: catálogo localizado por texto de descripción (estable entre
  // quants; los ids incluyen el archivo y son frágiles como clave i18n).
  if ((i18n.language || "").startsWith("es")) {
    const es = MODEL_DESCRIPTIONS_ES[model.description];
    if (es) {
      return es;
    }
  }
  return model.description;
}
