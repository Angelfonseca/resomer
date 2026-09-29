import { useState, useCallback, useEffect, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"

export interface Meeting {
  id: string
  title: string
  created_at: string
  audio_path: string | null
  state: "recording" | "processing" | "completed" | "error"
  expected_speakers: number | null
  category: string | null
}

export function useMeetings() {
  const [meetings, setMeetings] = useState<Meeting[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState("")
  // Espejo sincrónico de la lista para leer el valor previo dentro de
  // callbacks sin depender de `meetings` (evita recrear los callbacks en cada
  // cambio y snapshots obsoletos al revertir actualizaciones optimistas).
  const meetingsRef = useRef(meetings)
  meetingsRef.current = meetings

  const refreshMeetings = useCallback(async () => {
    try {
      const result = await invoke<Meeting[]>("list_meetings")
      setMeetings(result)
      setError("")
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setError(message)
    } finally {
      setLoading(false)
    }
  }, [])

  // Load meetings from database on mount
  useEffect(() => {
    refreshMeetings()
  }, [refreshMeetings])

  const createMeeting = useCallback(
    async (title: string, category: string | null = null) => {
      setLoading(true)
      setError("")
      try {
        const meeting = await invoke<Meeting>("create_meeting", { title, expectedSpeakers: null, category })
        setMeetings((prev) => [meeting, ...prev])
        return meeting
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        setError(message)
        throw err
      } finally {
        setLoading(false)
      }
    },
    []
  )

  const setMeetingCategory = useCallback(async (id: string, category: string | null) => {
    const previousCategory = meetingsRef.current.find((m) => m.id === id)?.category ?? null
    // Actualización optimista, igual que el borrado: se refleja de inmediato
    // y se revierte si el backend falla.
    setMeetings((prev) => prev.map((m) => (m.id === id ? { ...m, category } : m)))

    // Si es un ID temporal ("local-..."), no intentamos guardar en el backend todavía
    if (id.startsWith("local-")) {
      return
    }

    try {
      await invoke("update_meeting_category", { meetingId: id, category })
      // Forzar recarga para garantizar consistencia, especialmente importante
      // si la categoría era "Nueva categoría..." y necesita propagarse
      await refreshMeetings()
    } catch (err) {
      setMeetings((prev) =>
        prev.map((m) => (m.id === id ? { ...m, category: previousCategory } : m))
      )
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [refreshMeetings])

  const deleteMeeting = useCallback(
    async (id: string) => {
      // Actualización optimista: la quitamos de la lista de inmediato. Si el
      // backend falla, resincronizamos desde él (fuente de verdad) en vez de
      // restaurar un snapshot que puede haber quedado obsoleto.
      setMeetings((prev) => prev.filter((m) => m.id !== id))
      try {
        await invoke("delete_meeting", { meetingId: id })
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
        await refreshMeetings()
      }
    },
    [refreshMeetings]
  )

  return {
    meetings,
    loading,
    error,
    createMeeting,
    deleteMeeting,
    setMeetingCategory,
    refreshMeetings,
    setMeetings,
  }
}
