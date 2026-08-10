export interface Segment {
  start: number
  end: number
  speaker: string
}

/** Un tramo de transcripción con marcas de tiempo, tal como lo devuelve Whisper. */
export interface TranscriptSegment {
  start: number
  end: number
  text: string
}

/** Texto plano + segmentos con tiempos que devuelve `transcribe_audio`. */
export interface TranscriptionOutput {
  text: string
  segments: TranscriptSegment[]
}

/** Una intervención ya atribuida a un hablante: "quién dijo qué". */
export interface SpeakerUtterance {
  speaker: string
  start: number
  end: number
  text: string
}
