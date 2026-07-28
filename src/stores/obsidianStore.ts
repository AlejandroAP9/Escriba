import { create } from "zustand";

/**
 * Exportación a Obsidian pendiente de revisión.
 *
 * El diálogo se monta UNA sola vez en `App.tsx` y se abre desde donde sea
 * (Sesiones, Estudio) llamando a `requestObsidianExport`. La alternativa era
 * que cada sitio de llamada llevara su propio estado y su propio diálogo, con
 * el mismo código duplicado en dos árboles distintos.
 */
interface ObsidianPreviewStore {
  open: boolean;
  /** Título propuesto, editable por el usuario antes de guardar. */
  title: string;
  /** Cuerpo propuesto, editable. */
  content: string;
  requestObsidianExport: (title: string, content: string) => void;
  setTitle: (title: string) => void;
  setContent: (content: string) => void;
  close: () => void;
}

export const useObsidianStore = create<ObsidianPreviewStore>((set) => ({
  open: false,
  title: "",
  content: "",
  requestObsidianExport: (title, content) =>
    set({ open: true, title, content }),
  setTitle: (title) => set({ title }),
  setContent: (content) => set({ content }),
  close: () => set({ open: false }),
}));

/** Atajo para los sitios de llamada, que no necesitan el resto del store. */
export const requestObsidianExport = (title: string, content: string) =>
  useObsidianStore.getState().requestObsidianExport(title, content);
