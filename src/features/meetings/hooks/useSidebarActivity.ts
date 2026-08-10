import { useEffect, useRef, useState } from "react"
import type { Meeting } from "./useMeetings"

const LINGER_MS = 20_000

export interface ActivityItem {
  meeting: Meeting
  justFinished: boolean
}

/**
 * Deriva qué reuniones mostrar en el sidebar: solo las activas
 * (recording / processing / error) más las que ACABAN de completarse, que
 * permanecen 20s con el aviso "Pipeline terminado" antes de desaparecer del
 * sidebar (siguen disponibles en la vista Reuniones).
 */
export function useSidebarActivity(meetings: Meeting[]): ActivityItem[] {
  const prevState = useRef<Map<string, Meeting["state"]>>(new Map())
  const initialized = useRef(false)
  const [finished, setFinished] = useState<Map<string, number>>(new Map())

  useEffect(() => {
    const prev = prevState.current
    // Primera pasada: solo sembrar estados, sin emitir transiciones — evita
    // marcar como "recién terminadas" reuniones ya completas al abrir la app.
    if (!initialized.current) {
      for (const m of meetings) prev.set(m.id, m.state)
      initialized.current = true
      return
    }
    const newly: string[] = []
    for (const m of meetings) {
      const before = prev.get(m.id)
      if (m.state === "completed" && before && before !== "completed") newly.push(m.id)
      prev.set(m.id, m.state)
    }
    if (newly.length) {
      const now = Date.now()
      setFinished((f) => {
        const next = new Map(f)
        for (const id of newly) next.set(id, now)
        return next
      })
    }
  }, [meetings])

  // Vencer las que ya cumplieron 20s.
  useEffect(() => {
    if (finished.size === 0) return
    const timers = [...finished.entries()].map(([id, at]) =>
      window.setTimeout(
        () => {
          setFinished((f) => {
            const next = new Map(f)
            next.delete(id)
            return next
          })
        },
        Math.max(0, LINGER_MS - (Date.now() - at))
      )
    )
    return () => timers.forEach(clearTimeout)
  }, [finished])

  const active = meetings.filter(
    (m) => m.state === "recording" || m.state === "processing" || m.state === "error"
  )
  const lingering = meetings.filter((m) => m.state === "completed" && finished.has(m.id))
  return [
    ...active.map((meeting) => ({ meeting, justFinished: false })),
    ...lingering.map((meeting) => ({ meeting, justFinished: true })),
  ]
}
