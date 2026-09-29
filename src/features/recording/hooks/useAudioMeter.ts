import { useEffect, useRef, useState } from "react"

const BAR_COUNT = 48

/**
 * Live microphone level meter via Web Audio's AnalyserNode.
 * This opens a *separate* getUserMedia stream purely for visualization —
 * it does not touch the Rust/cpal recording pipeline, which owns the
 * actual capture that gets written to disk.
 */
export function useAudioMeter(active: boolean) {
  const [levels, setLevels] = useState<number[]>(() => new Array(BAR_COUNT).fill(0))
  const [errored, setErrored] = useState(false)
  const rafRef = useRef<number | null>(null)
  const streamRef = useRef<MediaStream | null>(null)
  const audioCtxRef = useRef<AudioContext | null>(null)

  useEffect(() => {
    if (!active) {
      setLevels(new Array(BAR_COUNT).fill(0))
      return
    }

    let cancelled = false
    // Un fallo transitorio anterior no debe dejar el medidor en modo sintético
    // para toda la sesión: cada activación reintenta el micrófono real.
    setErrored(false)

    async function start() {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true })
        if (cancelled) {
          stream.getTracks().forEach((t) => t.stop())
          return
        }
        streamRef.current = stream

        const AudioContextCtor =
          window.AudioContext ??
          (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
        if (!AudioContextCtor) {
          throw new Error("AudioContext no disponible en este WebView")
        }
        const audioCtx = new AudioContextCtor()
        audioCtxRef.current = audioCtx

        const source = audioCtx.createMediaStreamSource(stream)
        const analyser = audioCtx.createAnalyser()
        analyser.fftSize = 256
        analyser.smoothingTimeConstant = 0.75
        source.connect(analyser)

        const data = new Uint8Array(analyser.frequencyBinCount)
        const bucketSize = Math.floor(data.length / BAR_COUNT)

        const tick = () => {
          analyser.getByteFrequencyData(data)
          const next: number[] = []
          for (let i = 0; i < BAR_COUNT; i++) {
            let sum = 0
            for (let j = 0; j < bucketSize; j++) {
              sum += data[i * bucketSize + j]
            }
            next.push(Math.min(1, sum / bucketSize / 180))
          }
          setLevels(next)
          rafRef.current = requestAnimationFrame(tick)
        }
        tick()
      } catch {
        if (!cancelled) setErrored(true)
      }
    }

    start()

    return () => {
      cancelled = true
      if (rafRef.current) cancelAnimationFrame(rafRef.current)
      streamRef.current?.getTracks().forEach((t) => t.stop())
      audioCtxRef.current?.close().catch(() => {})
      streamRef.current = null
      audioCtxRef.current = null
    }
  }, [active])

  return { levels, errored }
}
