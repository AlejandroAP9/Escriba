import React, { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Code2, Feather, Heart, House, Scale } from "lucide-react";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SettingContainer } from "../../ui/SettingContainer";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { AppDataDirectory } from "../AppDataDirectory";
import { AppLanguageSelector } from "../AppLanguageSelector";
import { ShowWhatsNewOnUpdate } from "../ShowWhatsNewOnUpdate";
import { LogDirectory } from "../debug";
import { useModelStore } from "../../../stores/modelStore";

const BRAND = "Escriba";
const EMPTY = "";
// Tecnologías reales sobre las que corre Escriba (nombres propios, no traducibles).
const TECH = [
  "Tauri",
  "whisper.cpp",
  "ggml",
  "Parakeet",
  "Silero VAD",
  "Whisper",
  "React",
];

export const AboutSettings: React.FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");
  const { models } = useModelStore();
  const modelCount = models.length;

  useEffect(() => {
    const fetchVersion = async () => {
      try {
        const appVersion = await getVersion();
        setVersion(appVersion);
      } catch (error) {
        console.error("Failed to get app version:", error);
        setVersion("0.1.2");
      }
    };

    fetchVersion();
  }, []);

  const handleDonateClick = async () => {
    try {
      await openUrl("https://handy.computer/donate");
    } catch (error) {
      console.error("Failed to open donate link:", error);
    }
  };

  const badges = [
    { icon: House, label: t("settings.general.hero.local") },
    { icon: Code2, label: t("settings.about.identity.openSource") },
    { icon: Scale, label: t("settings.about.identity.mit") },
  ];

  const numbers = [
    { value: modelCount > 0 ? String(modelCount) : "—", label: t("settings.about.numbers.models") },
    { value: "100%", label: t("settings.about.numbers.local") },
    { value: "0", label: t("settings.about.numbers.telemetry") },
    { value: "0", label: t("settings.about.numbers.servers") },
    { value: "MIT", label: t("settings.about.numbers.license") },
  ];

  return (
    <div className="mx-auto w-full max-w-3xl space-y-6 py-2">
      {/* Tarjeta de identidad: el ADN de Escriba en 10 segundos. */}
      <Card
        variant="hero"
        className="relative overflow-hidden px-6 py-8 text-center"
        style={{
          background:
            "linear-gradient(135deg, var(--color-background), var(--color-vitela))",
        }}
      >
        <Feather
          className="pointer-events-none absolute -right-6 -top-6 text-logo-primary/10"
          width={140}
          height={140}
          strokeWidth={1}
        />
        <div
          className="text-4xl text-text"
          style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
        >
          {BRAND}
        </div>
        <div
          className="mt-1 text-lg italic text-logo-primary"
          style={{ fontFamily: "var(--font-serif)" }}
        >
          {t("settings.about.identity.tagline")}
        </div>
        <p className="mx-auto mt-3 max-w-sm text-sm text-mid-gray">
          {t("settings.about.identity.pitch")}
        </p>
        <div className="mt-5 flex flex-wrap justify-center gap-2">
          {badges.map(({ icon: Icon, label }) => (
            <span
              key={label}
              className="flex items-center gap-1.5 rounded-full border border-logo-primary/30 bg-logo-primary/5 px-3 py-1 text-[11px] font-medium uppercase tracking-wider text-logo-primary"
            >
              <Icon width={12} height={12} />
              {label}
            </span>
          ))}
        </div>
        {/* eslint-disable-next-line i18next/no-literal-string */}
        <div className="mt-4 font-mono text-xs text-mid-gray">v{version}</div>
      </Card>

      {/* Escriba en números: resumen del producto, con datos reales. */}
      <div>
        <p className="mb-2 font-mono text-[10px] font-semibold uppercase tracking-[0.14em] text-mid-gray">
          {t("settings.about.numbers.title")}
        </p>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-5">
          {numbers.map((n) => (
            <Card key={n.label} variant="metric" className="p-3">
              <p
                className="text-2xl text-text"
                style={{ fontFamily: "var(--font-serif)", fontWeight: 600 }}
              >
                {n.value}
              </p>
              <p className="mt-0.5 text-[11px] leading-tight text-mid-gray">
                {n.label}
              </p>
            </Card>
          ))}
        </div>
      </div>

      <SettingsGroup title={t("settings.about.title")}>
        <AppLanguageSelector descriptionMode="tooltip" grouped={true} />
        <SettingContainer
          title={t("settings.about.version.title")}
          description={t("settings.about.version.description")}
          grouped={true}
        >
          {/* eslint-disable-next-line i18next/no-literal-string */}
          <span className="text-sm font-mono">v{version}</span>
        </SettingContainer>

        <ShowWhatsNewOnUpdate descriptionMode="tooltip" grouped={true} />

        <SettingContainer
          title={t("settings.about.sourceCode.title")}
          description={t("settings.about.sourceCode.description")}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="md"
            onClick={() => openUrl("https://github.com/AlejandroAP9/Escriba")}
          >
            {t("settings.about.sourceCode.button")}
          </Button>
        </SettingContainer>

        <AppDataDirectory descriptionMode="tooltip" grouped={true} />
        <LogDirectory grouped={true} />
      </SettingsGroup>

      {/* Donación reencuadrada: apoyar un proyecto abierto, no una compra. */}
      <div className="flex flex-wrap items-center justify-between gap-4 rounded-card border border-lacre/25 bg-lacre/5 p-5">
        <div className="min-w-0 flex-1">
          <p className="flex items-center gap-2 text-sm font-semibold text-text">
            <Heart width={15} height={15} className="text-lacre" />
            {t("settings.about.support.title")}
          </p>
          <p className="mt-1 text-xs leading-relaxed text-mid-gray">
            {t("settings.about.support.body")}
          </p>
        </div>
        <Button variant="primary" size="md" onClick={handleDonateClick}>
          {t("settings.about.supportDevelopment.button")}
        </Button>
      </div>

      {/* Reconocimientos: tecnologías reales + crédito a la base (Handy). */}
      <SettingsGroup title={t("settings.about.acknowledgments.title")}>
        <SettingContainer
          title={t("settings.about.acknowledgments.ggml.title")}
          description={t("settings.about.acknowledgments.ggml.description")}
          grouped={true}
          layout="stacked"
        >
          <div className="text-sm text-mid-gray">
            {t("settings.about.acknowledgments.ggml.details")}
          </div>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.tech.title")}
          description={EMPTY}
          grouped={true}
          layout="stacked"
        >
          <div className="flex flex-wrap gap-1.5">
            {TECH.map((name) => (
              <span
                key={name}
                className="rounded-md border border-mid-gray/20 bg-mid-gray/5 px-2 py-0.5 font-mono text-[11px] text-mid-gray"
              >
                {name}
              </span>
            ))}
          </div>
        </SettingContainer>

        <SettingContainer
          title={t("settings.about.tech.basedOn")}
          description={EMPTY}
          grouped={true}
        >
          <Button
            variant="secondary"
            size="sm"
            onClick={() => openUrl("https://github.com/cjpais/Handy")}
          >
            {t("settings.about.tech.basedOnButton")}
          </Button>
        </SettingContainer>
      </SettingsGroup>

      {/* Cierre emocional. */}
      <p
        className="pb-4 pt-2 text-center text-lg italic text-mid-gray"
        style={{ fontFamily: "var(--font-serif)" }}
      >
        {t("settings.about.closing")}
      </p>
    </div>
  );
};
