import React, { useState } from "react";
import { ChevronDown } from "lucide-react";

interface CollapsibleGroupProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
}

/**
 * Grupo de ajustes colapsable (acordeón): el encabezado abre/cierra el cuerpo.
 * Mismo lenguaje visual que SettingsGroup (tarjeta con profundidad), pero evita
 * mostrar decenas de controles a la vez.
 */
export const CollapsibleGroup: React.FC<CollapsibleGroupProps> = ({
  title,
  children,
  defaultOpen = false,
}) => {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div>
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        className="flex w-full items-center justify-between rounded-lg px-2 py-2 text-left transition-colors hover:bg-mid-gray/5"
      >
        <span className="text-xs font-medium uppercase tracking-wide text-mid-gray">
          {title}
        </span>
        <ChevronDown
          width={16}
          height={16}
          className={`text-mid-gray transition-transform duration-200 ${
            open ? "rotate-180" : ""
          }`}
        />
      </button>
      {open && (
        <div className="mt-1 overflow-visible rounded-card border border-line bg-background shadow-lift">
          <div className="divide-y divide-mid-gray/12">{children}</div>
        </div>
      )}
    </div>
  );
};
