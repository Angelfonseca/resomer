import { useEffect, useRef, useState } from "react"
import { listen, type UnlistenFn } from "@tauri-apps/api/event"

const BAR_COUNT = 48

interface AudioLevels {
  system: number
  mic: number
}

const emptyLevels = () => new Array(BAR_COUNT).fill(0)

/**
 * Nivel de audio en vivo REAL del audio del sistema y del micrófono (medidos
 * por separado, incluso cuando están mezclados en modo "ambos"), calculado
 * por el helper de Swift a partir del audio que efectivamente se está
 * escribiendo al WAV, y reenviado desde Rust como evento Tauri
 * "system-audio-level". A diferencia de useAudioMeter (que abre un
 * getUserMedia aparte solo para visualizar el micrófono), esto refleja la
 * señal real capturada — útil para confirmar que el audio realmente se está
 * grabando y para distinguir visualmente ambas fuentes en modo "ambos".
 */
export function useSystemAudioMeter(active: boolean) {
  const [systemLevels, setSystemLevels] = useState<number[]>(emptyLevels)
  const [micLevels, setMicLevels] = useState<number[]>(emptyLevels)
  const unlistenRef = useRef<UnlistenFn | null>(null)

  useEffect(() => {
    if (!active) {
      setSystemLevels(emptyLevels())
      setMicLevels(emptyLevels())
      return
    }

    let cancelled = false

    listen<AudioLevels>("system-audio-level", (event) => {
      const system = Math.min(1, Math.max(0, event.payload.system))
      const mic = Math.min(1, Math.max(0, event.payload.mic))
      setSystemLevels((prev) => [...prev.slice(1), system])
      setMicLevels((prev) => [...prev.slice(1), mic])
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

  return { systemLevels, micLevels }
}
