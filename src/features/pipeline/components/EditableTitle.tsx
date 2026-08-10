import { useState } from "react"
import { Check, Pencil, X } from "lucide-react"

/**
 * Título de reunión editable in-place. La IA lo genera al hacer el resumen y el
 * usuario puede corregirlo. Persiste vía el comando `update_meeting_title`.
 */
export function EditableTitle({
  title,
  onSave,
}: {
  title: string
  onSave: (title: string) => void
}) {
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(title)

  const start = () => {
    setDraft(title)
    setEditing(true)
  }
  const commit = () => {
    const t = draft.trim()
    if (t && t !== title) onSave(t)
    setEditing(false)
  }

  if (editing) {
    return (
      <div className="flex items-center gap-2">
        <input
          autoFocus
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit()
            if (e.key === "Escape") setEditing(false)
          }}
          className="min-w-0 flex-1 rounded-control border border-hairline-strong bg-canvas-raised px-3 py-1.5 font-display text-lg font-semibold text-ink focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"
        />
        <button aria-label="Guardar título" onClick={commit} className="text-ink-mute hover:text-signal">
          <Check className="size-4" />
        </button>
        <button aria-label="Cancelar" onClick={() => setEditing(false)} className="text-ink-mute hover:text-ink">
          <X className="size-4" />
        </button>
      </div>
    )
  }

  return (
    <button
      onClick={start}
      className="group flex items-center gap-2 text-left"
      title="Editar título"
    >
      <span className="font-display text-lg font-semibold text-ink">{title}</span>
      <Pencil className="size-3.5 text-ink-mute opacity-0 transition-opacity group-hover:opacity-100" />
    </button>
  )
}
