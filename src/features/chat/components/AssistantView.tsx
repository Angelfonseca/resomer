import { useState } from "react"
import { Check, MessageSquarePlus, Pencil, Trash2, X } from "lucide-react"
import { cn } from "../../../lib/cn"
import { GlobalAssistant } from "./GlobalAssistant"
import { useGlobalConversations } from "../hooks/useGlobalConversations"

export function AssistantView({
  onOpenMeetingById,
}: {
  onOpenMeetingById?: (meetingId: string) => void
}) {
  const { conversations, activeId, setActiveId, loading, create, rename, remove, syncTitles } =
    useGlobalConversations()
  const [editingId, setEditingId] = useState<string | null>(null)
  const [draft, setDraft] = useState("")

  const startEdit = (id: string, title: string) => {
    setEditingId(id)
    setDraft(title)
  }
  const commitEdit = () => {
    if (editingId && draft.trim()) rename(editingId, draft.trim())
    setEditingId(null)
  }

  const active = conversations.find((c) => c.id === activeId)

  return (
    <div className="mx-auto flex w-full max-w-5xl gap-6">
      {/* Riel de conversaciones */}
      <aside className="w-56 shrink-0 space-y-2">
        <button
          onClick={() => create()}
          className="flex w-full items-center gap-2 rounded-control border border-signal/30 bg-signal-dim px-3 py-2 text-sm font-medium text-signal transition-colors hover:border-signal/60"
        >
          <MessageSquarePlus className="size-4" />
          Nuevo chat
        </button>

        <div className="space-y-0.5">
          {loading && <p className="px-3 py-2 text-xs text-ink-mute">Cargando…</p>}
          {conversations.map((c) => {
            const isActive = c.id === activeId
            const isEditing = c.id === editingId
            return (
              <div
                key={c.id}
                className={cn(
                  "group flex items-center gap-1 rounded-md px-2 py-1.5 text-left transition-colors",
                  isActive ? "bg-panel-hi" : "hover:bg-panel-hover"
                )}
              >
                {isEditing ? (
                  <>
                    <input
                      autoFocus
                      value={draft}
                      onChange={(e) => setDraft(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") commitEdit()
                        if (e.key === "Escape") setEditingId(null)
                      }}
                      className="min-w-0 flex-1 rounded border border-hairline-strong bg-canvas-raised px-2 py-1 text-xs text-ink focus:border-signal focus:outline-none"
                    />
                    <button aria-label="Guardar" onClick={commitEdit} className="text-ink-mute hover:text-signal">
                      <Check className="size-3.5" />
                    </button>
                    <button aria-label="Cancelar" onClick={() => setEditingId(null)} className="text-ink-mute hover:text-ink">
                      <X className="size-3.5" />
                    </button>
                  </>
                ) : (
                  <>
                    <button
                      onClick={() => setActiveId(c.id)}
                      className="min-w-0 flex-1 truncate text-xs text-ink-dim group-hover:text-ink"
                      title={c.title}
                    >
                      {c.title}
                    </button>
                    <button
                      aria-label="Renombrar chat"
                      onClick={() => startEdit(c.id, c.title)}
                      className="text-ink-mute opacity-0 transition-opacity hover:text-ink group-hover:opacity-100"
                    >
                      <Pencil className="size-3.5" />
                    </button>
                    <button
                      aria-label="Borrar chat"
                      onClick={() => remove(c.id)}
                      className="text-ink-mute opacity-0 transition-opacity hover:text-rec group-hover:opacity-100"
                    >
                      <Trash2 className="size-3.5" />
                    </button>
                  </>
                )}
              </div>
            )
          })}
        </div>
      </aside>

      {/* Chat activo */}
      <div className="min-w-0 flex-1">
        <GlobalAssistant
          conversationId={activeId ?? undefined}
          conversationTitle={active?.title}
          onOpenMeetingById={onOpenMeetingById}
          onAsked={syncTitles}
        />
      </div>
    </div>
  )
}
