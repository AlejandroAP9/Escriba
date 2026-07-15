import type { SidebarSection } from "@/components/Sidebar";

/**
 * Navegación entre pantallas desde cualquier componente, sin acoplar cada
 * feature al estado de App: se emite un evento y App cambia la sección.
 */
export const NAVIGATE_EVENT = "escriba:navigate";

export const navigateTo = (section: SidebarSection) => {
  window.dispatchEvent(new CustomEvent(NAVIGATE_EVENT, { detail: section }));
};
