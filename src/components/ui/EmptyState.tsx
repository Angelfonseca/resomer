import type { ReactNode } from "react"

export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon?: ReactNode
  title: string
  description?: string
  action?: ReactNode
}) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 rounded-panel border border-dashed border-hairline-strong px-8 py-14 text-center">
      {icon && <div className="text-ink-mute [&_svg]:size-8">{icon}</div>}
      <div className="space-y-1">
        <p className="font-display text-base font-medium text-ink">{title}</p>
        {description && <p className="max-w-xs text-sm text-ink-dim">{description}</p>}
      </div>
      {action}
    </div>
  )
}
