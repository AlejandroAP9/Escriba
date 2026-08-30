import React, { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, X } from "lucide-react";
import type { TextReplacement } from "@/bindings";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { Button } from "../ui/Button";
import { SettingContainer } from "../ui/SettingContainer";

interface TextReplacementsProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/**
 * Editor de reglas deterministas de buscar/reemplazar que se aplican al texto
 * final tras dictar (después del post-proceso con IA). Cada regla puede ser
 * texto literal o expresión regular.
 */
export const TextReplacements: React.FC<TextReplacementsProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting } = useSettings();
    const [rules, setRules] = useState<TextReplacement[]>(
      () => getSetting("text_replacements") || [],
    );

    // Antes esto solo se guardaba en el onBlur de cada campo: si escribias una
    // regla y te ibas a otra app a probarla sin hacer clic fuera, la regla no
    // existia (reporte de Juan Francisco Ceccarelli, comunidad, 12-ago-2026).
    // Ahora se guarda mientras escribes, con un respiro para no escribir en
    // disco en cada tecla, y se vuelca al desmontar para no perder lo ultimo.
    const pendiente = useRef<TextReplacement[] | null>(null);
    const temporizador = useRef<ReturnType<typeof setTimeout> | null>(null);

    const persist = useCallback(
      (next: TextReplacement[]) => {
        if (temporizador.current) {
          clearTimeout(temporizador.current);
          temporizador.current = null;
        }
        pendiente.current = null;
        updateSetting("text_replacements", next);
      },
      [updateSetting],
    );

    const persistPronto = useCallback(
      (next: TextReplacement[]) => {
        pendiente.current = next;
        if (temporizador.current) clearTimeout(temporizador.current);
        temporizador.current = setTimeout(() => {
          temporizador.current = null;
          const listo = pendiente.current;
          pendiente.current = null;
          if (listo) updateSetting("text_replacements", listo);
        }, 400);
      },
      [updateSetting],
    );

    // Cerrar Ajustes a media palabra tampoco puede perder la regla.
    useEffect(() => {
      return () => {
        if (temporizador.current) clearTimeout(temporizador.current);
        if (pendiente.current) {
          updateSetting("text_replacements", pendiente.current);
          pendiente.current = null;
        }
      };
    }, [updateSetting]);

    const addRule = () => {
      const next = [...rules, { find: "", replace: "", is_regex: false }];
      setRules(next);
      persist(next);
    };

    const removeRule = (index: number) => {
      const next = rules.filter((_, i) => i !== index);
      setRules(next);
      persist(next);
    };

    const editField = (
      index: number,
      field: keyof TextReplacement,
      value: string | boolean,
    ) => {
      const next = rules.map((r, i) =>
        i === index ? { ...r, [field]: value } : r,
      );
      setRules(next);
      persistPronto(next);
    };

    const toggleRegex = (index: number) => {
      const next = rules.map((r, i) =>
        i === index ? { ...r, is_regex: !r.is_regex } : r,
      );
      setRules(next);
      persist(next);
    };

    return (
      <SettingContainer
        title={t("settings.advanced.textReplacements.title")}
        description={t("settings.advanced.textReplacements.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        layout="stacked"
      >
        <div className="flex flex-col gap-2 w-full">
          {rules.map((rule, index) => (
            <div key={index} className="flex items-center gap-2">
              <Input
                type="text"
                variant="compact"
                className="flex-1"
                value={rule.find}
                onChange={(e) => editField(index, "find", e.target.value)}
                onBlur={() => persist(rules)}
                placeholder={t("settings.advanced.textReplacements.find")}
              />
              <span className="text-mid-gray shrink-0" aria-hidden>
                {"→"}
              </span>
              <Input
                type="text"
                variant="compact"
                className="flex-1"
                value={rule.replace}
                onChange={(e) => editField(index, "replace", e.target.value)}
                onBlur={() => persist(rules)}
                placeholder={t("settings.advanced.textReplacements.replace")}
              />
              <label
                className="flex items-center gap-1 text-xs text-mid-gray cursor-pointer shrink-0"
                title={t("settings.advanced.textReplacements.regex")}
              >
                <input
                  type="checkbox"
                  checked={rule.is_regex}
                  onChange={() => toggleRegex(index)}
                />
                <span className="font-mono">{".*"}</span>
              </label>
              <button
                type="button"
                onClick={() => removeRule(index)}
                title={t("settings.advanced.textReplacements.remove")}
                className="shrink-0 p-1 rounded-md text-mid-gray hover:text-text"
              >
                <X width={16} height={16} />
              </button>
            </div>
          ))}
          <Button
            onClick={addRule}
            variant="secondary"
            size="sm"
            className="self-start inline-flex items-center gap-1"
          >
            <Plus width={14} height={14} />
            {t("settings.advanced.textReplacements.add")}
          </Button>
        </div>
      </SettingContainer>
    );
  },
);

TextReplacements.displayName = "TextReplacements";
