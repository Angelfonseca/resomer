import { useMemo } from "react"
import { AudioLines, CalendarClock, CheckCircle2, Mic, Sparkles, Workflow, Settings2, Folder } from "lucide-react"
import { motion, AnimatePresence } from "motion/react"
import { cn } from "../../lib/cn"
import { StatusDot } from "../ui/StatusDot"
import type { Meeting } from "../../features/meetings/hooks/useMeetings"
import { useSidebarActivity } from "../../features/meetings/hooks/useSidebarActivity"
import logo from "../../assets/logo.png"

export type View = "studio" | "recording" | "pipeline" | "meetings" | "assistant" | "settings"

const NAV_ITEMS: { id: View; label: string; icon: typeof AudioLines }[] = [
  { id: "studio", label: "Studio", icon: AudioLines },
  { id: "recording", label: "Grabar", icon: Mic },
  { id: "pipeline", label: "Pipeline", icon: Workflow },
  { id: "meetings", label: "Reuniones", icon: CalendarClock },
  { id: "assistant", label: "Asistente", icon: Sparkles },
  { id: "settings", label: "Ajustes", icon: Settings2 },
]

function relativeTime(iso: string): string {
  const diffMs = Date.now() - new Date(iso).getTime()
  const mins = Math.round(diffMs / 60000)
  if (mins < 1) return "ahora"
  if (mins < 60) return `hace ${mins} min`
  const hours = Math.round(mins / 60)
  if (hours < 24) return `hace ${hours} h`
  return `hace ${Math.round(hours / 24)} d`
}

const meetingStateTone: Record<Meeting["state"], "signal" | "rec" | "done" | "idle"> = {
  recording: "rec",
  processing: "signal",
  completed: "done",
  error: "idle",
}

