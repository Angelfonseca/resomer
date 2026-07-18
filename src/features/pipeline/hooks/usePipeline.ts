import { useState, useCallback } from "react"
import { invoke } from "@tauri-apps/api/core"
import type { Segment } from "../types"

// Mirrors the default in src-tauri/src/infra/config.rs (ConfigManager).
// There is no frontend command yet to read the configured base URL, so we
// keep this in sync manually until that's wired up.
const API_BASE_URL = "https://api.nan.builders/v1"
const TRANSCRIPTION_ENDPOINT = `${API_BASE_URL}/audio/transcriptions`
const CHAT_ENDPOINT = `${API_BASE_URL}/chat/completions`

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
  transcript?: string
  summary?: string
  progress: number
  error?: string
}

interface MeetingData {
  segments: Segment[]
  transcript: string | null
  summary: string | null
}

export const usePipeline = () => {
  const [state, setState] = useState<PipelineState>({
    step: "idle",
    progress: 0,
  })
  const [loadingExisting, setLoadingExisting] = useState(false)

  const reset = useCallback(() => {
    setState({ step: "idle", progress: 0 })
  }, [])

  // Al abrir una reunión ya existente, carga lo que ya haya guardado en la
  // base de datos (segments/transcript/summary) en vez de re-ejecutar el
  // pipeline desde cero. Si no hay nada guardado (grabación sin procesar, o
  // que quedó en error), deja el estado en "idle" para que el usuario pueda
  // ejecutarlo/reintentarlo con el botón normal.
  const loadExisting = useCallback(async (meetingId: string, audioPath: string) => {
    setLoadingExisting(true)
    try {
      const data = await invoke<MeetingData>("get_meeting_data", { meetingId })
      if (data.transcript && data.summary) {
        setState({
          step: "complete",
          audioPath,
          segments: data.segments,
          transcript: data.transcript,
          summary: data.summary,
          progress: 100,
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
    async (audioPath: string, meetingId?: string, apiModel = "mimo-v2.5", transcriptionModel = "whisper") => {
      try {
        setState({ step: "diarizing", audioPath, progress: 15 })
        const segments = await invoke<Segment[]>("diarize_audio", { audioPath })

        setState((prev) => ({ ...prev, segments, progress: 40 }))

        setState((prev) => ({ ...prev, step: "transcribing", progress: 50 }))
        const transcript = await invoke<string>("transcribe_audio", {
          audioPath,
          apiEndpoint: TRANSCRIPTION_ENDPOINT,
          model: transcriptionModel,
        })

        setState((prev) => ({ ...prev, transcript, progress: 75 }))

        setState((prev) => ({ ...prev, step: "summarizing", progress: 85 }))
        const summary = await invoke<string>("summarize_text", {
          text: transcript,
          apiEndpoint: CHAT_ENDPOINT,
          model: apiModel,
        })

        // Save results to database if meetingId is provided
        if (meetingId) {
          await invoke("save_pipeline_results", {
            meetingId,
            segments,
            transcript,
            summary,
          })
        }

        setState({
          step: "complete",
          audioPath,
          segments,
          transcript,
          summary,
          progress: 100,
        })
      } catch (err) {
        const error = err instanceof Error ? err.message : String(err)
        setState((prev) => ({ ...prev, step: "error", error }))
        if (meetingId) {
          await invoke("mark_meeting_error", { meetingId, error }).catch(() => {})
        }
      }
    },
    []
  )

  return { state, runPipeline, reset, loadExisting, loadingExisting }
}
