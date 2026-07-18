import { cn } from "../../lib/cn"

export type StatusTone = "signal" | "rec" | "done" | "idle" | "warn"

const toneClasses: Record<StatusTone, string> = {
  signal: "bg-signal shadow-[0_0_0_3px_var(--color-signal-dim)]",
  rec: "bg-rec shadow-[0_0_0_3px_var(--color-rec-dim)]",
  done: "bg-done shadow-[0_0_0_3px_var(--color-done-dim)]",
  idle: "bg-ink-mute",
  warn: "bg-warn shadow-[0_0_0_3px_color-mix(in_oklab,var(--color-warn)_25%,transparent)]",
}

export function StatusDot({
  tone = "idle",
  pulse = false,
  className,
}: {
  tone?: StatusTone
  pulse?: boolean
  className?: string
}) {
  return (
    <span className={cn("relative inline-flex size-2 rounded-full", toneClasses[tone], className)}>
      {pulse && (
        <span
          className={cn(
            "absolute inset-0 rounded-full animate-ping",
            tone === "rec" ? "bg-rec" : "bg-signal"
          )}
        />
      )}
    </span>
  )
}
