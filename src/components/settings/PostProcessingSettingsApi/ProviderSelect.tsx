import React from "react";
import { useTranslation } from "react-i18next";
import { Dropdown, type DropdownOption } from "../../ui/Dropdown";

/** Proveedores que corren en el propio equipo: nada sale de la máquina. */
const LOCAL_PROVIDER_IDS = ["local_llm", "apple_intelligence"];

interface ProviderSelectProps {
  options: DropdownOption[];
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}

export const ProviderSelect: React.FC<ProviderSelectProps> = React.memo(
  ({ options, value, onChange, disabled }) => {
    const { t } = useTranslation();
    const isRemote = !LOCAL_PROVIDER_IDS.includes(value);

    return (
      <div className="flex flex-1 flex-col gap-1.5">
        <Dropdown
          options={options}
          selectedValue={value}
          onSelect={onChange}
          disabled={disabled}
          className="w-full"
        />
        {/*
          Consentimiento en el punto donde se toma la decisión.
          Elegir un proveedor remoto manda el TEXTO transcrito a un tercero, y
          hasta ahora ninguna pantalla lo decía: el ajuste existía, venía
          apagado (que es lo correcto), pero la consecuencia no estaba escrita
          donde el usuario elige. Se dice también lo que NO sale, porque es la
          mitad que hace la promesa creíble.
        */}
        {isRemote && (
          <p className="text-xs text-warning" role="note">
            {t("settings.privacy.remoteProviderNotice")}
          </p>
        )}
      </div>
    );
  },
);

ProviderSelect.displayName = "ProviderSelect";
