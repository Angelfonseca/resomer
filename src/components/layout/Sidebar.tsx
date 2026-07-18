import { AudioLines, Mic, Workflow, Settings2 } from "lucide-react"
import { motion } from "motion/react"
import { cn } from "../../lib/cn"
import { StatusDot } from "../ui/StatusDot"
import type { Meeting } from "../../features/meetings/hooks/useMeetings"

export type View = "studio" | "recording" | "pipeline" | "settings"

const NAV_ITEMS: { id: View; label: string; icon: typeof AudioLines }[] = [
  { id: "studio", label: "Studio", icon: AudioLines },
  { id: "recording", label: "Grabar", icon: Mic },
  { id: "pipeline", label: "Pipeline", icon: Workflow },
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
}: {
  currentView: View
  onNavigate: (view: View) => void
  meetings: Meeting[]
  pipelineEnabled: boolean
  hasApiKey: boolean | null
}) {
  return (
    <aside className="flex w-60 shrink-0 flex-col border-r border-hairline bg-canvas-raised">
      {/* Brand */}
      <div className="flex items-center gap-2.5 px-5 py-5">
        <div className="flex size-8 items-center justify-center rounded-md border border-signal/40 bg-signal-dim font-display text-sm font-bold text-signal">
          R
        </div>
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

      {/* Meetings library */}
      <div className="mt-6 flex min-h-0 flex-1 flex-col px-3">
        <p className="px-3 pb-2 font-mono text-[10px] uppercase tracking-[0.14em] text-ink-mute">
          Reuniones
        </p>
        <div className="flex-1 space-y-0.5 overflow-y-auto">
          {meetings.length === 0 && (
            <p className="px-3 py-2 text-xs text-ink-mute">
              Tus grabaciones aparecerán aquí.
            </p>
          )}
          {meetings.map((meeting) => (
            <div
              key={meeting.id}
              className="group flex items-center gap-2.5 rounded-md px-3 py-2 hover:bg-panel-hover"
            >
              <StatusDot tone={meetingStateTone[meeting.state]} pulse={meeting.state === "recording"} />
              <div className="min-w-0 flex-1">
                <p className="truncate text-xs font-medium text-ink">{meeting.title}</p>
                <p className="font-mono text-[10px] text-ink-mute">
                  {relativeTime(meeting.created_at)}
                </p>
              </div>
            </div>
          ))}
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
