import { AudioLines, ListMusic, Mic, Sparkles } from "lucide-react"
import { motion } from "motion/react"
import { Button } from "../../../components/ui/Button"
import { EmptyState } from "../../../components/ui/EmptyState"
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

export function StudioView({
  meetings,
  onStartRecording,
}: {
  meetings: Meeting[]
  onStartRecording: () => void
}) {
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

      {/* Recent meetings */}
      <div>
        <div className="mb-3 flex items-center justify-between">
          <p className="font-mono text-[11px] uppercase tracking-[0.14em] text-ink-mute">
            Reuniones recientes
          </p>
        </div>

        {meetings.length === 0 ? (
          <EmptyState
            icon={<ListMusic />}
            title="Aún no hay reuniones"
            description="Cuando grabes tu primera reunión, aparecerá aquí junto con su estado de procesamiento."
          />
        ) : (
          <div className="space-y-2">
            {meetings.map((meeting) => (
              <Panel key={meeting.id} className="flex items-center gap-3 px-4 py-3">
                <StatusDot
                  tone={meetingStateTone[meeting.state]}
                  pulse={meeting.state === "recording"}
                />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium text-ink">{meeting.title}</p>
                  <p className="font-mono text-[11px] text-ink-mute">
                    {relativeTime(meeting.created_at)}
                  </p>
                </div>
                <span className="font-mono text-[11px] text-ink-dim">
                  {meetingStateLabel[meeting.state]}
                </span>
              </Panel>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
