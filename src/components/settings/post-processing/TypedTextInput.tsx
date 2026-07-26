import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Button } from "../../ui/Button";
import { Textarea } from "../../ui/Textarea";
import { commands, type TypedTextAction } from "@/bindings";

/**
 * Alternativa por teclado al dictado.
 *
 * Toda la corrección, traducción y tonos de Escriba pasaban por hablar. Alguien
 * que en ese momento no pueda usar la voz (una oficina compartida, afonía, una
 * discapacidad del habla) no tenía forma de usar el motor de IA local que la app
 * ya tiene instalado, aunque la infraestructura estuviera entera.
 *
 * El resultado se muestra y se copia en vez de pegarse: con la ventana principal
 * enfocada, la aplicación de destino ya perdió el foco, así que pegar iría al
 * sitio equivocado.
 */
export const TypedTextInput: React.FC = () => {
  const { t } = useTranslation();
  const [input, setInput] = useState("");
  const [output, setOutput] = useState("");
  // Guarda cuál de las dos acciones está corriendo, no solo que algo corre:
  // así el botón pulsado muestra "Procesando…" y el otro queda deshabilitado,
  // en vez de que ambos digan lo mismo y no se sepa cuál se pidió.
  const [busy, setBusy] = useState<TypedTextAction | null>(null);

  const run = async (action: TypedTextAction) => {
    if (!input.trim() || busy) return;
    setBusy(action);
    setOutput("");
    try {
      const result = await commands.processTypedText(input, action);
      if (result.status === "ok") {
        setOutput(result.data);
      } else if (result.error === "POST_PROCESS_DISABLED") {
        toast.warning(t("settings.typedText.disabled"));
      } else {
        toast.error(t("settings.typedText.failed"));
      }
    } catch {
      toast.error(t("settings.typedText.failed"));
    } finally {
      setBusy(null);
    }
  };

  const copy = async () => {
    await writeText(output);
    toast.success(t("settings.typedText.copied"));
  };

  return (
    <div className="flex flex-col gap-3 rounded-lg border border-mid-gray/20 px-4 py-3">
      <div>
        <h3 className="text-sm font-medium">{t("settings.typedText.title")}</h3>
        <p className="mt-1 text-sm text-mid-gray">
          {t("settings.typedText.description")}
        </p>
      </div>

      <Textarea
        value={input}
        onChange={(e) => setInput(e.target.value)}
        placeholder={t("settings.typedText.placeholder")}
        rows={4}
        aria-label={t("settings.typedText.inputLabel")}
      />

      <div className="flex flex-wrap items-center gap-2">
        <Button
          variant="primary"
          size="sm"
          onClick={() => run("correct")}
          disabled={busy !== null || !input.trim()}
        >
          {busy === "correct"
            ? t("settings.typedText.working")
            : t("settings.typedText.run")}
        </Button>
        <Button
          variant="secondary"
          size="sm"
          onClick={() => run("translate")}
          disabled={busy !== null || !input.trim()}
        >
          {busy === "translate"
            ? t("settings.typedText.working")
            : t("settings.typedText.translate")}
        </Button>
      </div>

      {output && (
        <div className="flex flex-col gap-2">
          {/* aria-live: el resultado llega de forma asíncrona, así que hay que
              anunciarlo o quien no ve la pantalla no sabe que ya está. */}
          <div
            role="status"
            aria-live="polite"
            className="whitespace-pre-wrap rounded-lg bg-mid-gray/10 p-3 text-sm text-text select-text"
          >
            {output}
          </div>
          <div>
            <Button variant="secondary" size="sm" onClick={copy}>
              {t("settings.typedText.copy")}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
};

TypedTextInput.displayName = "TypedTextInput";
