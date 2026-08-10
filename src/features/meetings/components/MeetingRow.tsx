import { useState } from "react"
import { Trash2, X } from "lucide-react"
import { motion, AnimatePresence } from "motion/react"
import { IconButton } from "../../../components/ui/IconButton"
import { Panel } from "../../../components/ui/Panel"
import { StatusDot } from "../../../components/ui/StatusDot"
import type { Meeting } from "../hooks/useMeetings"

function relativeTime(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime()
  const mins = Math.round(diffMs / 60000)
  if (mins < 1) return "ahora"
  if (mins < 60) return `hace ${mins} min`
  const hours = Math.round(mins / 60)
  if (hours < 24) return `hace ${hours} h`
  return `hace ${Math.round(hours / 24)} d`
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("es", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  })
}

export const meetingStateLabel: Record<Meeting["state"], string> = {
  recording: "Grabando",
  processing: "Procesando",
  completed: "Completa",
  error: "Error",
}

export const meetingStateTone: Record<Meeting["state"], "signal" | "rec" | "done" | "idle"> = {
  recording: "rec",
  processing: "signal",
  completed: "done",
  error: "idle",
}

export function MeetingRow({
  meeting,
  onDelete,
  onOpen,
  categories,
  onSetCategory,
}: {
  meeting: Meeting
  onDelete?: (id: string) => void
  onOpen?: (meeting: Meeting) => void
  categories?: string[]
  onSetCategory?: (id: string, category: string | null) => void
}) {
  const [confirming, setConfirming] = useState(false)
  const [isCreatingCategory, setIsCreatingCategory] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState("")

  // Solo se puede abrir una reunión que ya tenga audio grabado y no esté
  // grabándose en este momento (el WAV aún se está escribiendo).
  const canOpen = Boolean(onOpen && meeting.audio_path && meeting.state !== "recording")

  return (
    <Panel
      onClick={canOpen ? () => onOpen?.(meeting) : undefined}
      className={
        "flex items-center gap-3 px-4 py-3 transition-colors hover:bg-panel-hi" +
        (canOpen ? " cursor-pointer" : "")
      }
    >
      <StatusDot tone={meetingStateTone[meeting.state]} pulse={meeting.state === "recording"} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-ink">{meeting.title}</p>
        <p className="font-mono text-[11px] text-ink-mute">
          {formatDate(meeting.created_at)} · {relativeTime(meeting.created_at)}
          {meeting.expected_speakers && ` · ${meeting.expected_speakers} participantes`}
        </p>
      </div>

      <AnimatePresence mode="wait" initial={false}>
        {confirming ? (
          <motion.div
            key="confirm"
            initial={{ opacity: 0, x: 6 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: 6 }}
            className="flex items-center gap-1.5"
            onClick={(e) => e.stopPropagation()}
          >
            <span className="font-mono text-[11px] text-ink-mute">¿Borrar?</span>
            <button
              onClick={() => {
                setConfirming(false)
                onDelete?.(meeting.id)
              }}
              className="rounded-control border border-rec/40 bg-rec-dim px-2 py-1 font-mono text-[11px] text-rec hover:bg-rec/20"
            >
              Sí
            </button>
            <IconButton
              aria-label="Cancelar"
              icon={<X />}
              size="sm"
              variant="ghost"
              onClick={() => setConfirming(false)}
            />
          </motion.div>
        ) : (
          <motion.div
            key="info"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="flex items-center gap-2"
            onClick={(e) => e.stopPropagation()}
          >
            {onSetCategory && (
              isCreatingCategory ? (
                <input
                  autoFocus
                  type="text"
                  placeholder="Nueva categoría…"
                  value={newCategoryName}
                  onChange={(e) => setNewCategoryName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && newCategoryName.trim()) {
                      e.preventDefault()
                      onSetCategory(meeting.id, newCategoryName.trim())
                      setIsCreatingCategory(false)
                      setNewCategoryName("")
                    }
                    if (e.key === "Escape") {
                      setIsCreatingCategory(false)
                      setNewCategoryName("")
                    }
                  }}
                  onBlur={() => {
                    if (newCategoryName.trim()) {
                      onSetCategory(meeting.id, newCategoryName.trim())
                    }
                    setIsCreatingCategory(false)
                    setNewCategoryName("")
                  }}
                  className="w-28 rounded-control border border-signal/40 bg-canvas-raised px-1.5 py-1 font-mono text-[10px] text-ink placeholder:text-ink-mute focus:border-signal focus:outline-none"
                />
              ) : (
                <select
                  value={meeting.category ?? ""}
                  onChange={(e) => {
                    const value = e.target.value
                    if (value === "__new__") {
                      setIsCreatingCategory(true)
                      return
                    }
                    onSetCategory(meeting.id, value || null)
                  }}
                  className="w-28 rounded-control border border-hairline bg-canvas-raised px-1.5 py-1 font-mono text-[10px] text-ink-dim"
                >
                  <option value="">Sin categoría</option>
                  {categories?.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                  <option value="__new__">+ Nueva…</option>
                </select>
              )
            )}
            <span
              className={
                "font-mono text-[11px] " +
                (meeting.state === "completed" ? "text-done" : "text-ink-dim")
              }
            >
              {meetingStateLabel[meeting.state]}
            </span>
            {onDelete && (
              <IconButton
                aria-label="Borrar reunión"
                icon={<Trash2 />}
                size="sm"
                variant="ghost"
                className="text-ink-mute hover:text-rec"
                onClick={() => setConfirming(true)}
              />
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </Panel>
  )
}
