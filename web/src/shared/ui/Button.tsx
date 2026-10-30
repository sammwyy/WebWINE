import type { ComponentProps } from "react";

interface ButtonProps extends ComponentProps<"button"> {
  /** default dialog button (Enter): highlighted / pulsing depending on the theme */
  isDefault?: boolean;
}

export function Button({ isDefault = false, className = "", type = "button", ...props }: ButtonProps) {
  return <button type={type} className={`ww-button ${isDefault ? "default" : ""} ${className}`} {...props} />;
}
