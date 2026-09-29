// Paleta de hablantes compartida por la línea de tiempo y la transcripción
// atribuida, para que un mismo hablante tenga el mismo color en ambas vistas.
export const CHANNEL_COLORS = [
  "var(--color-channel-1)",
  "var(--color-channel-2)",
  "var(--color-channel-3)",
  "var(--color-channel-4)",
]

/** "Speaker-0" → "Hablante 1"; cualquier otra etiqueta se muestra tal cual. */
export function speakerLabel(speaker: string): string {
  const m = speaker.match(/^Speaker-(\d+)$/)
  return m ? `Hablante ${Number(m[1]) + 1}` : speaker
}
