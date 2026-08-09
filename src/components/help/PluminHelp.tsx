import React, { useEffect, useMemo, useState } from "react";
import { HelpCircle, Loader2, Square, Volume2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { commands, type PluminHelpResponse } from "@/bindings";
import { navigateTo } from "@/lib/navigation";
import type { SidebarSection } from "@/components/Sidebar";
import { Button } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import { Textarea } from "@/components/ui/Textarea";
import { MicButton } from "@/components/shared/MicButton";
import { Plumin } from "@/components/shared/Plumin";
import { useSettings } from "@/hooks/useSettings";

const VALID_SECTIONS = new Set<SidebarSection>([
  "home",
  "general",
  "models",
  "advanced",
  "history",
  "studio",
  "interpreter",
  "translator",
  "conversation",
  "mcp",
  "postprocessing",
  "debug",
  "about",
]);

const isSidebarSection = (value: string): value is SidebarSection =>
  VALID_SECTIONS.has(value as SidebarSection);

export const PluminHelp: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting } = useSettings();
  const [open, setOpen] = useState(false);
  const [question, setQuestion] = useState("");
  const [response, setResponse] = useState<PluminHelpResponse | null>(null);
  const [busy, setBusy] = useState(false);
  const [speaking, setSpeaking] = useState(false);

  const answer = useMemo(() => {
    if (!response) return "";
    return (
      response.answer ??
      t(`pluminHelp.answers.${response.topic}`, {
        defaultValue: t("pluminHelp.answers.overview"),
      })
    );
  }, [response, t]);

  useEffect(() => {
    if (open) return;
    commands.conversationSpeakStop();
    window.speechSynthesis?.cancel();
    setSpeaking(false);
  }, [open]);

  const ask = async (event: React.FormEvent) => {
    event.preventDefault();
    const trimmed = question.trim();
    if (!trimmed || busy) return;
    setBusy(true);
    setResponse(null);
    try {
      const result = await commands.askPlumin(trimmed);
      if (result.status === "ok") setResponse(result.data);
    } catch (error) {
      console.error("Plumín help failed:", error);
    } finally {
      setBusy(false);
    }
  };

  const openRecommendedSection = () => {
    if (!response || !isSidebarSection(response.section)) return;
    navigateTo(response.section);
    setOpen(false);
  };

  const speak = async () => {
    if (!answer || speaking) return;
    setSpeaking(true);
    // El motor de voz sale de los ajustes, igual que en el resto de la app:
    // antes iba "auto" a fuego, así que quien eligió la voz del sistema
    // recibía la neural de todos modos. Se reusa el de "Tu tinta en voz" por
    // ser el mismo gesto: leer en voz alta un texto que está en pantalla.
    const engine =
      (getSetting("read_selection_voice_engine") as string | undefined) ??
      "system";
    let handled = false;
    try {
      handled = await commands.conversationSpeak(answer, engine);
    } catch {
      handled = false;
    }
    if (handled) {
      // La ruta nativa vuelve del await con la lectura ya terminada. Sin este
      // apagado, "Detener lectura" quedaba encendido para siempre: era el
      // único camino donde setSpeaking(true) no se revertía nunca.
      setSpeaking(false);
      return;
    }
    if ("speechSynthesis" in window) {
      const utterance = new SpeechSynthesisUtterance(answer);
      utterance.onend = () => setSpeaking(false);
      utterance.onerror = () => setSpeaking(false);
      window.speechSynthesis.speak(utterance);
      return;
    }
    // Sin ninguna ruta de voz disponible no hay lectura que detener.
    setSpeaking(false);
  };

  const stopSpeaking = () => {
    commands.conversationSpeakStop();
    window.speechSynthesis?.cancel();
    setSpeaking(false);
  };

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="flex items-center gap-1.5 rounded-md px-2 py-1 text-text/60 transition-colors hover:bg-mid-gray/10 hover:text-text focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
        aria-label={t("pluminHelp.open")}
      >
        <HelpCircle className="h-4 w-4" aria-hidden="true" />
        <span>{t("pluminHelp.open")}</span>
      </button>

      <Dialog
        open={open}
        onOpenChange={setOpen}
        title={t("pluminHelp.title")}
        description={t("pluminHelp.description")}
        closeLabel={t("common.close")}
        className="max-w-xl"
      >
        <div className="flex flex-col gap-4">
          <div className="flex items-start gap-3 rounded-lg bg-logo-primary/8 p-3">
            <Plumin pose={response ? "guia" : "escucha"} size={76} />
            <p className="pt-2 text-sm text-text">{t("pluminHelp.intro")}</p>
          </div>

          <form onSubmit={ask} className="flex flex-col gap-2">
            <label
              htmlFor="plumin-help-question"
              className="text-sm font-medium"
            >
              {t("pluminHelp.questionLabel")}
            </label>
            <div className="relative">
              <Textarea
                id="plumin-help-question"
                value={question}
                onChange={(event) =>
                  setQuestion(event.target.value.slice(0, 500))
                }
                placeholder={t("pluminHelp.placeholder")}
                rows={3}
                disabled={busy}
                className="w-full pe-11"
              />
              <div className="absolute end-2 top-2">
                <MicButton
                  disabled={busy}
                  title={t("pluminHelp.dictate")}
                  onText={(text) =>
                    setQuestion((current) =>
                      `${current}${current.trim() ? " " : ""}${text}`.slice(
                        0,
                        500,
                      ),
                    )
                  }
                />
              </div>
            </div>
            <div className="flex justify-end">
              <Button type="submit" disabled={busy || !question.trim()}>
                {busy ? (
                  <span className="flex items-center gap-2">
                    <Loader2
                      className="h-4 w-4 animate-spin"
                      aria-hidden="true"
                    />
                    {t("pluminHelp.thinking")}
                  </span>
                ) : (
                  t("pluminHelp.ask")
                )}
              </Button>
            </div>
          </form>

          {response && (
            <div
              role="status"
              aria-live="polite"
              className="flex flex-col gap-3 rounded-lg border border-logo-primary/25 bg-logo-primary/8 p-4"
            >
              <p className="whitespace-pre-wrap text-sm leading-relaxed text-text select-text">
                {answer}
              </p>
              <p className="text-xs text-mid-gray">
                {response.used_local_engine
                  ? t("pluminHelp.localAnswer")
                  : t("pluminHelp.guideFallback")}
              </p>
              <div className="flex flex-wrap gap-2">
                {isSidebarSection(response.section) && (
                  <Button size="sm" onClick={openRecommendedSection}>
                    {t("pluminHelp.openSection")}
                  </Button>
                )}
                {speaking ? (
                  <Button size="sm" variant="secondary" onClick={stopSpeaking}>
                    <Square
                      className="me-1 inline h-3.5 w-3.5"
                      aria-hidden="true"
                    />
                    {t("pluminHelp.stopSpeaking")}
                  </Button>
                ) : (
                  <Button size="sm" variant="secondary" onClick={speak}>
                    <Volume2
                      className="me-1 inline h-4 w-4"
                      aria-hidden="true"
                    />
                    {t("pluminHelp.speak")}
                  </Button>
                )}
              </div>
            </div>
          )}
        </div>
      </Dialog>
    </>
  );
};
