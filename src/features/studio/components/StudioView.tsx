import { useMemo, useState } from "react"
import { ArrowDownAZ, ArrowUpAZ, AudioLines, ListMusic, Mic, Search, Sparkles, Trash2, X } from "lucide-react"
import { motion, AnimatePresence } from "motion/react"
import { Button } from "../../../components/ui/Button"
import { EmptyState } from "../../../components/ui/EmptyState"
import { IconButton } from "../../../components/ui/IconButton"
import { Panel } from "../../../components/ui/Panel"
import { StatusDot } from "../../../components/ui/StatusDot"
import type { Meeting } from "../../meetings/hooks/useMeetings"

const SIGNAL_MODULES = [
  {
    icon: Mic,
    title: "Grabación",
    description: "Captura micrófono y audio del sistema en WAV de alta fidelidad.",
  },
  {
    icon: AudioLines,
    title: "Diarización",
    description: "Detecta quién habla y cuándo, localmente y en privado.",
  },
  {
    icon: Sparkles,
    title: "Resumen",
    description: "Transcribe y resume con IA en segundos, listo para compartir.",
  },
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

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("es", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  })
}

const meetingStateLabel: Record<Meeting["state"], string> = {
  recording: "Grabando",
  processing: "Procesando",
  completed: "Completa",
  error: "Error",
}

const meetingStateTone: Record<Meeting["state"], "signal" | "rec" | "done" | "idle"> = {
  recording: "rec",
  processing: "signal",
  completed: "done",
  error: "idle",
}

function MeetingRow({
  meeting,
  onDelete,
}: {
  meeting: Meeting
  onDelete?: (id: string) => void
}) {
  const [confirming, setConfirming] = useState(false)

  return (
    <Panel className="flex items-center gap-3 px-4 py-3 transition-colors hover:bg-panel-hi">
      <StatusDot tone={meetingStateTone[meeting.state]} pulse={meeting.state === "recording"} />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-ink">{meeting.title}</p>
        <p className="font-mono text-[11px] text-ink-mute">
          {formatDate(meeting.created_at)} · {relativeTime(meeting.created_at)}
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
          >
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

export function StudioView({
  meetings,
  onStartRecording,
  onDeleteMeeting,
}: {
  meetings: Meeting[]
  onStartRecording: () => void
  onDeleteMeeting?: (id: string) => void
}) {
  const [query, setQuery] = useState("")
  const [sortAsc, setSortAsc] = useState(false)

  const filteredSorted = useMemo(() => {
    const q = query.trim().toLowerCase()
    const filtered = q
      ? meetings.filter((m) => m.title.toLowerCase().includes(q))
      : meetings
    return [...filtered].sort((a, b) => {
      const diff = new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      return sortAsc ? diff : -diff
    })
  }, [meetings, query, sortAsc])

  const completedMeetings = filteredSorted.filter((m) => m.state === "completed")
  const processingMeetings = filteredSorted.filter((m) => m.state !== "completed")
  const hasAnyMeetings = meetings.length > 0
  const hasResults = filteredSorted.length > 0

  return (
    <div className="mx-auto w-full max-w-4xl space-y-8">
      {/* Hero */}
      <Panel raised className="relative overflow-hidden px-8 py-10 text-center">
        <div
          className="pointer-events-none absolute inset-x-0 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--color-signal), transparent)" }}
        />
        <motion.div
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4 }}
          className="mx-auto mb-6 flex size-16 items-center justify-center rounded-full bg-signal-dim"
        >
          <span className="relative flex size-3">
            <StatusDot tone="signal" pulse className="size-3" />
          </span>
        </motion.div>
        <p className="mb-2 font-mono text-xs uppercase tracking-[0.16em] text-signal">
          Listo para grabar
        </p>
        <h1 className="mb-3 font-display text-3xl font-semibold text-ink">
          Tu próxima reunión, capturada y resumida
        </h1>
        <p className="mx-auto mb-7 max-w-md text-sm text-ink-dim">
          Graba, diariza, transcribe y resume automáticamente — todo en un
          único flujo de trabajo.
        </p>
        <Button size="lg" leftIcon={<Mic className="size-4" />} onClick={onStartRecording}>
          Nueva grabación
        </Button>
      </Panel>

      {/* Signal chain modules */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
        {SIGNAL_MODULES.map((mod, i) => (
          <motion.div
            key={mod.title}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.35, delay: 0.05 * i }}
          >
            <Panel className="h-full p-5">
              <div className="mb-3 flex size-9 items-center justify-center rounded-md bg-signal-dim text-signal">
                <mod.icon className="size-[18px]" strokeWidth={2} />
              </div>
              <h3 className="mb-1 font-display text-sm font-semibold text-ink">{mod.title}</h3>
              <p className="text-xs leading-relaxed text-ink-dim">{mod.description}</p>
            </Panel>
          </motion.div>
        ))}
      </div>

      {/* Search + sort controls */}
      {hasAnyMeetings && (
        <div className="flex items-center gap-2">
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-ink-mute" />
            <input
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Buscar reuniones por título…"
              className="w-full rounded-control border border-hairline-strong bg-canvas-raised py-2 pl-9 pr-3 text-sm text-ink placeholder:text-ink-mute transition-colors focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"
            />
          </div>
          <IconButton
            aria-label={sortAsc ? "Ordenar: más antiguas primero" : "Ordenar: más recientes primero"}
            icon={sortAsc ? <ArrowUpAZ /> : <ArrowDownAZ />}
            variant="solid"
            onClick={() => setSortAsc((v) => !v)}
          />
        </div>
      )}

      {/* Processing/Recording meetings */}
      {processingMeetings.length > 0 && (
        <div>
          <div className="mb-3">
            <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-signal">
              En proceso
            </p>
          </div>
          <div className="space-y-2">
            {processingMeetings.map((meeting) => (
              <MeetingRow key={meeting.id} meeting={meeting} onDelete={onDeleteMeeting} />
            ))}
          </div>
        </div>
      )}

      {/* Completed meetings */}
      {completedMeetings.length > 0 && (
        <div>
          <div className="mb-3">
            <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-done">
              Completadas
            </p>
          </div>
          <div className="space-y-2">
            {completedMeetings.map((meeting) => (
              <MeetingRow key={meeting.id} meeting={meeting} onDelete={onDeleteMeeting} />
            ))}
          </div>
        </div>
      )}

      {/* No search results */}
      {hasAnyMeetings && !hasResults && (
        <EmptyState
          icon={<Search />}
          title="Sin resultados"
          description={`Ninguna reunión coincide con "${query}".`}
        />
      )}

      {/* Empty state */}
      {!hasAnyMeetings && (
        <EmptyState
          icon={<ListMusic />}
          title="Aún no hay reuniones"
          description="Cuando grabes tu primera reunión, aparecerá aquí junto con su estado de procesamiento."
        />
      )}
    </div>
  )
}
