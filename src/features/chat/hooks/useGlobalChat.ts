import { useCallback, useEffect, useRef, useState } from "react"
import { invoke } from "@tauri-apps/api/core"

// El endpoint del gateway lo resuelve el backend desde su config.

export interface GlobalSource {
  meeting_id: string
  meeting_title: string
}

export interface GlobalChatMessage {
  role: "user" | "assistant"
  content: string
  created_at: string
  // Solo presente en mensajes recién respondidos en esta sesión — el
  // historial persistido no guarda las fuentes citadas de cada turno, así
  // que al recargar la conversación los mensajes antiguos no traen chips.
  sources?: GlobalSource[]
}

interface GlobalAnswer {
  answer: string
  sources: GlobalSource[]
}

export function useGlobalChat(conversationId: string | undefined) {
  const [messages, setMessages] = useState<GlobalChatMessage[]>([])
  const [loadingHistory, setLoadingHistory] = useState(true)
  const [sending, setSending] = useState(false)
  const [error, setError] = useState<string | null>(null)
  // Guarda contra respuestas obsoletas: si el usuario cambia de conversación
  // mientras una pregunta está en vuelo, la respuesta de la conversación
  // anterior no se inyecta en la nueva.
  const activeConvRef = useRef(conversationId)
  activeConvRef.current = conversationId

  useEffect(() => {
    setError(null)
    if (!conversationId) {
      setMessages([])
      setLoadingHistory(false)
      return
    }
    let cancelled = false
    setLoadingHistory(true)
    invoke<GlobalChatMessage[]>("get_global_chat_history", { conversationId })
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
  }, [conversationId])

  const ask = useCallback(
    async (question: string, model = "mimo-v2.6-flash") => {
      if (!conversationId || !question.trim()) return
      const requestedConv = conversationId

      setError(null)
      setSending(true)

      const userMessage: GlobalChatMessage = {
        role: "user",
        content: question,
        created_at: new Date().toISOString(),
      }
      setMessages((prev) => [...prev, userMessage])

      try {
        const result = await invoke<GlobalAnswer>("ask_global_question", {
          conversationId,
          question,
          model,
        })
        if (requestedConv !== activeConvRef.current) return
        setMessages((prev) => [
          ...prev,
          {
            role: "assistant",
            content: result.answer,
            created_at: new Date().toISOString(),
            sources: result.sources,
          },
        ])
      } catch (err) {
        if (requestedConv !== activeConvRef.current) return
        const message = err instanceof Error ? err.message : String(err)
        setError(message)
        // La pregunta optimista quedó sin respuesta: la quitamos para que el
        // usuario pueda reintentarla sin ver un mensaje "colgado".
        setMessages((prev) => prev.filter((m) => m !== userMessage))
      } finally {
        if (requestedConv === activeConvRef.current) setSending(false)
      }
    },
    [conversationId]
  )

  return { messages, loadingHistory, sending, error, ask }
}
