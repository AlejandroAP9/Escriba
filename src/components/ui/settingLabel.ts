import { createContext, useContext } from "react";

/**
 * Id del `<h3>` que titula una fila de ajustes.
 *
 * `SettingContainer` pinta el título y el control como hermanos, sin ninguna
 * relación programática entre ambos: el control queda sin nombre accesible y un
 * lector de pantalla solo anuncia su valor ("Abajo al centro") sin decir de qué
 * ajuste es. Pasar el id por contexto deja que cada control se enlace solo, sin
 * tocar los sitios donde se usa.
 *
 * Es `undefined` fuera de un `SettingContainer`; los controles deben tolerarlo
 * y caer a su comportamiento anterior.
 */
export const SettingLabelContext = createContext<string | undefined>(undefined);

export const useSettingLabelId = () => useContext(SettingLabelContext);
