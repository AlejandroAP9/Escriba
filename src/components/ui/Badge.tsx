import React from "react";

interface BadgeProps {
  children: React.ReactNode;
  variant?: "primary" | "success" | "secondary";
  className?: string;
}

const Badge: React.FC<BadgeProps> = ({
  children,
  variant = "primary",
  className = "",
}) => {
  const variantClasses = {
    // Oro con texto tinta: reservado para el estado que importa (Activo).
    primary: "bg-logo-primary text-ink font-semibold",
    success: "bg-green-500/20 text-green-400",
    secondary: "bg-mid-gray/15 text-text/75",
  };

  return (
    <span
      className={`inline-flex items-center px-3 py-1 rounded-full text-xs font-medium ${variantClasses[variant]} ${className}`}
    >
      {children}
    </span>
  );
};

export default Badge;
