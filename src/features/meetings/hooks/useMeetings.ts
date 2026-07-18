import { useState, useCallback } from "react"
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
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState("")

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

  return {
    meetings,
    loading,
    error,
    createMeeting,
    setMeetings,
  }
}