export function Sidebar({
  currentView,
  onNavigate,
  meetings,
  pipelineEnabled,
  hasApiKey,
  onOpenMeeting,
}: {
  currentView: View
  onNavigate: (view: View) => void
  meetings: Meeting[]
  pipelineEnabled: boolean
  hasApiKey: boolean | null
  onOpenMeeting?: (meeting: Meeting) => void
}) {
  // Solo actividad en curso (grabando/procesando/error) más las que acaban
  // de completarse (con aviso, 20s) — la lista completa, con búsqueda y
  // filtros, vive en la vista "Reuniones".
  const activity = useSidebarActivity(meetings)

  const availableCategories = useMemo(() => {
    const set = new Set(["Clientes", "Interno", "Personal"])
    for (const m of meetings) if (m.category) set.add(m.category)
    return [...set].sort((a, b) => a.localeCompare(b))
  }, [meetings])

  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-hairline bg-canvas-raised">
      {/* Brand */}
      <div className="flex items-center gap-2.5 px-5 py-5">
        <img src={logo} alt="Resomer" className="size-9 shrink-0 rounded-md object-contain" />
        <span className="font-display text-[15px] font-semibold tracking-tight text-ink">
          Resomer
        </span>
      </div>

      {/* Primary nav */}
      <nav className="flex flex-col gap-0.5 px-3">
        {NAV_ITEMS.map((item) => {
          const isActive = currentView === item.id
          const isDisabled = item.id === "pipeline" && !pipelineEnabled
          const Icon = item.icon
          return (
            <button
              key={item.id}
              onClick={() => !isDisabled && onNavigate(item.id)}
              disabled={isDisabled}
              className={cn(
                "relative flex items-center gap-3 rounded-md px-3 py-2 text-left text-sm transition-colors",
                isActive ? "text-ink" : "text-ink-dim hover:text-ink hover:bg-panel-hover",
                isDisabled && "opacity-35 cursor-not-allowed hover:bg-transparent hover:text-ink-dim"
              )}
            >
              {isActive && (
                <motion.div
                  layoutId="sidebar-active"
                  className="absolute inset-0 rounded-md bg-panel-hi"
                  transition={{ type: "spring", stiffness: 500, damping: 40 }}
                />
              )}
              {isActive && (
                <motion.div
                  layoutId="sidebar-active-bar"
                  className="absolute left-0 top-1.5 bottom-1.5 w-0.5 rounded-full bg-signal"
                  transition={{ type: "spring", stiffness: 500, damping: 40 }}
                />
              )}
              <Icon className="relative size-4 shrink-0" strokeWidth={2} />
              <span className="relative font-medium">{item.label}</span>
            </button>
          )
        })}
      </nav>

      {/* Categorías */}
      <div className="mt-6 flex flex-col px-3">
        <div className="flex items-center justify-between px-3 pb-2">
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-ink-mute">
            Categorías
          </span>
        </div>
        <div className="space-y-0.5">
          {availableCategories.map((cat) => (
            <button
              key={cat}
              onClick={() => {
                onNavigate("meetings")
                // TODO: Idealmente aquí podríamos pasar el filtro a MeetingsView
              }}
              className="group flex w-full items-center gap-3 rounded-md px-3 py-2 text-left hover:bg-panel-hover"
            >
              <Folder className="size-3.5 shrink-0 text-ink-mute group-hover:text-ink-dim" />
              <span className="truncate text-xs font-medium text-ink-dim group-hover:text-ink">
                {cat}
              </span>
            </button>
          ))}
        </div>
      </div>

      {/* En curso: solo actividad activa/reciente — la biblioteca completa
          vive en "Reuniones". */}
      <div className="mt-6 flex min-h-0 flex-1 flex-col px-3">
        <button
          onClick={() => onNavigate("meetings")}
          className="flex items-center justify-between px-3 pb-2 text-left hover:text-ink"
        >
          <span className="font-mono text-[10px] uppercase tracking-[0.14em] text-ink-mute">
            En curso
          </span>
          {meetings.length > 0 && (
            <span className="font-mono text-[10px] text-ink-mute">Ver todas</span>
          )}
        </button>
        <div className="flex-1 space-y-0.5 overflow-y-auto">
          {activity.length === 0 && (
            <p className="px-3 py-2 text-xs text-ink-mute">
              No hay grabaciones activas.
            </p>
          )}
          <AnimatePresence initial={false}>
            {activity.map(({ meeting, justFinished }) => {
              const canOpen = Boolean(
                onOpenMeeting && meeting.audio_path && meeting.state !== "recording"
              )
              return (
                <motion.button
                  key={meeting.id}
                  layout
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  exit={{ opacity: 0 }}
                  onClick={canOpen ? () => onOpenMeeting?.(meeting) : undefined}
                  disabled={!canOpen}
                  className={cn(
                    "group flex w-full items-center gap-2.5 rounded-md px-3 py-2 text-left",
                    canOpen ? "hover:bg-panel-hover cursor-pointer" : "cursor-default opacity-70"
                  )}
                >
                  <StatusDot
                    tone={meetingStateTone[meeting.state]}
                    pulse={meeting.state === "recording" || meeting.state === "processing"}
                  />
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-xs font-medium text-ink">{meeting.title}</p>
                    {justFinished ? (
                      <p className="flex items-center gap-1 font-mono text-[10px] text-done">
                        <CheckCircle2 className="size-3" strokeWidth={2} />
                        Pipeline terminado
                      </p>
                    ) : meeting.state === "processing" ? (
                      <p className="font-mono text-[10px] text-signal">Procesando…</p>
                    ) : meeting.state === "error" ? (
                      <p className="font-mono text-[10px] text-rec">Error · reintentar</p>
                    ) : (
                      <p className="font-mono text-[10px] text-ink-mute">
                        {relativeTime(meeting.created_at)}
                      </p>
                    )}
                  </div>
                </motion.button>
              )
            })}
          </AnimatePresence>
        </div>
      </div>

      {/* Status chip */}
      <button
        onClick={() => onNavigate("settings")}
        className="flex items-center gap-2 border-t border-hairline px-5 py-4 text-left hover:bg-panel-hover"
      >
        <StatusDot tone={hasApiKey ? "done" : "idle"} />
        <span className="font-mono text-[11px] text-ink-dim">
          {hasApiKey === null ? "Verificando…" : hasApiKey ? "API conectada" : "Sin configurar"}
        </span>
      </button>
    </aside>
  )
}
