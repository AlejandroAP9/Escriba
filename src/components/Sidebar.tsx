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
  MessageCircle,
  Bot,
  SlidersHorizontal,
  Feather,
} from "lucide-react";
import EscribaLogo from "./icons/EscribaLogo";
import { HomeScreen } from "./home/HomeScreen";
import { StudioSettings } from "./studio/StudioSettings";
import { InterpreterSettings } from "./interpreter/InterpreterSettings";
import { TranslatorSettings } from "./translator/TranslatorSettings";
import { ConversationSettings } from "./conversation/ConversationSettings";
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
  home: {
    labelKey: "sidebar.home",
    icon: Feather,
    component: HomeScreen,
    enabled: () => true,
  },
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
  conversation: {
    labelKey: "sidebar.conversation",
    icon: MessageCircle,
    component: ConversationSettings,
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

// Navegación agrupada por contexto (no una lista plana): la app tiene sentido
// de un vistazo. Inicio arriba sin encabezado; el resto en secciones.
const NAV_GROUPS: { labelKey?: string; ids: SidebarSection[] }[] = [
  { ids: ["home"] },
  { labelKey: "sidebar.groupDictation", ids: ["models", "history", "studio"] },
  {
    labelKey: "sidebar.groupTools",
    ids: ["conversation", "translator", "interpreter"],
  },
  { labelKey: "sidebar.groupDevelopers", ids: ["mcp", "postprocessing"] },
  {
    labelKey: "sidebar.groupConfig",
    ids: ["general", "advanced", "debug", "about"],
  },
];

export const Sidebar: React.FC<SidebarProps> = ({
  activeSection,
  onSectionChange,
}) => {
  const { t } = useTranslation();
  const { settings } = useSettings();

  const isEnabled = (id: SidebarSection) =>
    SECTIONS_CONFIG[id]?.enabled(settings) ?? false;

  return (
    <div
      className="flex flex-col w-56 h-full items-center px-3 text-ink-fg overflow-y-auto"
      style={{
        // Tapa de libro: gradiente vertical muy leve, borde e highlight interior.
        background: "linear-gradient(180deg, #21192f 0%, var(--color-ink) 60%)",
        borderRight: "1px solid rgba(255,240,210,0.10)",
        boxShadow:
          "inset 0 1px 0 rgba(255,255,255,0.04), inset -12px 0 24px -18px rgba(0,0,0,0.6)",
      }}
    >
      <EscribaLogo
        width={168}
        className="mt-10 mb-6 shrink-0"
        onDark
        tagline
        liveWave
      />
      {/*
        Navegación principal. Las secciones eran `<div onClick>` sin role, sin
        tabIndex y sin onKeyDown: no había ninguna forma de cambiar de sección
        con el teclado, ni de que un lector de pantalla supiera que esto es la
        navegación. Son `<button>` dentro de un `<nav>`, y la activa lleva
        `aria-current` para que se anuncie como tal.
      */}
      <nav
        aria-label={t("sidebar.navLabel")}
        className="flex w-full flex-col gap-4 border-t border-white/10 pb-6 pt-4"
      >
        {NAV_GROUPS.map((group, gi) => {
          const ids = group.ids.filter(isEnabled);
          if (ids.length === 0) return null;
          const groupLabelId = group.labelKey ? `nav-group-${gi}` : undefined;
          return (
            <div
              key={gi}
              className="flex w-full flex-col gap-1"
              role="group"
              aria-labelledby={groupLabelId}
            >
              {group.labelKey && (
                <p
                  id={groupLabelId}
                  className="px-3 pb-1 text-[10px] font-semibold uppercase tracking-[0.14em] text-ink-fg/55"
                >
                  {t(group.labelKey)}
                </p>
              )}
              {ids.map((id) => {
                const config = SECTIONS_CONFIG[id];
                const Icon = config.icon;
                const isActive = activeSection === id;
                return (
                  <button
                    key={id}
                    type="button"
                    aria-current={isActive ? "page" : undefined}
                    className={`flex gap-2.5 items-center px-3 py-2 w-full rounded-lg cursor-pointer text-start transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary focus-visible:ring-offset-2 focus-visible:ring-offset-ink ${
                      isActive
                        ? "bg-logo-primary text-ink font-semibold shadow-sm"
                        : "text-ink-fg/80 hover:bg-white/10 hover:text-ink-fg"
                    }`}
                    onClick={() => onSectionChange(id)}
                  >
                    <Icon
                      width={20}
                      height={20}
                      className="shrink-0"
                      aria-hidden="true"
                    />
                    <span
                      className="text-sm font-medium truncate"
                      title={t(config.labelKey)}
                    >
                      {t(config.labelKey)}
                    </span>
                  </button>
                );
              })}
            </div>
          );
        })}
      </nav>
    </div>
  );
};
