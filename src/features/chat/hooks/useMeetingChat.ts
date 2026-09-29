import { useCallback, useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"

// El endpoint del gateway lo resuelve el backend desde su config; el frontend
// ya no lo envía.

export interface ChatMessage {
  role: "user" | "assistant"
  content: string
  created_at: string
}

export function useMeetingChat(meetingId: string | undefined) {
  const [messages, setMessages] = useState<ChatMessage[]>([])
  const [loadingHistory, setLoadingHistory] = useState(true)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!meetingId) {
      setMessages([])
      setLoadingHistory(false)
      return
    }
    let cancelled = false
    setLoadingHistory(true)
    invoke<ChatMessage[]>("get_chat_history", { meetingId })
      .then((history) => {
        if (!cancelled) setMessages(history)
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof Error ? err.message : String(err))
      })
      .finally(() => {
        if (!cancelled) setLoadingHistory(false)
      })
    return () => {
      cancelled = true
    }
  }, [meetingId])

  const ask = useCallback(
    async (question: string, model = "mimo-v2.5") => {
      if (!meetingId || !question.trim()) return

      setError(null)
      setSending(true)

      // Optimista: mostramos la pregunta de inmediato, antes de la respuesta.
      const userMessage: ChatMessage = {
        role: "user",
        content: question,
        created_at: new Date().toISOString(),
      }
      setMessages((prev) => [...prev, userMessage])

      try {
        const answer = await invoke<string>("ask_meeting_question", {
          meetingId,
          question,
          model,
        })
        setMessages((prev) => [
          ...prev,
          { role: "assistant", content: answer, created_at: new Date().toISOString() },
        ])
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        setError(message)
        // La pregunta optimista quedó sin respuesta: la quitamos para que el
        // usuario pueda reintentarla sin ver un mensaje "colgado".
        setMessages((prev) => prev.filter((m) => m !== userMessage))
      } finally {
        setSending(false)
      }
    },
    [meetingId]
  )

  return { messages, loadingHistory, sending, error, ask }
}
