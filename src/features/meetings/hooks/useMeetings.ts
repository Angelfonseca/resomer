import { useState, useCallback, useEffect } from "react"
import { invoke } from "@tauri-apps/api/core"

export interface Meeting {
  id: string
  title: string
  created_at: string
  audio_path: string | null
  state: "recording" | "processing" | "completed" | "error"
}

export function useMeetings() {
  const [meetings, setMeetings] = useState<Meeting[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState("")

  const refreshMeetings = useCallback(async () => {
    try {
      const result = await invoke<Meeting[]>("list_meetings")
      setMeetings(result)
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
    async (title: string) => {
      setLoading(true)
      setError("")
      try {
        const meeting = await invoke<Meeting>("create_meeting", { title })
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

  const deleteMeeting = useCallback(async (id: string) => {
    const previous = meetings
    // Actualización optimista: la quitamos de la lista de inmediato.
    setMeetings((prev) => prev.filter((m) => m.id !== id))
    try {
      await invoke("delete_meeting", { meetingId: id })
    } catch (err) {
      // Si falla, restauramos la lista y propagamos el error.
      setMeetings(previous)
      const message = err instanceof Error ? err.message : String(err)
      setError(message)
      throw err
    }
  }, [meetings])

  return {
    meetings,
    loading,
    error,
    createMeeting,
    deleteMeeting,
    refreshMeetings,
    setMeetings,
  }
}
