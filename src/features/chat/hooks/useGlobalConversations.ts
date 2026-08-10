import { useCallback, useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"

export interface Conversation {
  id: string
  title: string
  created_at: string
}

export function useGlobalConversations() {
  const [conversations, setConversations] = useState<Conversation[]>([])
  const [activeId, setActiveId] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  const refresh = useCallback(async () => {
    const list = await invoke<Conversation[]>("list_global_conversations")
    setConversations(list)
    return list
  }, [])

  const create = useCallback(async () => {
    const conv = await invoke<Conversation>("create_global_conversation")
    setConversations((prev) => [conv, ...prev])
    setActiveId(conv.id)
    return conv
  }, [])

  const rename = useCallback(async (id: string, title: string) => {
    setConversations((prev) => prev.map((c) => (c.id === id ? { ...c, title } : c)))
    await invoke("rename_global_conversation", { conversationId: id, title }).catch(() => {})
  }, [])

  const remove = useCallback(
    async (id: string) => {
      setConversations((prev) => prev.filter((c) => c.id !== id))
      setActiveId((cur) => (cur === id ? null : cur))
      await invoke("delete_global_conversation", { conversationId: id }).catch(() => {})
    },
    []
  )

  // Título en vivo: el backend lo auto-nombra con la primera pregunta, así que
  // tras enviar un mensaje refrescamos la lista para reflejarlo.
  const syncTitles = refresh

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      try {
        let list = await invoke<Conversation[]>("list_global_conversations")
        // Arranca siempre con al menos una conversación para escribir.
        if (list.length === 0) {
          const conv = await invoke<Conversation>("create_global_conversation")
          list = [conv]
        }
        if (!cancelled) {
          setConversations(list)
          setActiveId(list[0].id)
        }
      } finally {
        if (!cancelled) setLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  return { conversations, activeId, setActiveId, loading, create, rename, remove, syncTitles }
}
