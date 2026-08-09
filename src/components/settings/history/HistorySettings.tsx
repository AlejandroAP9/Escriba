import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { confirm as confirmDialog } from "@tauri-apps/plugin-dialog";
import {
  Check,
  Copy,
  FolderOpen,
  Inbox,
  Search,
  SearchX,
  Star,
  Trash2,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  commands,
  events,
  type HistoryEntry,
  type HistoryUpdatePayload,
} from "@/bindings";
import { RetranscribeMenu } from "../../shared/RetranscribeMenu";
import { useSettings } from "@/hooks/useSettings";
import { AudioPlayer } from "../../ui/AudioPlayer";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { SaveAudioRecordings } from "../SaveAudioRecordings";
import { HistoryLimit } from "../HistoryLimit";
import { RecordingRetentionPeriodSelector } from "../RecordingRetentionPeriod";
import { EmptyState, LoadingState } from "../../ui/EmptyState";

const IconButton: React.FC<{
  onClick: () => void;
  title: string;
  disabled?: boolean;
  active?: boolean;
  children: React.ReactNode;
}> = ({ onClick, title, disabled, active, children }) => (
  <button
    onClick={onClick}
    disabled={disabled}
    className={`p-1.5 rounded-md flex items-center justify-center transition-colors cursor-pointer disabled:cursor-not-allowed disabled:text-text/20 ${
      active
        ? "text-gold-text hover:text-gold-text/80"
        : "text-mid-gray hover:text-gold-text"
    }`}
    title={title}
  >
    {children}
  </button>
);

const PAGE_SIZE = 30;

// Umbral de recorte: sobre esto, la tarjeta muestra 3 líneas + "Ver más".
const PREVIEW_CHAR_LIMIT = 280;

/** Texto que Escriba realmente pegó. El original se conserva aparte para
 * auditoría y comparación cuando hubo una transformación. */
const finalEntryText = (entry: HistoryEntry) =>
  entry.post_processed_text?.trim()
    ? entry.post_processed_text
    : entry.transcription_text;

