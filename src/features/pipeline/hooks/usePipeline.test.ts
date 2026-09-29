import { describe, it, expect, vi, beforeEach } from "vitest"
import { renderHook, act } from "@testing-library/react"
import { usePipeline } from "./usePipeline"

const invokeMock = vi.fn()

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}))

vi.mock("@tauri-apps/plugin-notification", () => ({
  isPermissionGranted: vi.fn().mockResolvedValue(true),
  requestPermission: vi.fn().mockResolvedValue("granted"),
  sendNotification: vi.fn(),
}))

const DIARIZATION = [{ start: 0, end: 1, speaker: "Speaker-0" }]
const TRANSCRIPTION = { text: "hola mundo", segments: [{ start: 0, end: 1, text: "hola mundo" }] }
const UTTERANCES = [{ speaker: "Speaker-0", start: 0, end: 1, text: "hola mundo" }]

function mockHappyPath() {
  invokeMock.mockImplementation((cmd: string) => {
    switch (cmd) {
      case "diarize_audio":
        return Promise.resolve(DIARIZATION)
      case "transcribe_audio":
        return Promise.resolve(TRANSCRIPTION)
      case "attribute_speakers":
        return Promise.resolve(UTTERANCES)
      case "summarize_text":
        return Promise.resolve("- resumen")
      case "generate_meeting_title":
        return Promise.resolve("Titulo")
      default:
        return Promise.resolve()
    }
  })
}

describe("usePipeline", () => {
  beforeEach(() => {
    invokeMock.mockReset()
  })

  it("recorre el pipeline y persiste transcripción y resumen", async () => {
    mockHappyPath()
    const { result } = renderHook(() => usePipeline())

    await act(async () => {
      await result.current.runPipeline("audio.wav", "m1")
    })

    expect(result.current.state.step).toBe("complete")
    expect(result.current.state.transcript).toBe("hola mundo")
    expect(result.current.state.summary).toBe("- resumen")
    expect(result.current.state.transcriptSaveError).toBeUndefined()
    expect(result.current.state.summarySaveError).toBeUndefined()
  })

  it("mantiene el error de transcripción aunque el resumen se guarde", async () => {
    mockHappyPath()
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "save_transcript_results") return Promise.reject("db locked")
      if (cmd === "transcribe_audio") return Promise.resolve(TRANSCRIPTION)
      if (cmd === "attribute_speakers") return Promise.resolve(UTTERANCES)
      if (cmd === "summarize_text") return Promise.resolve("- resumen")
      return Promise.resolve(DIARIZATION)
    })

    const { result } = renderHook(() => usePipeline())
    await act(async () => {
      await result.current.runPipeline("audio.wav", "m1")
    })

    // El guardado de la transcripción falló y no debe quedar oculto por el
    // éxito del resumen.
    expect(result.current.state.transcriptSaveError).toBe("db locked")
    expect(result.current.state.summarySaveError).toBeUndefined()
    expect(result.current.state.step).toBe("complete")
  })

  it("marca error cuando ningún paso puede transcribir", async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === "diarize_audio") return Promise.resolve(DIARIZATION)
      if (cmd === "transcribe_audio") return Promise.reject(new Error("gateway caído"))
      return Promise.resolve()
    })

    const { result } = renderHook(() => usePipeline())
    await act(async () => {
      await result.current.runPipeline("audio.wav", "m1")
    })

    expect(result.current.state.step).toBe("error")
    expect(result.current.state.error).toContain("gateway caído")
  })
})
