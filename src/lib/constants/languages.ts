export interface Language {
  value: string;
  label: string;
}

export const CHINESE_LANGUAGE_CODE = "zh";

export const LANGUAGES: Language[] = [
  { value: "auto", label: "Detección automática" },
  { value: "en", label: "Inglés" },
  { value: CHINESE_LANGUAGE_CODE, label: "Chino" },
  { value: "zh-Hans", label: "Chino (simplificado)" },
  { value: "zh-Hant", label: "Chino (tradicional)" },
  { value: "yue", label: "Cantonés" },
  { value: "de", label: "Alemán" },
  { value: "es", label: "Español" },
  { value: "ru", label: "Ruso" },
  { value: "ko", label: "Coreano" },
  { value: "fr", label: "Francés" },
  { value: "ja", label: "Japonés" },
  { value: "pt", label: "Portugués" },
  { value: "tr", label: "Turco" },
  { value: "pl", label: "Polaco" },
  { value: "ca", label: "Catalán" },
  { value: "nl", label: "Neerlandés" },
  { value: "ar", label: "Árabe" },
  { value: "sv", label: "Sueco" },
  { value: "it", label: "Italiano" },
  { value: "id", label: "Indonesio" },
  { value: "hi", label: "Hindi" },
  { value: "fi", label: "Finés" },
  { value: "vi", label: "Vietnamita" },
  { value: "he", label: "Hebreo" },
  { value: "uk", label: "Ucraniano" },
  { value: "el", label: "Griego" },
  { value: "ms", label: "Malayo" },
  { value: "cs", label: "Checo" },
  { value: "ro", label: "Rumano" },
  { value: "da", label: "Danés" },
  { value: "hu", label: "Húngaro" },
  { value: "ta", label: "Tamil" },
  { value: "no", label: "Noruego" },
  { value: "th", label: "Tailandés" },
  { value: "ur", label: "Urdu" },
  { value: "hr", label: "Croata" },
  { value: "bg", label: "Búlgaro" },
  { value: "lt", label: "Lituano" },
  { value: "la", label: "Latín" },
  { value: "mi", label: "Maorí" },
  { value: "ml", label: "Malayalam" },
  { value: "cy", label: "Galés" },
  { value: "sk", label: "Eslovaco" },
  { value: "te", label: "Telugu" },
  { value: "fa", label: "Persa" },
  { value: "lv", label: "Letón" },
  { value: "bn", label: "Bengalí" },
  { value: "sr", label: "Serbio" },
  { value: "az", label: "Azerbaiyano" },
  { value: "sl", label: "Esloveno" },
  { value: "kn", label: "Canarés" },
  { value: "et", label: "Estonio" },
  { value: "mk", label: "Macedonio" },
  { value: "br", label: "Bretón" },
  { value: "eu", label: "Euskera" },
  { value: "is", label: "Islandés" },
  { value: "hy", label: "Armenio" },
  { value: "ne", label: "Nepalí" },
  { value: "mn", label: "Mongol" },
  { value: "bs", label: "Bosnio" },
  { value: "kk", label: "Kazajo" },
  { value: "sq", label: "Albanés" },
  { value: "sw", label: "Suajili" },
  { value: "gl", label: "Gallego" },
  { value: "mr", label: "Maratí" },
  { value: "pa", label: "Panyabí" },
  { value: "si", label: "Cingalés" },
  { value: "km", label: "Jemer" },
  { value: "sn", label: "Shona" },
  { value: "yo", label: "Yoruba" },
  { value: "so", label: "Somalí" },
  { value: "af", label: "Afrikáans" },
  { value: "oc", label: "Occitano" },
  { value: "ka", label: "Georgiano" },
  { value: "be", label: "Bielorruso" },
  { value: "tg", label: "Tayiko" },
  { value: "sd", label: "Sindhi" },
  { value: "gu", label: "Guyaratí" },
  { value: "am", label: "Amárico" },
  { value: "yi", label: "Yidis" },
  { value: "lo", label: "Lao" },
  { value: "uz", label: "Uzbeko" },
  { value: "fo", label: "Feroés" },
  { value: "ht", label: "Criollo haitiano" },
  { value: "ps", label: "Pastún" },
  { value: "tk", label: "Turcomano" },
  { value: "nn", label: "Noruego (nynorsk)" },
  { value: "mt", label: "Maltés" },
  { value: "sa", label: "Sánscrito" },
  { value: "lb", label: "Luxemburgués" },
  { value: "my", label: "Birmano" },
  { value: "bo", label: "Tibetano" },
  { value: "tl", label: "Tagalo" },
  { value: "mg", label: "Malgache" },
  { value: "as", label: "Asamés" },
  { value: "tt", label: "Tártaro" },
  { value: "haw", label: "Hawaiano" },
  { value: "ln", label: "Lingala" },
  { value: "ha", label: "Hausa" },
  { value: "ba", label: "Baskir" },
  { value: "jw", label: "Javanés" },
  { value: "su", label: "Sundanés" },
];

const CHINESE_OUTPUT_INTENTS = new Set(["zh-Hans", "zh-Hant"]);

const LANGUAGE_LABELS = new Map(
  LANGUAGES.map((language) => [language.value, language.label] as const),
);

export const MODEL_CAPABILITY_LANGUAGES: Language[] = LANGUAGES.filter(
  (language) =>
    language.value !== "auto" && !CHINESE_OUTPUT_INTENTS.has(language.value),
);

// Languages offered in the transcription-language picker. We surface the two
// explicit Chinese *output* variants (Simplified / Traditional) and hide the
// bare recognition code `zh` ("Chinese"): all three recognize identically, so
// the plain option only adds ambiguity about which script you get. `zh` stays in
// LANGUAGES — it's still a valid *effective* language (auto-detect and must-pick
// fallback can resolve to it) and its label is needed to render that state — it
// just isn't directly selectable.
export const SELECTABLE_LANGUAGES: Language[] = LANGUAGES.filter(
  (language) => language.value !== CHINESE_LANGUAGE_CODE,
);

// Collapse a language tag to the base code Handy matches on, dropping any
// BCP-47 region or script subtag: "en-US" → "en", "zh-CN" → "zh", "zh-Hant" →
// "zh". Bare and three-letter codes ("haw") pass through unchanged. This lets
// the picker match a model's *real* codes — which may be full locales like
// "en-US" (e.g. Nemotron Streaming) — against Handy's canonical bare-code
// LANGUAGES list without the backend having to mangle the codes the engine needs.
export const recognitionLanguage = (languageCode: string): string => {
  const separatorIndex = languageCode.indexOf("-");
  return separatorIndex === -1
    ? languageCode
    : languageCode.slice(0, separatorIndex);
};

export const supportsLanguageCode = (
  supportedLanguages: string[],
  languageCode: string,
): boolean => {
  const recognitionCode = recognitionLanguage(languageCode);
  return supportedLanguages.some(
    (supportedLanguage) =>
      recognitionLanguage(supportedLanguage) === recognitionCode,
  );
};

export const getUniqueCapabilityLanguages = (
  supportedLanguages: string[],
): string[] => {
  const seen = new Set<string>();
  return supportedLanguages.map(recognitionLanguage).filter((languageCode) => {
    if (seen.has(languageCode)) return false;
    seen.add(languageCode);
    return true;
  });
};

export const getLanguageLabel = (languageCode: string): string | undefined =>
  LANGUAGE_LABELS.get(languageCode);
