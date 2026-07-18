import { useEffect, useRef, useState } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

const BAR_COUNT = 48

/**
 * Nivel de audio en vivo REAL del audio del sistema (y del micrófono cuando
 * está mezclado en modo "ambos"), calculado por el helper de Swift a partir
 * del audio que efectivamente se está escribiendo al WAV, y reenviado desde
 * Rust como evento Tauri "system-audio-level". A diferencia de useAudioMeter
 * (que abre un getUserMedia aparte solo para visualizar el micrófono), esto
 * refleja la señal real capturada — útil para confirmar que el audio del
 * sistema realmente se está grabando.
 */
export function useSystemAudioMeter(active: boolean) {
  const [levels, setLevels] = useState<number[]>(() => new Array(BAR_COUNT).fill(0))
  const unlistenRef = useRef<UnlistenFn | null>(null)

  useEffect(() => {
    if (!active) {
      setLevels(new Array(BAR_COUNT).fill(0))
      return
    }

    let cancelled = false

    listen<number>("system-audio-level", (event) => {
      const value = Math.min(1, Math.max(0, event.payload))
      setLevels((prev) => {
        const next = prev.slice(1)
        next.push(value)
        return next
      })
    }).then((unlisten) => {
      if (cancelled) {
        unlisten()
      } else {
        unlistenRef.current = unlisten
      }
    })

    return () => {
      cancelled = true
      unlistenRef.current?.()
      unlistenRef.current = null
    }
  }, [active])

  return levels
}
