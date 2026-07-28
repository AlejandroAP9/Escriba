import type { SidebarSection } from "@/components/Sidebar";

/**
 * Navegación entre pantallas desde cualquier componente, sin acoplar cada
 * feature al estado de App: se emite un evento y App cambia la sección.
 */
export const NAVIGATE_EVENT = "escriba:navigate";

export const navigateTo = (section: SidebarSection) => {
  window.dispatchEvent(new CustomEvent(NAVIGATE_EVENT, { detail: section }));
};

/**
 * Volver a ver el asistente de bienvenida.
 *
 * Solo cambia el estado en memoria: NO toca `onboarding_completed` ni ningún
 * otro ajuste. Es la diferencia entre poder repetir la bienvenida y tener que
 * borrar la configuración para verla, que era la única forma que había.
 */
export const REPLAY_ONBOARDING_EVENT = "escriba:replay-onboarding";

export const replayOnboarding = () => {
  window.dispatchEvent(new Event(REPLAY_ONBOARDING_EVENT));
};
