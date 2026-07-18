import { type ButtonHTMLAttributes, type ReactNode, forwardRef } from "react"
import { Loader2 } from "lucide-react"
import { cn } from "../../lib/cn"

export type ButtonVariant = "primary" | "ghost" | "outline" | "danger" | "subtle"
export type ButtonSize = "sm" | "md" | "lg"

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode
  variant?: ButtonVariant
  size?: ButtonSize
  loading?: boolean
  leftIcon?: ReactNode
  rightIcon?: ReactNode
}

const variantClasses: Record<ButtonVariant, string> = {
  primary:
    "bg-signal text-signal-ink font-semibold shadow-[var(--shadow-signal)] hover:bg-signal-hi active:scale-[0.98]",
  ghost:
    "bg-transparent text-ink-dim hover:bg-panel-hover hover:text-ink",
  outline:
    "bg-transparent text-ink border border-hairline-strong hover:bg-panel-hover hover:border-signal/40",
  danger:
    "bg-rec text-ink font-semibold hover:brightness-110 active:scale-[0.98]",
  subtle:
    "bg-panel-hi text-ink hover:bg-panel-hover border border-hairline",
}

const sizeClasses: Record<ButtonSize, string> = {
  sm: "h-8 px-3 text-xs gap-1.5 rounded-md",
  md: "h-10 px-4 text-sm gap-2 rounded-lg",
  lg: "h-12 px-6 text-base gap-2.5 rounded-lg",
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      children,
      variant = "primary",
      size = "md",
      loading = false,
      disabled = false,
      leftIcon,
      rightIcon,
      className = "",
      ...props
    },
    ref
  ) => {
    return (
      <button
        ref={ref}
        disabled={disabled || loading}
        className={cn(
          "inline-flex items-center justify-center whitespace-nowrap font-body transition-all duration-150",
          "disabled:opacity-40 disabled:cursor-not-allowed disabled:active:scale-100",
          variantClasses[variant],
          sizeClasses[size],
          className
        )}
        {...props}
      >
        {loading ? (
          <Loader2 className="size-4 animate-spin" strokeWidth={2.5} />
        ) : (
          leftIcon
        )}
        {children}
        {!loading && rightIcon}
      </button>
    )
  }
)

Button.displayName = "Button"
