import { useState, useCallback, useRef } from "react"
import { invoke } from "@tauri-apps/api/core"
import { notifyPipelineResult } from "../../../lib/notify"
import type { Segment, SpeakerUtterance, TranscriptionOutput } from "../types"

// Los endpoints del gateway los resuelve el backend desde su config; el
// frontend ya no los envía (antes los hardcodeaba aquí, permitiendo filtrar la
// API key a un host arbitrario desde una webview comprometida).

export type PipelineStep =
  | "idle"
  | "diarizing"
  | "transcribing"
  | "summarizing"
  | "complete"
  | "error"

export interface PipelineState {
  step: PipelineStep
  audioPath?: string
  segments?: Segment[]
  utterances?: SpeakerUtterance[]
  transcript?: string
  summary?: string
  progress: number
  error?: string
  // Un guardado falló pero el resultado ya computado sigue disponible en
  // memoria — distinto de `error`, que es un fallo del propio procesamiento
  // (diarización/transcripción/resumen). Se llevan por separado: antes un
  // guardado de resumen exitoso borraba el error de un guardado de
  // transcripción fallido, y la transcripción se perdía en silencio.
  transcriptSaveError?: string
  summarySaveError?: string
}

interface MeetingData {
  segments: Segment[]
  utterances: SpeakerUtterance[]
  transcript: string | null
  summary: string | null
}

