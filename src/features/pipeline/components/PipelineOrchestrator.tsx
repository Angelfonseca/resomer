import { useEffect, useRef } from "react"
import { AlertCircle, Play, RotateCcw, Sparkles, Workflow } from "lucide-react"
import { motion } from "motion/react"
import { usePipeline } from "../hooks/usePipeline"
import { SpeakerTimeline } from "./SpeakerTimeline"
import { TextResultPanel } from "./TextResultPanel"
import { Button } from "../../../components/ui/Button"
import { Panel, PanelHeader, PanelBody } from "../../../components/ui/Panel"
import { SignalRail, type RailStep } from "../../../components/ui/SignalRail"
import { StatusDot } from "../../../components/ui/StatusDot"
import { EmptyState } from "../../../components/ui/EmptyState"

const STEPS: RailStep[] = [
  { key: "diarizing", label: "Diarizar" },
  { key: "transcribing", label: "Transcribir" },
  { key: "summarizing", label: "Resumir" },
]

const STEP_LABEL: Record<string, string> = {
  diarizing: "Detectando hablantes…",
  transcribing: "Transcribiendo audio…",
  summarizing: "Generando resumen…",
}

export function PipelineOrchestrator({ audioPath, meetingId }: { audioPath?: string; meetingId?: string }) {
  const { state, runPipeline, reset, loadExisting, loadingExisting } = usePipeline()

  // Al entrar con una reunión existente, carga lo que ya haya guardado
  // (transcripción/resumen) en vez de asumir que hay que grabar/ejecutar de
  // cero. Solo una vez por reunión (loadedFor evita relanzar en cada render).
  const loadedFor = useRef<string | null>(null)
  useEffect(() => {
    if (!audioPath || !meetingId) return
    if (loadedFor.current === meetingId) return
    loadedFor.current = meetingId
    loadExisting(meetingId, audioPath)
  }, [audioPath, meetingId, loadExisting])

  const currentIndex =
    state.step === "idle"
      ? -1
      : state.step === "complete"
        ? STEPS.length
        : state.step === "error"
          ? [state.segments, state.transcript, state.summary].filter(Boolean).length
          : STEPS.findIndex((s) => s.key === state.step)

  const isRunning = state.step !== "idle" && state.step !== "complete" && state.step !== "error"

  if (!audioPath) {
    return (
      <EmptyState
        icon={<Workflow />}
        title="Sin audio para procesar"
        description="Graba una reunión primero — el pipeline se activará automáticamente cuando termine."
      />
    )
  }

  if (loadingExisting) {
    return (
      <div className="flex items-center justify-center gap-2 py-16">
        <StatusDot tone="signal" pulse />
        <span className="font-mono text-xs text-ink-dim">Cargando reunión…</span>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <Panel raised>
        <PanelHeader
          eyebrow="Pipeline"
          title="Cadena de señal"
          action={
            (state.step === "complete" || state.step === "error") && (
              <Button
                variant="subtle"
                size="sm"
                leftIcon={<RotateCcw className="size-3.5" />}
                onClick={reset}
              >
                Nueva ejecución
              </Button>
            )
          }
        />
        <PanelBody className="space-y-6">
          <SignalRail steps={STEPS} currentIndex={currentIndex} />

          {state.step === "error" && (
            <div className="flex items-start gap-2 rounded-control border border-rec/30 bg-rec-dim px-4 py-3 text-sm text-ink">
              <AlertCircle className="mt-0.5 size-4 shrink-0 text-rec" />
              <span>{state.error}</span>
            </div>
          )}

          {state.step === "idle" && (
            <Button leftIcon={<Play className="size-4" />} onClick={() => runPipeline(audioPath, meetingId)}>
              Ejecutar pipeline
            </Button>
          )}

          {isRunning && (
            <div className="flex items-center gap-2">
              <StatusDot tone="signal" pulse />
              <span className="font-mono text-xs text-ink-dim">{STEP_LABEL[state.step]}</span>
            </div>
          )}

          {state.step === "complete" && (
            <div className="flex items-center gap-2">
              <StatusDot tone="done" />
              <span className="font-mono text-xs text-done">Pipeline completado</span>
            </div>
          )}
        </PanelBody>
      </Panel>

      {state.segments && state.segments.length > 0 && (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}>
          <Panel>
            <PanelHeader
              eyebrow="Diarización"
              title={`${new Set(state.segments.map((s) => s.speaker)).size} hablante(s) · ${state.segments.length} segmentos`}
            />
            <PanelBody>
              <SpeakerTimeline segments={state.segments} />
            </PanelBody>
          </Panel>
        </motion.div>
      )}

      {state.transcript && (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}>
          <Panel>
            <PanelHeader eyebrow="Transcripción" title="Texto completo" />
            <PanelBody>
              <TextResultPanel text={state.transcript} filename="transcripcion.txt" />
            </PanelBody>
          </Panel>
        </motion.div>
      )}

      {state.summary && (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}>
          <Panel raised>
            <PanelHeader
              eyebrow="Resumen"
              title="Resumen ejecutivo"
              action={
                <div className="flex size-9 items-center justify-center rounded-md bg-signal-dim text-signal">
                  <Sparkles className="size-[18px]" strokeWidth={2} />
                </div>
              }
            />
            <PanelBody>
              <TextResultPanel text={state.summary} filename="resumen.md" markdown />
            </PanelBody>
          </Panel>
        </motion.div>
      )}
    </div>
  )
}
