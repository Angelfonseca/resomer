import { cn } from "../../lib/cn"

function formatTimecode(totalSeconds: number, showHours = false): string {
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = Math.floor(totalSeconds % 60)

  if (showHours || hours > 0) {
    return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`
  }
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}`
}

export function Timecode({
  seconds,
  showHours = true,
  size = "md",
  className,
}: {
  seconds: number
  showHours?: boolean
  size?: "sm" | "md" | "lg" | "xl"
  className?: string
}) {
  const sizeClasses = {
    sm: "text-sm",
    md: "text-lg",
    lg: "text-3xl",
    xl: "text-6xl",
  }

  return (
    <span className={cn("font-mono tabular-nums tracking-tight text-ink", sizeClasses[size], className)}>
      {formatTimecode(seconds, showHours)}
    </span>
  )
}

export { formatTimecode }
