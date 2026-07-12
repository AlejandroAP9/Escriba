import React from "react";
import { useTranslation } from "react-i18next";
import {
  AudioLines,
  Check,
  Download,
  Globe,
  HardDrive,
  Languages,
  Loader2,
  Star,
  Trash2,
  Zap,
} from "lucide-react";
import type { ModelInfo } from "@/bindings";
import { formatModelSize } from "../../lib/utils/format";
import {
  getTranslatedModelDescription,
  getTranslatedModelName,
} from "../../lib/utils/modelTranslation";
import {
  getLanguageLabel,
  getUniqueCapabilityLanguages,
} from "../../lib/constants/languages";
import Badge from "../ui/Badge";
import { Button } from "../ui/Button";

// Get display text for model's language support
const getLanguageDisplayText = (
  supportedLanguages: string[],
  t: (key: string, options?: Record<string, unknown>) => string,
): string => {
  const capabilityLanguages = getUniqueCapabilityLanguages(supportedLanguages);
  if (capabilityLanguages.length === 1) {
    const langCode = capabilityLanguages[0];
    const langName = getLanguageLabel(langCode) || langCode;
    return t("modelSelector.capabilities.languageOnly", { language: langName });
  }
  return t("modelSelector.capabilities.languageCount", {
    total: capabilityLanguages.length,
  });
};

// Legacy = a blob (Url-sourced) .bin/ONNX model, kept runnable but no longer the
// advertised download (catalog GGUFs supersede it).
export const isLegacySource = (model: ModelInfo): boolean =>
  typeof model.source === "object" && "Url" in model.source;

// Extract a GGUF quantization label from a filename, if present (e.g. "Q8_0").
const getQuantLabel = (filename: string): string | null => {
  const match = filename.match(
    /[._-](IQ\d+_\w+|Q\d+(?:_\w+)?|F16|BF16|F32)\.gguf$/i,
  );
  return match ? match[1].toUpperCase() : null;
};

export type ModelCardStatus =
  | "downloadable"
  | "downloading"
  | "verifying"
  | "extracting"
  | "switching"
  | "active"
  | "available";

interface ModelCardProps {
  model: ModelInfo;
  variant?: "default" | "featured";
  status?: ModelCardStatus;
  disabled?: boolean;
  className?: string;
  onSelect: (modelId: string) => void;
  onDownload?: (modelId: string) => void;
  onDelete?: (modelId: string) => void;
  onCancel?: (modelId: string) => void;
  downloadProgress?: number;
  downloadSpeed?: number; // MB/s
  showRecommended?: boolean;
}

