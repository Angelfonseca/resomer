import { type HTMLAttributes, forwardRef } from "react"
import { cn } from "../../lib/cn"

interface PanelProps extends HTMLAttributes<HTMLDivElement> {
  raised?: boolean
}

export const Panel = forwardRef<HTMLDivElement, PanelProps>(
  ({ className, raised = false, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "rounded-panel border border-hairline shadow-[var(--shadow-panel)]",
        raised ? "bg-panel-hi" : "bg-panel",
        className
      )}
      {...props}
    >
      {children}
    </div>
  )
)

Panel.displayName = "Panel"

export function PanelHeader({
  eyebrow,
  title,
  action,
}: {
  eyebrow?: string
  title: string
  action?: React.ReactNode
}) {
  return (
    <div className="flex items-start justify-between gap-4 px-6 pt-6 pb-4">
      <div>
        {eyebrow && (
          <p className="mb-1 font-mono text-[11px] uppercase tracking-[0.14em] text-signal">
            {eyebrow}
          </p>
        )}
        <h2 className="font-display text-xl font-semibold text-ink">{title}</h2>
      </div>
      {action}
    </div>
  )
}

export function PanelBody({ className, children }: { className?: string; children: React.ReactNode }) {
  return <div className={cn("px-6 pb-6", className)}>{children}</div>
}
