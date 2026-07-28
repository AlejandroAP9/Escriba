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

/**
 * Tamaño base de la interfaz al 100%, en px. Tiene que coincidir con el
 * `font-size` de `:root` en App.css.
 */
const UI_BASE_FONT_PX = 15;

/**
 * Cuánto crece el texto en Modo Calma, ENCIMA de la escala elegida.
 * 130% × 1,1 = 143%: deliberado, la escala manual conserva su significado.
 */
const CALM_TEXT_BOOST = 1.1;

export const applyUiScale = (scale?: number, calmBoost = false) => {
  const pct = Math.min(UI_SCALE_MAX, Math.max(UI_SCALE_MIN, scale ?? 100));
  const effective = calmBoost ? pct * CALM_TEXT_BOOST : pct;
  // Se escala en px sobre la base real de la app, no en %.
  //
  // Antes se ponía `${pct}%`, y un porcentaje sobre el root se resuelve contra
  // el tamaño por omisión del navegador (16px), no contra los 15px que fija
  // App.css. Además al 100% se limpiaba el estilo inline. El resultado eran
  // pasos desiguales: 90% daba 14,4px (un 4% menos que 15) mientras que 115%
  // saltaba a 18,4px (un 23% más). El paso pequeño casi no se notaba y el
  // primero hacia arriba era brusco.
  document.documentElement.style.fontSize = `${(UI_BASE_FONT_PX * effective) / 100}px`;
};

/**
 * Modos de accesibilidad visual, como atributos en <html> (mismo mecanismo que
 * `data-theme`): el CSS de `styles/a11y.css` reacciona a su presencia. Los tres
 * componen entre sí y con ambos temas.
 */
export const applyA11yModes = (modes: {
  highContrast?: boolean;
  colorblind?: boolean;
  calm?: boolean;
  alwaysShowFocus?: boolean;
}) => {
  const root = document.documentElement;
  root.toggleAttribute("data-high-contrast", !!modes.highContrast);
  root.toggleAttribute("data-colorblind", !!modes.colorblind);
  root.toggleAttribute("data-calm", !!modes.calm);
  root.toggleAttribute("data-focus-always", !!modes.alwaysShowFocus);
};
