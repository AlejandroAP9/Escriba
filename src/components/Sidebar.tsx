import React from "react";
import { useTranslation } from "react-i18next";
import {
  Cog,
  FlaskConical,
  History,
  Info,
  Sparkles,
  Cpu,
  FileAudio,
  Radio,
  Languages,
  Bot,
  SlidersHorizontal,
} from "lucide-react";
import EscribaLogo from "./icons/EscribaLogo";
import { StudioSettings } from "./studio/StudioSettings";
import { InterpreterSettings } from "./interpreter/InterpreterSettings";
import { TranslatorSettings } from "./translator/TranslatorSettings";
import { McpSettings } from "./mcp/McpSettings";
import { useSettings } from "../hooks/useSettings";
import {
  GeneralSettings,
  AdvancedSettings,
  HistorySettings,
  DebugSettings,
  AboutSettings,
  PostProcessingSettings,
  ModelsSettings,
} from "./settings";

export type SidebarSection = keyof typeof SECTIONS_CONFIG;

interface IconProps {
  width?: number | string;
  height?: number | string;
  size?: number | string;
  className?: string;
  [key: string]: any;
}

interface SectionConfig {
  labelKey: string;
  icon: React.ComponentType<IconProps>;
  component: React.ComponentType;
  enabled: (settings: any) => boolean;
}

export const SECTIONS_CONFIG = {
  general: {
    labelKey: "sidebar.general",
    icon: SlidersHorizontal,
    component: GeneralSettings,
    enabled: () => true,
  },
  models: {
    labelKey: "sidebar.models",
    icon: Cpu,
    component: ModelsSettings,
    enabled: () => true,
  },
  advanced: {
    labelKey: "sidebar.advanced",
    icon: Cog,
    component: AdvancedSettings,
    enabled: () => true,
  },
  history: {
    labelKey: "sidebar.history",
    icon: History,
    component: HistorySettings,
    enabled: () => true,
  },
  studio: {
    labelKey: "sidebar.studio",
    icon: FileAudio,
    component: StudioSettings,
    enabled: () => true,
  },
  interpreter: {
    labelKey: "sidebar.interpreter",
    icon: Radio,
    component: InterpreterSettings,
    enabled: () => true,
  },
  translator: {
    labelKey: "sidebar.translator",
    icon: Languages,
    component: TranslatorSettings,
    enabled: () => true,
  },
  mcp: {
    labelKey: "sidebar.mcp",
    icon: Bot,
    component: McpSettings,
    enabled: () => true,
  },
  postprocessing: {
    labelKey: "sidebar.postProcessing",
    icon: Sparkles,
    component: PostProcessingSettings,
    // Escriba: la correccion con IA es feature principal, no experimental.
    enabled: () => true,
  },
  debug: {
    labelKey: "sidebar.debug",
    icon: FlaskConical,
    component: DebugSettings,
    enabled: (settings) => settings?.debug_mode ?? false,
  },
  about: {
    labelKey: "sidebar.about",
    icon: Info,
    component: AboutSettings,
    enabled: () => true,
  },
} as const satisfies Record<string, SectionConfig>;

interface SidebarProps {
  activeSection: SidebarSection;
  onSectionChange: (section: SidebarSection) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const availableSections = Object.entries(SECTIONS_CONFIG)
    .filter(([_, config]) => config.enabled(settings))
    .map(([id, config]) => ({ id: id as SidebarSection, ...config }));

  return (
    <div className="flex flex-col w-56 h-full items-center px-3 bg-ink text-ink-fg border-e border-logo-primary/25">
      <EscribaLogo width={132} className="mt-7 mb-6" onDark tagline />
      <div className="flex flex-col w-full items-center gap-1 pt-4 border-t border-white/10">
        {availableSections.map((section) => {
          const Icon = section.icon;
          const isActive = activeSection === section.id;

          return (
            <div
              key={section.id}
              className={`flex gap-2.5 items-center px-3 py-2.5 w-full rounded-lg cursor-pointer transition-colors ${
                isActive
                  ? "bg-logo-primary text-ink font-semibold shadow-sm"
                  : "text-ink-fg/80 hover:bg-white/10 hover:text-ink-fg"
              }`}
              onClick={() => onSectionChange(section.id)}
            >
              <Icon width={24} height={24} className="shrink-0" />
              <p
                className="text-sm font-medium truncate"
                title={t(section.labelKey)}
              >
                {t(section.labelKey)}
              </p>
            </div>
          );
        })}
      </div>
    </div>
  );
};