// Tiempo relativo localizado ("hace 14 minutos") sin cadenas por idioma.
function relativeTime(tsSeconds: number, locale: string): string {
  const diffSec = Math.round((tsSeconds * 1000 - Date.now()) / 1000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  const abs = Math.abs(diffSec);
  if (abs < 60) return rtf.format(Math.round(diffSec), "second");
  if (abs < 3600) return rtf.format(Math.round(diffSec / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(diffSec / 3600), "hour");
  return rtf.format(Math.round(diffSec / 86400), "day");
}

export const HistorySettings: React.FC = () => {
  const { t, i18n } = useTranslation();
  // Bandeja de entrada diaria (PRP-007): la acción por entrada solo existe
  // con el ajuste encendido. Es la vía rápida a propósito (sin diálogo); la
  // vía con revisión sigue siendo el export normal.
  const { getSetting } = useSettings();
  const inboxActivo = (getSetting("obsidian_daily_inbox") ?? false) as boolean;
  const sendToInbox = async (texto: string) => {
    const r = await commands.appendToObsidianInbox(texto);
    if (r.status === "ok") {
      toast.success(t("obsidian.inboxSaved"));
    } else if (r.error === "SIN_VAULT") {
      toast.warning(t("obsidian.inboxNoVault"));
    } else {
      toast.error(t("obsidian.inboxFailed"));
    }
  };
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasMore, setHasMore] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");
  const [filter, setFilter] = useState<"all" | "saved">("all");
  const sentinelRef = useRef<HTMLDivElement>(null);
  const entriesRef = useRef<HistoryEntry[]>([]);
  const loadingRef = useRef(false);

  // Keep ref in sync for use in IntersectionObserver callback
  useEffect(() => {
    entriesRef.current = entries;
  }, [entries]);

  const loadPage = useCallback(async (cursor?: number) => {
    const isFirstPage = cursor === undefined;
    if (!isFirstPage && loadingRef.current) return;
    loadingRef.current = true;

    if (isFirstPage) setLoading(true);

    try {
      const result = await commands.getHistoryEntries(
        cursor ?? null,
        PAGE_SIZE,
      );
      if (result.status === "ok") {
        const { entries: newEntries, has_more } = result.data;
        setEntries((prev) =>
          isFirstPage ? newEntries : [...prev, ...newEntries],
        );
        setHasMore(has_more);
      }
    } catch (error) {
      console.error("Failed to load history entries:", error);
    } finally {
      setLoading(false);
      loadingRef.current = false;
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadPage();
  }, [loadPage]);

  // Infinite scroll via IntersectionObserver
  useEffect(() => {
    if (loading) return;

    const sentinel = sentinelRef.current;
    if (!sentinel || !hasMore) return;

    const observer = new IntersectionObserver(
      (observerEntries) => {
        const first = observerEntries[0];
        if (first.isIntersecting) {
          const lastEntry = entriesRef.current[entriesRef.current.length - 1];
          if (lastEntry) {
            loadPage(lastEntry.id);
          }
        }
      },
      { threshold: 0 },
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loading, hasMore, loadPage]);

  // Listen for new entries added from the transcription pipeline
  useEffect(() => {
    const unlisten = events.historyUpdatePayload.listen((event) => {
      const payload: HistoryUpdatePayload = event.payload;
      if (payload.action === "added") {
        setEntries((prev) => [payload.entry, ...prev]);
      } else if (payload.action === "updated") {
        setEntries((prev) =>
          prev.map((e) => (e.id === payload.entry.id ? payload.entry : e)),
        );
      }
      // "deleted" and "toggled" are handled by optimistic updates only,
      // so we intentionally ignore them here to avoid double-mutation.
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggleSaved = async (id: number) => {
    // Optimistic update
    setEntries((prev) =>
      prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
    );
    try {
      const result = await commands.toggleHistoryEntrySaved(id);
      if (result.status !== "ok") {
        // Revert on failure
        setEntries((prev) =>
          prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
        );
      }
    } catch (error) {
      console.error("Failed to toggle saved status:", error);
      // Revert on failure
      setEntries((prev) =>
        prev.map((e) => (e.id === id ? { ...e, saved: !e.saved } : e)),
      );
    }
  };

  const copyToClipboard = async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      console.error("Failed to copy to clipboard:", error);
    }
  };

  const getAudioUrl = useCallback(async (fileName: string) => {
    try {
      const result = await commands.getAudioFilePath(fileName);
      if (result.status === "ok") {
        // El backend devuelve un protocolo privado, no una ruta de disco.
        // El elemento <audio> pide rangos y Rust descifra solo esos frames.
        return result.data;
      }
      return null;
    } catch (error) {
      console.error("Failed to get audio file path:", error);
      return null;
    }
  }, []);

  const deleteAudioEntry = async (id: number) => {
    // Optimistically remove
    setEntries((prev) => prev.filter((e) => e.id !== id));
    try {
      const result = await commands.deleteHistoryEntry(id);
      if (result.status !== "ok") {
        // Reload on failure
        loadPage();
      }
    } catch (error) {
      console.error("Failed to delete entry:", error);
      loadPage();
    }
  };

  const retryHistoryEntry = async (id: number, modelId: string | null) => {
    const result = await commands.retryHistoryEntryTranscription(id, modelId);
    if (result.status !== "ok") {
      throw new Error(String(result.error));
    }
  };

  const openRecordingsFolder = async () => {
    try {
      const result = await commands.openRecordingsFolder();
      if (result.status !== "ok") {
        throw new Error(String(result.error));
      }
    } catch (error) {
      console.error("Failed to open recordings folder:", error);
    }
  };

  // Filtro (texto + favoritos) sobre las entradas cargadas.
  const visibleEntries = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    return entries.filter((e) => {
      if (filter === "saved" && !e.saved) return false;
      if (
        q &&
        !e.transcription_text.toLowerCase().includes(q) &&
        !finalEntryText(e).toLowerCase().includes(q)
      )
        return false;
      return true;
    });
  }, [entries, searchQuery, filter]);

  // Agrupar por día: HOY / AYER / fecha. La unidad es la idea, no el archivo.
  const groups = useMemo(() => {
    const out: { key: string; label: string; items: HistoryEntry[] }[] = [];
    const now = new Date();
    const todayKey = now.toDateString();
    const yesterdayKey = new Date(
      now.getTime() - 24 * 60 * 60 * 1000,
    ).toDateString();
    const fmt = new Intl.DateTimeFormat(i18n.language, {
      day: "numeric",
      month: "long",
    });
    const fmtWithYear = new Intl.DateTimeFormat(i18n.language, {
      day: "numeric",
      month: "long",
      year: "numeric",
    });

    for (const entry of visibleEntries) {
      const d = new Date(entry.timestamp * 1000);
      const key = d.toDateString();
      let label: string;
      if (key === todayKey) label = t("settings.history.today");
      else if (key === yesterdayKey) label = t("settings.history.yesterday");
      else
        label = (
          d.getFullYear() === now.getFullYear() ? fmt : fmtWithYear
        ).format(d);

      const last = out[out.length - 1];
      if (last && last.key === key) last.items.push(entry);
      else out.push({ key, label, items: [entry] });
    }
    return out;
  }, [visibleEntries, i18n.language, t]);

  let content: React.ReactNode;

  if (loading) {
    content = <LoadingState label={t("settings.history.loading")} />;
  } else if (entries.length === 0) {
    content = (
      <EmptyState plumin="neutral" title={t("settings.history.empty")} />
    );
  } else if (visibleEntries.length === 0) {
    content = (
      <EmptyState icon={SearchX} title={t("settings.history.noResults")} />
    );
  } else {
    content = (
      <>
        <div className="space-y-5">
          {groups.map((group) => (
            <div key={group.key} className="space-y-2.5">
              <p className="px-1 font-mono text-3xs font-semibold uppercase tracking-[0.14em] text-mid-gray">
                {group.label}
              </p>
              {group.items.map((entry) => (
                <HistoryEntryComponent
                  key={entry.id}
                  entry={entry}
                  onToggleSaved={() => toggleSaved(entry.id)}
                  onCopyText={() => copyToClipboard(finalEntryText(entry))}
                  onSendToInbox={
                    inboxActivo
                      ? () => sendToInbox(finalEntryText(entry))
                      : undefined
                  }
                  getAudioUrl={getAudioUrl}
                  deleteAudio={deleteAudioEntry}
                  retryTranscription={retryHistoryEntry}
                />
              ))}
            </div>
          ))}
        </div>
        {/* Sentinel for infinite scroll */}
        <div ref={sentinelRef} className="h-1" />
      </>
    );
  }

  return (
    <div className="max-w-3xl w-full mx-auto space-y-4">
      {/* Encabezado: título + filtros + carpeta (discreta) */}
      <div className="flex items-center justify-between gap-2 px-1">
        <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
          {t("settings.history.title")}
        </h2>
        <div className="flex items-center gap-1.5">
          <div className="flex items-center rounded-lg bg-mid-gray/10 p-0.5 text-xs">
            <button
              onClick={() => setFilter("all")}
              className={`rounded-md px-2.5 py-1 transition-colors ${
                filter === "all"
                  ? "bg-background text-text shadow-sm"
                  : "text-mid-gray hover:text-text"
              }`}
            >
              {t("settings.history.filterAll")}
            </button>
            <button
              onClick={() => setFilter("saved")}
              className={`flex items-center gap-1 rounded-md px-2.5 py-1 transition-colors ${
                filter === "saved"
                  ? "bg-background text-text shadow-sm"
                  : "text-mid-gray hover:text-text"
              }`}
            >
              <Star width={11} height={11} />
              {t("settings.history.filterSaved")}
            </button>
          </div>
          <IconButton
            onClick={openRecordingsFolder}
            title={t("settings.history.openFolder")}
          >
            <FolderOpen width={16} height={16} />
          </IconButton>
        </div>
      </div>

      {/* Búsqueda: la pantalla es un historial de ideas; se busca por texto. */}
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-mid-gray pointer-events-none" />
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder={t("settings.history.searchPlaceholder")}
          className="w-full pl-9 pr-3 py-2 text-sm bg-background border border-mid-gray/50 rounded-lg shadow-[inset_0_1px_3px_rgba(27,20,38,0.06)] focus:outline-none focus:ring-1 focus:ring-logo-primary placeholder:text-mid-gray/70"
        />
      </div>

      {content}

      {/* Guardado: cuánto se conserva y si queda el audio. Vivía en Avanzado,
          pero quien decide cuánto historial guardar lo decide AQUÍ, mirándolo. */}
      <SettingsGroup title={t("settings.history.storageTitle")}>
        <SaveAudioRecordings descriptionMode="inline" grouped={true} />
        <HistoryLimit descriptionMode="inline" grouped={true} />
        <RecordingRetentionPeriodSelector
          descriptionMode="inline"
          grouped={true}
        />
      </SettingsGroup>
    </div>
  );
};

interface HistoryEntryProps {
  entry: HistoryEntry;
  onToggleSaved: () => void;
  onCopyText: () => void;
  /** Presente solo con la bandeja de entrada diaria activada. */
  onSendToInbox?: () => void;
  getAudioUrl: (fileName: string) => Promise<string | null>;
  deleteAudio: (id: number) => Promise<void>;
  retryTranscription: (id: number, modelId: string | null) => Promise<void>;
}

const HistoryEntryComponent: React.FC<HistoryEntryProps> = ({
  entry,
  onToggleSaved,
  onCopyText,
  onSendToInbox,
  getAudioUrl,
  deleteAudio,
  retryTranscription,
}) => {
  const { t, i18n } = useTranslation();
  const [showCopied, setShowCopied] = useState(false);
  const [retrying, setRetrying] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [showOriginal, setShowOriginal] = useState(false);

  const displayText = finalEntryText(entry);
  const hasTranscription = displayText.trim().length > 0;
  const isLong = displayText.length > PREVIEW_CHAR_LIMIT;
  const hasDifferentOriginal =
    Boolean(entry.post_processed_text?.trim()) &&
    entry.post_processed_text !== entry.transcription_text;

  const handleLoadAudio = useCallback(
    () => getAudioUrl(entry.file_name),
    [getAudioUrl, entry.file_name],
  );

  const handleCopyText = () => {
    if (!hasTranscription) {
      return;
    }

    onCopyText();
    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  const handleDeleteEntry = async () => {
    // Borra el texto Y el .wav, sin papelera ni deshacer: puede ser la única
    // copia de algo dictado hace semanas. Descartar una sesión ya preguntaba;
    // esto no, y era la acción más destructiva de la app (auditoría externa,
    // 30-jul-2026).
    const proceed = await confirmDialog(t("settings.history.deleteConfirm"), {
      title: "Escriba",
      kind: "warning",
    });
    if (!proceed) return;
    try {
      await deleteAudio(entry.id);
    } catch (error) {
      console.error("Failed to delete entry:", error);
      toast.error(t("settings.history.deleteError"));
    }
  };

  const handleRetranscribe = async (modelId: string | null = null) => {
    try {
      setRetrying(true);
      await retryTranscription(entry.id, modelId);
    } catch (error) {
      console.error("Failed to re-transcribe:", error);
      toast.error(t("settings.history.retranscribeError"));
    } finally {
      setRetrying(false);
    }
  };

  const timeLabel = new Intl.DateTimeFormat(i18n.language, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(entry.timestamp * 1000));

  return (
    <div className="group relative rounded-card border border-line bg-background px-4 py-3 shadow-card transition-all duration-150 hover:-translate-y-0.5 hover:border-logo-primary/30 hover:shadow-[0_10px_24px_-14px_rgba(27,20,38,0.25)]">
      {/* Acciones: aparecen al pasar el cursor (o con teclado), como toolbar flotante */}
      <div className="pointer-events-none absolute right-2.5 top-2 z-10 flex items-center gap-0.5 rounded-lg border border-line bg-background/95 px-1 py-0.5 opacity-0 shadow-sm backdrop-blur-sm transition-opacity group-hover:pointer-events-auto group-hover:opacity-100 focus-within:pointer-events-auto focus-within:opacity-100">
        <IconButton
          onClick={handleCopyText}
          disabled={!hasTranscription || retrying}
          title={t("settings.history.copyToClipboard")}
        >
          {showCopied ? (
            <Check width={15} height={15} />
          ) : (
            <Copy width={15} height={15} />
          )}
        </IconButton>
        {onSendToInbox && (
          <IconButton
            onClick={onSendToInbox}
            disabled={!hasTranscription || retrying}
            title={t("settings.history.sendToInbox")}
          >
            <Inbox width={15} height={15} />
          </IconButton>
        )}
        <IconButton
          onClick={onToggleSaved}
          disabled={retrying}
          active={entry.saved}
          title={
            entry.saved
              ? t("settings.history.unsave")
              : t("settings.history.save")
          }
        >
          <Star
            width={15}
            height={15}
            fill={entry.saved ? "currentColor" : "none"}
          />
        </IconButton>
        <RetranscribeMenu
          onRetranscribe={(modelId) => handleRetranscribe(modelId)}
          disabled={retrying}
          label={t("settings.history.retranscribe")}
        />
        <IconButton
          onClick={handleDeleteEntry}
          disabled={retrying}
          title={t("settings.history.delete")}
        >
          <Trash2 width={15} height={15} />
        </IconButton>
      </div>

      {/* El texto es el protagonista: primera línea de la tarjeta. */}
      <p
        className={`text-[15px] leading-relaxed ${
          retrying
            ? ""
            : hasTranscription
              ? "text-text select-text cursor-text whitespace-pre-wrap wrap-break-word"
              : "text-text/60"
        } ${!expanded && isLong ? "line-clamp-3" : ""}`}
        style={
          retrying
            ? { animation: "transcribe-pulse 3s ease-in-out infinite" }
            : undefined
        }
      >
        {retrying && (
          <style>{`
            @keyframes transcribe-pulse {
              0%, 100% { color: color-mix(in srgb, var(--color-text) 40%, transparent); }
              50% { color: color-mix(in srgb, var(--color-text) 90%, transparent); }
            }
          `}</style>
        )}
        {retrying
          ? t("settings.history.transcribing")
          : hasTranscription
            ? displayText
            : t("settings.history.transcriptionFailed")}
      </p>
      {isLong && !retrying && (
        <button
          onClick={() => setExpanded((v) => !v)}
          className="mt-1 text-xs font-medium text-gold-text hover:underline"
        >
          {expanded
            ? t("settings.history.showLess")
            : t("settings.history.showMore")}
        </button>
      )}
      {hasDifferentOriginal && !retrying && (
        <div className="mt-2">
          <button
            type="button"
            onClick={() => setShowOriginal((value) => !value)}
            aria-expanded={showOriginal}
            className="text-xs font-medium text-gold-text hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
          >
            {showOriginal
              ? t("settings.history.hideOriginal")
              : t("settings.history.showOriginal")}
          </button>
          {showOriginal && (
            <div className="mt-2 rounded-lg border border-line bg-surface/55 px-3 py-2">
              <p className="text-3xs font-semibold uppercase tracking-[0.12em] text-mid-gray">
                {t("settings.history.originalLabel")}
              </p>
              <p className="mt-1 whitespace-pre-wrap wrap-break-word text-sm leading-relaxed text-text/75">
                {entry.transcription_text}
              </p>
            </div>
          )}
        </div>
      )}

      {/* Metadatos: hora + tiempo relativo + audio de respaldo, en una línea. */}
      <div className="mt-2 flex items-center gap-2 text-xs text-mid-gray">
        {entry.saved && (
          <Star
            width={12}
            height={12}
            className="shrink-0 text-gold-text"
            fill="currentColor"
          />
        )}
        <span>{relativeTime(entry.timestamp, i18n.language)}</span>
        <span aria-hidden="true">·</span>
        <span className="tabular-nums">{timeLabel}</span>
        {/* Sin .wav no hay reproductor: guardar audio viene apagado de
            fábrica, así que la mayoría de las entradas son solo texto y un
            play que no suena sería peor que no tenerlo. */}
        {entry.file_name ? (
          <AudioPlayer
            onLoadRequest={handleLoadAudio}
            compact
            className="ms-auto w-44 max-w-[45%]"
          />
        ) : (
          <span className="ms-auto text-2xs text-mid-gray/70">
            {t("settings.history.noAudio")}
          </span>
        )}
      </div>
    </div>
  );
};