export const usePipeline = (onMeetingUpdated?: () => void) => {
  const [state, setState] = useState<PipelineState>({
    step: "idle",
    progress: 0,
  })
  const [loadingExisting, setLoadingExisting] = useState(false)
  const [regenerating, setRegenerating] = useState(false)
  // Espejo del estado para leer resultados actuales desde callbacks estables
  // (sin meter `state` en sus deps y arriesgar cierres obsoletos).
  const stateRef = useRef(state)
  stateRef.current = state

  const reset = useCallback(() => {
    setState({ step: "idle", progress: 0 })
  }, [])

  // Al abrir una reunión ya existente, carga lo que ya haya guardado en la
  // base de datos (segments/transcript/summary) en vez de re-ejecutar el
  // pipeline desde cero. Si ya hay transcripción pero falta el resumen (un
  // guardado previo se quedó a medias), deja lo ya calculado en memoria para
  // que solo haga falta generar el resumen, no re-diarizar ni re-transcribir.
  const loadExisting = useCallback(async (meetingId: string, audioPath: string) => {
    setLoadingExisting(true)
    try {
      const data = await invoke<MeetingData>("get_meeting_data", { meetingId })
      if (data.transcript && data.summary) {
        setState({
          step: "complete",
          audioPath,
          segments: data.segments,
          utterances: data.utterances,
          transcript: data.transcript,
          summary: data.summary,
          progress: 100,
        })
      } else if (data.transcript) {
        setState({
          step: "idle",
          audioPath,
          segments: data.segments,
          utterances: data.utterances,
          transcript: data.transcript,
          progress: 0,
        })
      } else {
        setState({ step: "idle", audioPath, progress: 0 })
      }
    } catch {
      setState({ step: "idle", audioPath, progress: 0 })
    } finally {
      setLoadingExisting(false)
    }
  }, [])

  const runPipeline = useCallback(
    async (
      audioPath: string,
      meetingId?: string,
      apiModel = "mimo-v2.6-flash",
      transcriptionModel = "whisper",
      numSpeakers?: number | null,
      meetingLabel = "Reunión"
    ) => {
      try {
        setState({ step: "diarizing", audioPath, progress: 15 })
        const segments = await invoke<Segment[]>("diarize_audio", {
          audioPath,
          numSpeakers: numSpeakers ?? undefined,
        })

        setState((prev) => ({ ...prev, segments, progress: 40 }))

        setState((prev) => ({ ...prev, step: "transcribing", progress: 50 }))
        const transcription = await invoke<TranscriptionOutput>("transcribe_audio", {
          audioPath,
          model: transcriptionModel,
        })
        const transcript = transcription.text

        // Cruza diarización + transcripción con marcas de tiempo → quién dijo qué.
        const utterances = await invoke<SpeakerUtterance[]>("attribute_speakers", {
          diarization: segments,
          transcript: transcription.segments,
        })

        setState((prev) => ({ ...prev, transcript, utterances, progress: 75 }))

        // Persistir la transcripción YA, antes de intentar el resumen: es la
        // parte lenta/costosa (transcribir audio largo vía Whisper), y si el
        // paso de resumen falla más adelante (red, gateway caído) no debe
        // perderse — solo habría que regenerar el resumen, no todo el pipeline.
        if (meetingId) {
          try {
            await invoke("save_transcript_results", { meetingId, segments, utterances, transcript })
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err)
            setState((prev) => ({ ...prev, transcriptSaveError: message }))
          }
        }

        setState((prev) => ({ ...prev, step: "summarizing", progress: 85 }))
        const summary = await invoke<string>("summarize_text", {
          text: transcript,
          model: apiModel,
        })

        setState((prev) => ({ ...prev, summary }))

        if (meetingId) {
          try {
            await invoke("update_summary", { meetingId, summary })
            setState((prev) => ({ ...prev, summarySaveError: undefined }))
            // La IA titula la reunión a partir del resumen. No bloqueante: si
            // falla, la reunión conserva su título "Reunión N".
            try {
              await invoke("generate_meeting_title", {
                meetingId,
                model: apiModel,
              })
            } catch {
              /* título opcional */
            }
            onMeetingUpdated?.()
          } catch (err) {
            const message = err instanceof Error ? err.message : String(err)
            setState((prev) => ({ ...prev, summarySaveError: message }))
          }
        }

        setState((prev) => ({
          ...prev,
          step: "complete",
          audioPath,
          segments,
          utterances,
          transcript,
          summary,
          progress: 100,
        }))
        notifyPipelineResult("Pipeline completado", `${meetingLabel} — resumen listo`)
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err)
        setState((prev) => ({ ...prev, step: "error", error }))
        if (meetingId) {
          await invoke("mark_meeting_error", { meetingId, error }).catch(() => {})
        }
        notifyPipelineResult("Error en el pipeline", `${meetingLabel} — ${error}`)
      }
    },
    [onMeetingUpdated]
  )

  // Reintenta persistir lo que ya está calculado en memoria (transcripción
  // y/o resumen), sin recomputar nada. Cubre el caso en que un guardado falló
  // por un problema transitorio (red, DB ocupada) pero el resultado del
  // procesamiento sigue disponible en este mismo estado.
  const retrySave = useCallback(
    async (meetingId: string) => {
      const { segments, utterances, transcript, summary } = stateRef.current
      const message = (err: unknown) => (err instanceof Error ? err.message : String(err))

      // Reintenta cada guardado por separado para que el éxito de uno no
      // oculte el fallo pendiente del otro.
      if (transcript) {
        try {
          await invoke("save_transcript_results", {
            meetingId,
            segments: segments ?? [],
            utterances: utterances ?? [],
            transcript,
          })
          setState((prev) => ({ ...prev, transcriptSaveError: undefined }))
        } catch (err) {
          setState((prev) => ({ ...prev, transcriptSaveError: message(err) }))
        }
      }
      if (summary) {
        try {
          await invoke("update_summary", { meetingId, summary })
          setState((prev) => ({ ...prev, summarySaveError: undefined }))
        } catch (err) {
          setState((prev) => ({ ...prev, summarySaveError: message(err) }))
        }
      }
      onMeetingUpdated?.()
    },
    [onMeetingUpdated]
  )

  // Guarda una edición manual del resumen y refleja el cambio en memoria.
  const updateSummary = useCallback(async (meetingId: string, summary: string) => {
    await invoke("update_summary", { meetingId, summary })
    setState((prev) => ({ ...prev, summary }))
  }, [])

  // Genera (o regenera) el resumen desde la transcripción actual, lo persiste
  // y re-titula la reunión. Requiere que ya exista transcripción en el estado
  // — cubre tanto "regenerar" como "generar el resumen que quedó pendiente".
  // `instructions` es lo que el usuario espera de este resumen en concreto
  // (p. ej. "enfócate en las decisiones técnicas", "más breve") y se manda tal
  // cual al prompt del backend.
  const regenerateSummary = useCallback(
    async (meetingId: string, instructions?: string, model = "mimo-v2.6-flash") => {
      const transcript = stateRef.current.transcript
      if (!transcript) return
      setRegenerating(true)
      try {
        const summary = await invoke<string>("summarize_text", {
          text: transcript,
          model,
          instructions: instructions?.trim() || undefined,
        })
        await invoke("update_summary", { meetingId, summary })
        setState((prev) => ({
          ...prev,
          summary,
          step: "complete",
          progress: 100,
          summarySaveError: undefined,
        }))
        try {
          await invoke("generate_meeting_title", {
            meetingId,
            model,
          })
          onMeetingUpdated?.()
        } catch {
          /* título opcional */
        }
      } catch (err) {
        // Sin este catch, un fallo al regenerar quedaba como promesa
        // rechazada sin manejar y el usuario no veía nada.
        const msg = err instanceof Error ? err.message : String(err)
        setState((prev) => ({ ...prev, summarySaveError: msg }))
      } finally {
        setRegenerating(false)
      }
    },
    [onMeetingUpdated]
  )

  return {
    state,
    runPipeline,
    reset,
    loadExisting,
    loadingExisting,
    updateSummary,
    regenerateSummary,
    regenerating,
    retrySave,
  }
}
