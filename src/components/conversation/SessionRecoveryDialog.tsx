import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ask } from "@tauri-apps/plugin-dialog";
import { FileDown, RotateCcw, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { commands, type ResumenPendiente } from "@/bindings";
import { navigateTo } from "@/lib/navigation";
import { requestObsidianExport } from "@/stores/obsidianStore";
import { Button } from "@/components/ui/Button";
import { Dialog } from "@/components/ui/Dialog";
import { Plumin } from "@/components/shared/Plumin";

/**
 * Recuperación de sesiones (PRP-009, Fase 2).
 *
 * Se monta UNA vez en App.tsx (mismo patrón que ObsidianPreviewDialog): al
 * arrancar pregunta al backend por journals sin cierre y, si hay, ofrece las
 * tres salidas del PRP. Recuperar repuebla la sesión en el backend y navega:
 * el efecto de reconexión de ConversationSettings pinta el resto solo.
 */
export const SessionRecoveryDialog: React.FC = () => {
  const { t } = useTranslation();
  const [pending, setPending] = useState<ResumenPendiente[]>([]);
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    commands.sessionRecoveryList().then((list) => {
      if (list.length > 0) {
        setPending(list);
        setOpen(true);
      }
    });
  }, []);

  const dropFromList = (id: string) => {
    setPending((prev) => {
      const next = prev.filter((p) => p.id !== id);
      if (next.length === 0) setOpen(false);
      return next;
    });
  };

  const formatDuration = (ms: number) => {
    const total = Math.floor(ms / 1000);
    const mm = Math.floor(total / 60);
    const ss = total % 60;
    return `${mm}:${ss.toString().padStart(2, "0")}`;
  };

  const modeLabel = (modo: string) =>
    t(`conversation.variant.${modo}`, { defaultValue: modo });

  const recover = async (id: string) => {
    setBusy(true);
    try {
      const r = await commands.sessionRecover(id);
      if (r.status === "ok") {
        toast.success(t("recovery.recovered"));
        setOpen(false);
        navigateTo("conversation");
      } else {
        toast.error(t("recovery.error"), { description: r.error });
      }
    } finally {
      setBusy(false);
    }
  };

  const exportDoc = async (session: ResumenPendiente) => {
    const r = await commands.sessionRecoveryDoc(session.id);
    if (r.status !== "ok") {
      toast.error(t("recovery.error"), { description: r.error });
      return;
    }
    const firstLine =
      r.data.text
        .split("\n")
        .map((l) => l.trim())
        .find((l) => l.length > 0) || modeLabel(session.modo);
    const title = firstLine.replace(/^#+\s*/, "").slice(0, 80);
    // Si el usuario cancela el export, la sesión sigue pendiente y volverá a
    // ofrecerse en el próximo arranque: no se pierde nada por cerrar aquí.
    setOpen(false);
    requestObsidianExport(title, r.data.text, () => {
      void commands.sessionRecoveryConfirm(session.id);
    });
  };

  const discard = async (id: string) => {
    const confirmed = await ask(t("recovery.discardConfirm"), {
      title: t("recovery.discardTitle"),
      kind: "warning",
    });
    if (!confirmed) return;
    const r = await commands.sessionRecoveryDiscard(id);
    if (r.status === "ok") {
      dropFromList(id);
    } else {
      toast.error(t("recovery.error"), { description: r.error });
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        // Cerrar sin decidir no descarta nada: vuelve al próximo arranque.
        if (!next && !busy) setOpen(false);
      }}
      title={t("recovery.title")}
      description={t("recovery.subtitle")}
      closeLabel={t("common.close")}
      className="max-w-lg"
    >
      <div className="flex flex-col gap-4">
        <div className="flex justify-center">
          <Plumin pose="guia" size={76} />
        </div>
        {pending.map((session) => (
          <div
            key={session.id}
            className="flex flex-col gap-3 rounded-lg border border-line bg-background p-4"
          >
            <div className="flex items-baseline justify-between gap-2">
              <p className="text-sm font-semibold text-text">
                {modeLabel(session.modo)}
              </p>
              <p className="text-xs text-mid-gray">
                {new Date(session.wall_ms).toLocaleString()}
              </p>
            </div>
            <p className="text-xs text-mid-gray">
              {t("recovery.summary", {
                turns: session.turnos,
                duration: formatDuration(session.duracion_ms),
              })}
              {session.tiene_documento && <> · {t("recovery.hasDoc")}</>}
            </p>
            {session.cola_rota && (
              <p className="text-3xs leading-relaxed text-mid-gray">
                {t("recovery.brokenTail")}
              </p>
            )}
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                disabled={busy}
                onClick={() => recover(session.id)}
              >
                <RotateCcw
                  className="me-1 inline h-3.5 w-3.5"
                  aria-hidden="true"
                />
                {t("recovery.recover")}
              </Button>
              {session.tiene_documento && (
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={busy}
                  onClick={() => exportDoc(session)}
                >
                  <FileDown
                    className="me-1 inline h-3.5 w-3.5"
                    aria-hidden="true"
                  />
                  {t("recovery.export")}
                </Button>
              )}
              <Button
                size="sm"
                variant="danger-ghost"
                disabled={busy}
                onClick={() => discard(session.id)}
              >
                <Trash2
                  className="me-1 inline h-3.5 w-3.5"
                  aria-hidden="true"
                />
                {t("recovery.discard")}
              </Button>
            </div>
          </div>
        ))}
      </div>
    </Dialog>
  );
};
