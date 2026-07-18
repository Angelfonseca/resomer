import { cn } from "../../lib/cn"

export function Waveform({
  levels,
  active,
  className,
}: {
  levels: number[]
  active: boolean
  className?: string
}) {
  return (
    <div
      className={cn(
        "flex h-full w-full items-center justify-center gap-[3px]",
        className
      )}
      role="img"
      aria-label={active ? "Nivel de audio en vivo" : "Sin señal de audio"}
    >
      {levels.map((level, i) => {
        const height = Math.max(6, level * 100)
        return (
          <span
            key={i}
            className={cn(
              "w-full max-w-[4px] rounded-full transition-[height] duration-75 ease-out",
              active ? "bg-signal" : "bg-hairline-strong"
            )}
            style={{
              height: `${height}%`,
              opacity: active ? 0.35 + level * 0.65 : 1,
            }}
          />
        )
      })}
    </div>
  )
}
