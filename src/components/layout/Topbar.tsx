import type { ReactNode } from "react"

export function Topbar({
  title,
  subtitle,
  action,
}: {
  title: string
  subtitle?: string
  action?: ReactNode
}) {
  return (
    <header className="flex shrink-0 items-center justify-between border-b border-hairline px-8 py-5">
      <div>
        <h1 className="font-display text-2xl font-semibold text-ink">{title}</h1>
        {subtitle && <p className="mt-0.5 text-sm text-ink-dim">{subtitle}</p>}
      </div>
      {action}
    </header>
  )
}