const ModelCard: React.FC<ModelCardProps> = ({
  model,
  variant = "default",
  status = "downloadable",
  disabled = false,
  className = "",
  onSelect,
  onDownload,
  onDelete,
  onCancel,
  downloadProgress,
  downloadSpeed,
  showRecommended = true,
}) => {
  const { t } = useTranslation();
  const isFeatured = variant === "featured";
  const isClickable =
    status === "available" || status === "active" || status === "downloadable";

  // Get translated model name and description
  const displayName = getTranslatedModelName(model, t);
  const displayDescription = getTranslatedModelDescription(model, t);
  const showModelSize =
    status === "downloadable" || status === "available" || status === "active";
  const formattedModelSize = formatModelSize(Number(model.size_mb));
  const quantLabel = getQuantLabel(model.filename);
  const capabilityLanguages = getUniqueCapabilityLanguages(
    model.supported_languages,
  );

  const baseClasses =
    "flex flex-col rounded-xl px-4 py-3 gap-2 text-left transition-all duration-200";

  const getVariantClasses = () => {
    // El oro se reserva SOLO para el modelo activo (una única jerarquía). Los
    // recomendados y la biblioteca usan un borde gris fino: el ojo encuentra al
    // instante cuál está activo.
    if (status === "active") {
      return "border-2 border-logo-primary/50 bg-logo-primary/[0.07]";
    }
    return "border border-mid-gray/15";
  };

  const getInteractiveClasses = () => {
    if (!isClickable) return "";
    if (disabled) return "opacity-50 cursor-not-allowed";
    // Hover que ELEVA 2px con sombra sutil (sin zoom brusco).
    return "cursor-pointer hover:border-logo-primary/40 hover:-translate-y-0.5 hover:shadow-[0_12px_28px_-14px_rgba(27,20,38,0.30)] group";
  };

  const handleClick = () => {
    if (!isClickable || disabled) return;
    if (status === "downloadable" && onDownload) {
      onDownload(model.id);
    } else {
      onSelect(model.id);
    }
  };

  // Badge útil: en vez del "Recomendado" genérico repetido, responde POR QUÉ
  // destaca este modelo (mejor calidad / más rápido / en vivo / más idiomas).
  const recommendationBadge = (): {
    icon: React.ReactNode;
    label: string;
  } | null => {
    if (!showRecommended || !model.is_recommended) return null;
    const iconCls = "w-3 h-3 mr-1";
    if (model.accuracy_score >= 0.9)
      return {
        icon: <Star className={iconCls} />,
        label: t("modelSelector.why.bestQuality"),
      };
    if (model.speed_score >= 0.9)
      return {
        icon: <Zap className={iconCls} />,
        label: t("modelSelector.why.fastest"),
      };
    if (model.supports_streaming)
      return {
        icon: <AudioLines className={iconCls} />,
        label: t("modelSelector.why.live"),
      };
    if (getUniqueCapabilityLanguages(model.supported_languages).length >= 90)
      return {
        icon: <Globe className={iconCls} />,
        label: t("modelSelector.why.mostLanguages"),
      };
    return { icon: null, label: t("onboarding.recommended") };
  };
  const whyBadge = recommendationBadge();

  const handleDelete = (e: React.MouseEvent) => {
    e.stopPropagation();
    onDelete?.(model.id);
  };

  return (
    <div
      onClick={handleClick}
      onKeyDown={(e) => {
        if (e.key === "Enter" && isClickable) handleClick();
      }}
      role={isClickable ? "button" : undefined}
      tabIndex={isClickable ? 0 : undefined}
      className={[
        baseClasses,
        getVariantClasses(),
        getInteractiveClasses(),
        className,
      ]
        .filter(Boolean)
        .join(" ")}
    >
      {/* Top section: name/description + score bars */}
      <div className="flex justify-between items-center w-full">
        <div className="flex flex-col items-start flex-1 min-w-0">
          <div className="flex items-center gap-3 flex-wrap">
            <h3
              className={`text-base font-semibold text-text ${isClickable ? "group-hover:text-logo-primary" : ""} transition-colors`}
            >
              {displayName}
            </h3>
            {whyBadge && (
              <Badge variant="secondary">
                {whyBadge.icon}
                {whyBadge.label}
              </Badge>
            )}
            {status === "active" && (
              <Badge variant="primary">
                <Check className="w-3 h-3 mr-1" />
                {t("modelSelector.active")}
              </Badge>
            )}
            {model.is_custom && (
              <Badge variant="secondary">{t("modelSelector.custom")}</Badge>
            )}
            {isLegacySource(model) && (
              <Badge variant="secondary">{t("modelSelector.legacy")}</Badge>
            )}
            {status === "switching" && (
              <Badge variant="secondary">
                <Loader2 className="w-3 h-3 mr-1 animate-spin" />
                {t("modelSelector.switching")}
              </Badge>
            )}
          </div>
          <p className="text-text/60 text-sm leading-relaxed">
            {displayDescription}
          </p>
        </div>
        {(model.accuracy_score > 0 || model.speed_score > 0) && (
          // Etiqueta pegada ARRIBA de su barra (se lee de un vistazo), barras
          // más anchas y gruesas.
          <div className="hidden sm:flex flex-col gap-2 ms-4 w-28 shrink-0">
            <div>
              <p className="mb-1 text-[11px] leading-none text-mid-gray">
                {t("onboarding.modelCard.accuracy")}
              </p>
              <div className="h-2 w-full overflow-hidden rounded-full bg-mid-gray/15">
                <div
                  className="h-full rounded-full bg-logo-primary"
                  style={{ width: `${model.accuracy_score * 100}%` }}
                />
              </div>
            </div>
            <div>
              <p className="mb-1 text-[11px] leading-none text-mid-gray">
                {t("onboarding.modelCard.speed")}
              </p>
              <div className="h-2 w-full overflow-hidden rounded-full bg-mid-gray/15">
                <div
                  className="h-full rounded-full bg-logo-primary"
                  style={{ width: `${model.speed_score * 100}%` }}
                />
              </div>
            </div>
          </div>
        )}
      </div>

      <hr className="w-full border-mid-gray/20" />

      {/* Bottom row: tags + action buttons (full width) */}
      <div className="flex items-center gap-3 w-full -mb-0.5 mt-0.5 h-5">
        {/* Ícono en gris, texto más oscuro: la información se lee, el ícono acompaña. */}
        {capabilityLanguages.length > 0 && (
          <div
            className="flex items-center gap-1 text-xs text-text/75"
            title={
              capabilityLanguages.length === 1
                ? t("modelSelector.capabilities.singleLanguage")
                : t("modelSelector.capabilities.languageSelection")
            }
          >
            <Globe className="w-3.5 h-3.5 text-mid-gray/70" />
            <span>{getLanguageDisplayText(model.supported_languages, t)}</span>
          </div>
        )}
        {model.supports_translation && (
          <div
            className="flex items-center gap-1 text-xs text-text/75"
            title={t("modelSelector.capabilities.translation")}
          >
            <Languages className="w-3.5 h-3.5 text-mid-gray/70" />
            <span>{t("modelSelector.capabilities.translate")}</span>
          </div>
        )}
        {model.supports_streaming && (
          <div
            className="flex items-center gap-1 text-xs text-text/75"
            title={t("modelSelector.capabilities.streaming")}
          >
            <AudioLines className="w-3.5 h-3.5 text-mid-gray/70" />
            <span>{t("modelSelector.streaming")}</span>
          </div>
        )}
        {showModelSize && (
          <span className="flex items-center gap-1.5 ms-auto text-xs text-text/75">
            {status === "downloadable" ? (
              <Download className="w-3.5 h-3.5 text-mid-gray/70" />
            ) : (
              <HardDrive className="w-3.5 h-3.5 text-mid-gray/70" />
            )}
            <span>{formattedModelSize}</span>
            {quantLabel && <span className="text-mid-gray/70">{quantLabel}</span>}
          </span>
        )}
        {onDelete && (status === "available" || status === "active") && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleDelete}
            title={t("modelSelector.deleteModel", { modelName: displayName })}
            className="flex items-center gap-1.5 text-mid-gray hover:text-red-600 hover:bg-red-500/10"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>{t("common.delete")}</span>
          </Button>
        )}
      </div>

      {/* Download/extract progress */}
      {status === "downloading" && downloadProgress !== undefined && (
        <div className="w-full mt-3">
          <div className="w-full h-1.5 bg-mid-gray/20 rounded-full overflow-hidden">
            <div
              className="h-full bg-logo-primary rounded-full transition-all duration-300"
              style={{ width: `${downloadProgress}%` }}
            />
          </div>
          <div className="flex items-center justify-between text-xs mt-1">
            <span className="text-text/50">
              {t("modelSelector.downloading", {
                percentage: Math.round(downloadProgress),
              })}
            </span>
            <div className="flex items-center gap-2">
              {downloadSpeed !== undefined && downloadSpeed > 0 && (
                <span className="tabular-nums text-text/50">
                  {t("modelSelector.downloadSpeed", {
                    speed: downloadSpeed.toFixed(1),
                  })}
                </span>
              )}
              {onCancel && (
                <Button
                  variant="danger-ghost"
                  size="sm"
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    onCancel(model.id);
                  }}
                  aria-label={t("modelSelector.cancelDownload")}
                >
                  {t("modelSelector.cancel")}
                </Button>
              )}
            </div>
          </div>
        </div>
      )}
      {status === "verifying" && (
        <div className="w-full mt-3">
          <div className="w-full h-1.5 bg-mid-gray/20 rounded-full overflow-hidden">
            <div className="h-full bg-logo-primary rounded-full animate-pulse w-full" />
          </div>
          <p className="text-xs text-text/50 mt-1">
            {t("modelSelector.verifyingGeneric")}
          </p>
        </div>
      )}
      {status === "extracting" && (
        <div className="w-full mt-3">
          <div className="w-full h-1.5 bg-mid-gray/20 rounded-full overflow-hidden">
            <div className="h-full bg-logo-primary rounded-full animate-pulse w-full" />
          </div>
          <p className="text-xs text-text/50 mt-1">
            {t("modelSelector.extractingGeneric")}
          </p>
        </div>
      )}
    </div>
  );
};

export default ModelCard;
