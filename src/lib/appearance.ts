/**
 * Apariencia accesible (tanda de inclusión, feedback de la comunidad 15-jul):
 * tema manual y escala de texto. El tema se aplica estampando `data-theme` en
 * <html> (gana sobre prefers-color-scheme en ambas direcciones; sin atributo
 * manda el sistema). La escala usa font-size en % sobre el root: toda la UI
 * está en rem, así que escala completa de una vez.
 */

export const applyUiTheme = (theme?: string) => {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") {
    root.setAttribute("data-theme", theme);
  } else {
    root.removeAttribute("data-theme");
  }
};

export const UI_SCALE_MIN = 90;
export const UI_SCALE_MAX = 130;

export const applyUiScale = (scale?: number) => {
  const pct = Math.min(UI_SCALE_MAX, Math.max(UI_SCALE_MIN, scale ?? 100));
  // A 100% se limpia el estilo inline para no pisar el default del navegador.
  document.documentElement.style.fontSize = pct === 100 ? "" : `${pct}%`;
};
