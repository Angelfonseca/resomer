import { type ButtonHTMLAttributes, type ReactNode, forwardRef } from "react"
import { cn } from "../../lib/cn"

export type IconButtonVariant = "ghost" | "solid" | "signal"
export type IconButtonSize = "sm" | "md" | "lg" | "xl"

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: ReactNode
  variant?: IconButtonVariant
  size?: IconButtonSize
  "aria-label": string
}

const variantClasses: Record<IconButtonVariant, string> = {
  ghost: "bg-transparent text-ink-dim hover:bg-panel-hover hover:text-ink",
  solid: "bg-panel-hi text-ink border border-hairline-strong hover:bg-panel-hover",
  signal: "bg-signal text-signal-ink shadow-[var(--shadow-signal)] hover:bg-signal-hi",
}

const sizeClasses: Record<IconButtonSize, string> = {
  sm: "size-8 [&_svg]:size-4",
  md: "size-10 [&_svg]:size-[18px]",
  lg: "size-14 [&_svg]:size-6",
  xl: "size-20 [&_svg]:size-8",
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ icon, variant = "ghost", size = "md", disabled, className, ...props }, ref) => (
    <button
      ref={ref}
      disabled={disabled}
      className={cn(
        "inline-flex items-center justify-center rounded-full transition-all duration-150 active:scale-95",
        "disabled:opacity-40 disabled:cursor-not-allowed disabled:active:scale-100",
        variantClasses[variant],
        sizeClasses[size],
        className
      )}
      {...props}
    >
      {icon}
    </button>
  )
)

IconButton.displayName = "IconButton"
