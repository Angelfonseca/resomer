import { cn } from "../../lib/cn"

// Modo "dual": dos señales espejadas desde una línea central (sistema hacia
// arriba, micrófono hacia abajo) para distinguirlas por color Y por posición
// — legible incluso si los colores se confunden. Reusa el token de paleta
// "channel-2" (ya usado para diferenciar hablantes) para no introducir un
// color nuevo fuera del sistema de diseño.
export function Waveform({
  levels,
  systemLevels,
  micLevels,
  active,
  className,
}: {
  levels?: number[]
  systemLevels?: number[]
  micLevels?: number[]
  active: boolean
  className?: string
}) {
  if (systemLevels && micLevels) {
    return (
      <div
        className={cn("flex h-full w-full items-stretch justify-center gap-[3px]", className)}
        role="img"
        aria-label={active ? "Nivel de sistema y micrófono en vivo" : "Sin señal de audio"}
      >
        {systemLevels.map((sys, i) => {
          const mic = micLevels[i] ?? 0
          const sysHeight = Math.max(6, sys * 100)
          const micHeight = Math.max(6, mic * 100)
          return (
            <div key={i} className="flex w-full max-w-[4px] flex-col items-center justify-center gap-px">
              <span
                className={cn("w-full rounded-t-full transition-[height] duration-75 ease-out")}
                style={{
                  height: `${sysHeight / 2}%`,
                  backgroundColor: active ? "var(--color-channel-2)" : "var(--color-hairline-strong)",
                  opacity: active ? 0.35 + sys * 0.65 : 1,
                }}
              />
              <span
                className={cn(
                  "w-full rounded-b-full transition-[height] duration-75 ease-out",
                  active ? "bg-signal" : "bg-hairline-strong"
                )}
                style={{
                  height: `${micHeight / 2}%`,
                  opacity: active ? 0.35 + mic * 0.65 : 1,
                }}
              />
            </div>
          )
        })}
      </div>
    )
  }

  return (
    <div
      className={cn(
        "flex h-full w-full items-center justify-center gap-[3px]",
        className
      )}
      role="img"
      aria-label={active ? "Nivel de audio en vivo" : "Sin señal de audio"}
    >
      {(levels ?? []).map((level, i) => {
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
