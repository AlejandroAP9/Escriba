/**
 * Comportamiento de scroll que respeta "reducir movimiento" del sistema
 * (auditoría #17): con la preferencia activa, salta sin animación en vez de
 * hacer scroll suave. Coherente con la línea "Para todos los ojos".
 */
export function scrollBehavior(): ScrollBehavior {
  if (
    typeof window !== "undefined" &&
    window.matchMedia?.("(prefers-reduced-motion: reduce)").matches
  ) {
    return "auto";
  }
  return "smooth";
}
