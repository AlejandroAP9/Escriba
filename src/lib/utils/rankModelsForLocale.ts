import type { ModelInfo } from "@/bindings";

/**
 * Escriba: los top picks del catálogo vienen pensados para angloparlantes
 * (rank 1 = Parakeet, SOLO inglés). Si la app corre en otro idioma,
 * re-rankeamos: los recomendados que hablan el idioma del usuario suben,
 * los que no lo hablan bajan al final de su grupo.
 * No inventa recomendados nuevos: solo reordena los del catálogo.
 */
export function rankModelsForLocale(
  models: ModelInfo[],
  appLanguage: string,
): ModelInfo[] {
  const lang = (appLanguage || "en").split("-")[0].toLowerCase();
  if (lang === "en") return models;

  const speaksUserLanguage = (m: ModelInfo) =>
    m.supported_languages.length === 0 || // desconocido: no castigar
    m.supported_languages.includes(lang);

  // Orden estable: primero los que hablan tu idioma (manteniendo el rank
  // original entre ellos), después los que no.
  return [...models].sort((a, b) => {
    const aSpeaks = speaksUserLanguage(a) ? 0 : 1;
    const bSpeaks = speaksUserLanguage(b) ? 0 : 1;
    return aSpeaks - bSpeaks;
  });
}

/** Un modelo recomendado que NO habla el idioma del usuario merece aviso. */
export function isEnglishOnlyForUser(
  model: ModelInfo,
  appLanguage: string,
): boolean {
  const lang = (appLanguage || "en").split("-")[0].toLowerCase();
  if (lang === "en") return false;
  return (
    model.supported_languages.length > 0 &&
    !model.supported_languages.includes(lang)
  );
}
