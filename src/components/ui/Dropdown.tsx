import React, { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettingLabelId } from "./settingLabel";

export interface DropdownOption {
  value: string;
  label: string;
  disabled?: boolean;
}

interface DropdownProps {
  options: DropdownOption[];
  className?: string;
  selectedValue: string | null;
  onSelect: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  onRefresh?: () => void;
  /**
   * Nombre accesible del control. Sin esto, un lector de pantalla solo anuncia
   * el valor elegido ("Abajo al centro") sin decir de qué ajuste es.
   */
  ariaLabel?: string;
}

export const Dropdown: React.FC<DropdownProps> = ({
  options,
  selectedValue,
  onSelect,
  className = "",
  placeholder = "Select an option...",
  disabled = false,
  onRefresh,
  ariaLabel,
}) => {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listboxRef = useRef<HTMLDivElement>(null);
  const baseId = useId();
  const listboxId = `${baseId}-listbox`;
  const valueId = `${baseId}-value`;
  const optionId = (index: number) => `${baseId}-option-${index}`;
  const settingLabelId = useSettingLabelId();

  // `aria-label` REEMPLAZA el contenido del botón, así que ponerlo a secas
  // anunciaría "Estilo del overlay" y se perdería el valor elegido. Nombrando
  // por referencia al título y al valor se oyen los dos: "Estilo del overlay,
  // Abajo al centro". Fuera de un SettingContainer no hay título al que
  // apuntar y se cae al comportamiento anterior (solo el valor).
  const labelledBy =
    !ariaLabel && settingLabelId ? `${settingLabelId} ${valueId}` : undefined;

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const selectedIndex = options.findIndex(
    (option) => option.value === selectedValue,
  );
  const selectedOption =
    selectedIndex >= 0 ? options[selectedIndex] : undefined;

  // El foco entra en la lista al abrirla: es lo que convierte al listbox en una
  // sola parada de tabulación en vez de una por opción (con 30 micrófonos
  // conectados, tabular por todos era el camino largo hacia ninguna parte).
  useEffect(() => {
    if (isOpen) listboxRef.current?.focus();
  }, [isOpen]);

  // Mantiene visible la opción activa cuando se navega con las flechas más
  // allá del borde del panel, que tiene scroll propio.
  useEffect(() => {
    if (!isOpen || activeIndex < 0) return;
    document
      .getElementById(optionId(activeIndex))
      ?.scrollIntoView({ block: "nearest" });
  }, [isOpen, activeIndex]);

  const firstEnabled = (from: number, step: number) => {
    for (let i = from; i >= 0 && i < options.length; i += step) {
      if (!options[i].disabled) return i;
    }
    return -1;
  };

  const open = () => {
    if (disabled) return;
    if (onRefresh) onRefresh();
    const start =
      selectedIndex >= 0 && !options[selectedIndex]?.disabled
        ? selectedIndex
        : firstEnabled(0, 1);
    setActiveIndex(start);
    setIsOpen(true);
  };

  const close = (returnFocus: boolean) => {
    setIsOpen(false);
    setActiveIndex(-1);
    if (returnFocus) triggerRef.current?.focus();
  };

  const handleSelect = (value: string) => {
    onSelect(value);
    close(true);
  };

  const handleToggle = () => {
    if (disabled) return;
    if (isOpen) close(true);
    else open();
  };

  const handleTriggerKeyDown = (event: React.KeyboardEvent) => {
    if (disabled || isOpen) return;
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      open();
    }
  };

  const handleListboxKeyDown = (event: React.KeyboardEvent) => {
    switch (event.key) {
      case "ArrowDown": {
        event.preventDefault();
        const next = firstEnabled(activeIndex + 1, 1);
        if (next >= 0) setActiveIndex(next);
        break;
      }
      case "ArrowUp": {
        event.preventDefault();
        const prev = firstEnabled(activeIndex - 1, -1);
        if (prev >= 0) setActiveIndex(prev);
        break;
      }
      case "Home": {
        event.preventDefault();
        const first = firstEnabled(0, 1);
        if (first >= 0) setActiveIndex(first);
        break;
      }
      case "End": {
        event.preventDefault();
        const last = firstEnabled(options.length - 1, -1);
        if (last >= 0) setActiveIndex(last);
        break;
      }
      case "Enter":
      case " ": {
        event.preventDefault();
        const option = options[activeIndex];
        if (option && !option.disabled) handleSelect(option.value);
        break;
      }
      case "Escape": {
        event.preventDefault();
        close(true);
        break;
      }
      case "Tab":
        // Sin preventDefault: cerrar y dejar que el foco siga su camino normal.
        close(false);
        break;
    }
  };

  return (
    <div className={`relative ${className}`} ref={dropdownRef}>
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        aria-controls={isOpen ? listboxId : undefined}
        aria-label={ariaLabel}
        aria-labelledby={labelledBy}
        className={`px-2 py-[5px] text-sm font-semibold bg-mid-gray/10 border border-mid-gray/80 rounded-md min-w-[200px] w-full text-start grid grid-cols-[1fr_auto] gap-2 items-center transition-all duration-150 ${
          disabled
            ? "opacity-50 cursor-not-allowed"
            : "hover:bg-logo-primary/10 cursor-pointer hover:border-logo-primary"
        }`}
        onClick={handleToggle}
        onKeyDown={handleTriggerKeyDown}
        disabled={disabled}
      >
        <span id={valueId} className="truncate">
          {selectedOption?.label || placeholder}
        </span>
        <svg
          className={`w-4 h-4 transition-transform duration-200 ${isOpen ? "transform rotate-180" : ""}`}
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
          aria-hidden="true"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M19 9l-7 7-7-7"
          />
        </svg>
      </button>
      {isOpen && !disabled && (
        <div
          ref={listboxRef}
          id={listboxId}
          role="listbox"
          tabIndex={-1}
          aria-label={ariaLabel}
          aria-labelledby={!ariaLabel ? settingLabelId : undefined}
          aria-activedescendant={
            activeIndex >= 0 ? optionId(activeIndex) : undefined
          }
          onKeyDown={handleListboxKeyDown}
          className="absolute top-full left-0 right-0 mt-1 bg-background border border-mid-gray/80 rounded-md shadow-lg z-50 max-h-60 overflow-y-auto focus:outline-none"
        >
          {options.length === 0 ? (
            <div className="px-2 py-1 text-sm text-mid-gray">
              {t("common.noOptionsFound")}
            </div>
          ) : (
            options.map((option, index) => (
              <div
                key={option.value}
                id={optionId(index)}
                role="option"
                aria-selected={selectedValue === option.value}
                aria-disabled={option.disabled || undefined}
                className={`w-full px-2 py-1 text-sm text-start transition-colors duration-150 ${
                  selectedValue === option.value
                    ? "bg-logo-primary/20 font-semibold"
                    : ""
                } ${
                  index === activeIndex && !option.disabled
                    ? "bg-logo-primary/10"
                    : ""
                } ${
                  option.disabled
                    ? "opacity-50 cursor-not-allowed"
                    : "cursor-pointer hover:bg-logo-primary/10"
                }`}
                onClick={() => {
                  if (!option.disabled) handleSelect(option.value);
                }}
                onMouseEnter={() => {
                  if (!option.disabled) setActiveIndex(index);
                }}
              >
                <span className="whitespace-normal break-words">
                  {option.label}
                </span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
