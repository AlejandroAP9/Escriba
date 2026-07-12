import React from "react";

interface SettingsGroupProps {
  title?: string;
  description?: string;
  children: React.ReactNode;
}

export const SettingsGroup: React.FC<SettingsGroupProps> = ({
  title,
  description,
  children,
}) => {
  return (
    <div className="space-y-2">
      {title && (
        <div className="px-4">
          <h2 className="text-xs font-medium text-mid-gray uppercase tracking-wide">
            {title}
          </h2>
          {description && (
            <p className="text-xs text-mid-gray mt-1">{description}</p>
          )}
        </div>
      )}
      <div className="bg-background border border-mid-gray/15 rounded-xl overflow-visible shadow-[0_1px_2px_rgba(27,20,38,0.04),0_14px_30px_-18px_rgba(27,20,38,0.14)]">
        <div className="divide-y divide-mid-gray/12">{children}</div>
      </div>
    </div>
  );
};
