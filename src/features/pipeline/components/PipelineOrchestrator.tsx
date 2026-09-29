import { useEffect, useRef, useState } from "react"
import { AlertCircle, Play, RotateCcw, Sparkles, Workflow } from "lucide-react"
import { motion } from "motion/react"
import { invoke } from "@tauri-apps/api/core"
import { usePipeline } from "../hooks/usePipeline"
import { SpeakerTimeline } from "./SpeakerTimeline"
import { SpeakerTranscript } from "./SpeakerTranscript"
import { TextResultPanel } from "./TextResultPanel"
import { SummaryPanel } from "./SummaryPanel"
import { EditableTitle } from "./EditableTitle"
import { Button } from "../../../components/ui/Button"
import { Panel, PanelHeader, PanelBody } from "../../../components/ui/Panel"
import { SignalRail, type RailStep } from "../../../components/ui/SignalRail"
import { StatusDot } from "../../../components/ui/StatusDot"
import { EmptyState } from "../../../components/ui/EmptyState"
import { MeetingChat } from "../../chat"

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

export function PipelineOrchestrator({
  audioPath,
  meetingId,
  meetingTitle,
  meetingCategory,
  expectedSpeakers,
  categories = [],
  onSetCategory,
  onMeetingUpdated,
}: {
  audioPath?: string
  meetingId?: string
  meetingTitle?: string
  meetingCategory?: string | null
  expectedSpeakers?: number | null
  categories?: string[]
  onSetCategory?: (id: string, category: string | null) => void
  onMeetingUpdated?: () => void
}) {
  const {
    state,
    runPipeline,
    reset,
    loadExisting,
    loadingExisting,
    updateSummary,
    regenerateSummary,
    regenerating,
    retrySave,
  } = usePipeline(onMeetingUpdated)

  const [isCreatingCategory, setIsCreatingCategory] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState("")

  const saveTitle = (title: string) => {
    if (!meetingId) return
    invoke("update_meeting_title", { meetingId, title })
      .then(() => onMeetingUpdated?.())
      .catch(() => {})
  }

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
      {meetingId && meetingTitle && (
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1">
            <EditableTitle title={meetingTitle} onSave={saveTitle} />
          </div>
          {onSetCategory && (
            <div className="shrink-0 pt-1">
              {isCreatingCategory ? (
                <input
                  autoFocus
                  type="text"
                  placeholder="Nueva categoría…"
                  value={newCategoryName}
                  onChange={(e) => setNewCategoryName(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && newCategoryName.trim()) {
                      e.preventDefault()
                      onSetCategory(meetingId, newCategoryName.trim())
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
                      onSetCategory(meetingId, newCategoryName.trim())
                    }
                    setIsCreatingCategory(false)
                    setNewCategoryName("")
                  }}
                  className="w-36 rounded-control border border-signal/40 bg-canvas-raised px-2 py-1 font-mono text-[11px] text-ink placeholder:text-ink-mute focus:border-signal focus:outline-none"
                />
              ) : (
                <select
                  value={meetingCategory ?? ""}
                  onChange={(e) => {
                    const value = e.target.value
                    if (value === "__new__") {
                      setIsCreatingCategory(true)
                      return
                    }
                    onSetCategory(meetingId, value || null)
                  }}
                  className="rounded-control border border-hairline bg-canvas-raised px-3 py-1.5 font-mono text-[11px] text-ink-dim hover:border-signal/40 focus:border-signal focus:outline-none"
                >
                  <option value="">Sin categoría</option>
                  {categories.map((c) => (
                    <option key={c} value={c}>
                      {c}
                    </option>
                  ))}
                  <option value="__new__">+ Nueva categoría…</option>
                </select>
              )}
            </div>
          )}
        </div>
      )}

      {(state.transcriptSaveError || state.summarySaveError) && (
        <div className="flex items-start gap-2 rounded-control border border-rec/30 bg-rec-dim px-4 py-3 text-sm text-ink">
          <AlertCircle className="mt-0.5 size-4 shrink-0 text-rec" />
          <div className="flex-1 space-y-2">
            {state.transcriptSaveError && (
              <p>No se pudo guardar la transcripción: {state.transcriptSaveError}</p>
            )}
            {state.summarySaveError && (
              <p>No se pudo guardar el resumen: {state.summarySaveError}</p>
            )}
            {meetingId && (
              <Button size="sm" variant="subtle" onClick={() => retrySave(meetingId)}>
                Reintentar guardado
              </Button>
            )}
          </div>
        </div>
      )}

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

          {state.step === "idle" && !state.transcript && (
            <Button
              leftIcon={<Play className="size-4" />}
              onClick={() =>
                runPipeline(audioPath, meetingId, undefined, undefined, expectedSpeakers, meetingTitle)
              }
            >
              Ejecutar pipeline
            </Button>
          )}

          {/* La transcripción ya está guardada (un guardado previo se quedó a
              medias) — solo falta el resumen, así que no hace falta
              re-diarizar ni re-transcribir el audio. */}
          {state.step === "idle" && state.transcript && meetingId && (
            <Button
              leftIcon={<Sparkles className="size-4" />}
              disabled={regenerating}
              onClick={() => regenerateSummary(meetingId)}
            >
              {regenerating ? "Generando resumen…" : "Generar resumen"}
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

      {state.utterances && state.utterances.length > 0 && (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}>
          <Panel>
            <PanelHeader
              eyebrow="Transcripción"
              title={`Por hablante · ${new Set(state.utterances.map((u) => u.speaker)).size} hablante(s)`}
            />
            <PanelBody>
              <SpeakerTranscript utterances={state.utterances} />
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
              <SummaryPanel
                summary={state.summary}
                title={meetingTitle ?? "Resumen"}
                regenerating={regenerating}
                onSave={meetingId ? (text) => updateSummary(meetingId, text) : undefined}
                onRegenerate={
                  meetingId ? (instructions) => regenerateSummary(meetingId, instructions) : undefined
                }
              />
            </PanelBody>
          </Panel>
        </motion.div>
      )}

      {state.transcript && (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}>
          <MeetingChat meetingId={meetingId} />
        </motion.div>
      )}
    </div>
  )
}
