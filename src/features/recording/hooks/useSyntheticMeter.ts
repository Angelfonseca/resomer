import { useEffect, useRef, useState } from "react"

const BAR_COUNT = 48

/** Gentle animated fallback for when live mic visualization isn't available. */
export function useSyntheticMeter(active: boolean) {
  const [levels, setLevels] = useState<number[]>(() => new Array(BAR_COUNT).fill(0))
  const rafRef = useRef<number | null>(null)
  const tRef = useRef(0)

  useEffect(() => {
    if (!active) {
      setLevels(new Array(BAR_COUNT).fill(0))
      return
    }

    const tick = () => {
      tRef.current += 0.09
      const next = Array.from({ length: BAR_COUNT }, (_, i) => {
        const wave = Math.sin(tRef.current + i * 0.4) * 0.5 + 0.5
        const noise = Math.random() * 0.25
        return Math.min(1, wave * 0.55 + noise)
      })
      setLevels(next)
      rafRef.current = requestAnimationFrame(tick)
    }
    tick()

    return () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current)
    }
  }, [active])

  return levels
}
