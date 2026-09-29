import { useCallback, useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"

export interface Conversation {
  id: string
  title: string
  created_at: string
}

const messageOf = (err: unknown) => (err instanceof Error ? err.message : String(err))

export function useGlobalConversations() {
  const [conversations, setConversations] = useState<Conversation[]>([])
  const [activeId, setActiveId] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      const list = await invoke<Conversation[]>("list_global_conversations")
      setConversations(list)
      setError(null)
      return list
    } catch (err) {
      setError(messageOf(err))
      return []
    }
  }, [])

  const create = useCallback(async () => {
    try {
      const conv = await invoke<Conversation>("create_global_conversation")
      setConversations((prev) => [conv, ...prev])
      setActiveId(conv.id)
      setError(null)
      return conv
    } catch (err) {
      setError(messageOf(err))
      return null
    }
  }, [])

  // Actualización optimista con resincronización desde el backend si falla
  // (el backend es la fuente de verdad; evita dejar la UI desincronizada).
  const rename = useCallback(
    async (id: string, title: string) => {
      setConversations((prev) => prev.map((c) => (c.id === id ? { ...c, title } : c)))
      try {
        await invoke("rename_global_conversation", { conversationId: id, title })
        setError(null)
      } catch (err) {
        setError(messageOf(err))
        await refresh()
      }
    },
    [refresh]
  )

  const remove = useCallback(
    async (id: string) => {
      setConversations((prev) => prev.filter((c) => c.id !== id))
      setActiveId((cur) => (cur === id ? null : cur))
      try {
        await invoke("delete_global_conversation", { conversationId: id })
        setError(null)
      } catch (err) {
        setError(messageOf(err))
        await refresh()
      }
    },
    [refresh]
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
      } catch (err) {
        if (!cancelled) setError(messageOf(err))
      } finally {
        if (!cancelled) setLoading(false)
      }
    })()
    return () => {
      cancelled = true
    }
  }, [])

  return {
    conversations,
    activeId,
    setActiveId,
    loading,
    error,
    create,
    rename,
    remove,
    syncTitles,
  }
}
