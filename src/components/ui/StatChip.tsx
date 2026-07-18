import { cn } from "../../lib/cn"

export function StatChip({
  label,
  value,
  tone = "default",
  className,
}: {
  label: string
  value: string | number
  tone?: "default" | "signal"
  className?: string
}) {
  return (
    <div
      className={cn(
        "rounded-control border border-hairline bg-canvas-raised px-4 py-3",
        className
      )}
    >
      <p className="mb-1 font-mono text-[10px] uppercase tracking-[0.12em] text-ink-mute">
        {label}
      </p>
      <p
        className={cn(
          "font-mono text-2xl font-medium tabular-nums",
          tone === "signal" ? "text-signal" : "text-ink"
        )}
      >
        {value}
      </p>
    </div>
  )
}
