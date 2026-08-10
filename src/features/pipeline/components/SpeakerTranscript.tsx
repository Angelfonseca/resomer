import { useMemo, useState } from "react"
import { Check, Copy, Download } from "lucide-react"
import { Button } from "../../../components/ui/Button"
import { formatTimecode } from "../../../components/ui/Timecode"
import { downloadTextFile } from "../../../lib/download"
import type { SpeakerUtterance } from "../types"

// Mismo orden/paleta que SpeakerTimeline, para que un mismo hablante tenga el
// mismo color en la línea de tiempo y en la transcripción atribuida.
const CHANNEL_COLORS = [
  "var(--color-channel-1)",
  "var(--color-channel-2)",
  "var(--color-channel-3)",
  "var(--color-channel-4)",
]

/** "Speaker-0" → "Hablante 1"; cualquier otra etiqueta se muestra tal cual. */
function speakerLabel(speaker: string): string {
  const m = speaker.match(/^Speaker-(\d+)$/)
  return m ? `Hablante ${Number(m[1]) + 1}` : speaker
}

export function SpeakerTranscript({ utterances }: { utterances: SpeakerUtterance[] }) {
  const [copied, setCopied] = useState(false)

  const colorFor = useMemo(() => {
    const speakers = [...new Set(utterances.map((u) => u.speaker))].sort()
    const map = new Map<string, string>()
    speakers.forEach((s, i) => map.set(s, CHANNEL_COLORS[i % CHANNEL_COLORS.length]))
    return (speaker: string) => map.get(speaker) ?? CHANNEL_COLORS[0]
  }, [utterances])

  // Texto plano para copiar/exportar: "Hablante 1 [00:12]: …"
  const asText = useMemo(
    () =>
      utterances
        .map((u) => `${speakerLabel(u.speaker)} [${formatTimecode(u.start)}]: ${u.text}`)
        .join("\n\n"),
    [utterances]
  )

  const handleCopy = async () => {
    await navigator.clipboard.writeText(asText)
    setCopied(true)
    setTimeout(() => setCopied(false), 1800)
  }

  return (
    <div className="space-y-3">
      <div className="max-h-[32rem] space-y-4 overflow-y-auto rounded-control border border-hairline bg-canvas-raised p-5">
        {utterances.map((u, i) => {
          const color = colorFor(u.speaker)
          return (
            <div key={i} className="flex gap-3">
              {/* Franja de color del hablante */}
              <span
                className="mt-1 w-0.5 shrink-0 self-stretch rounded-full"
                style={{ backgroundColor: color }}
              />
              <div className="min-w-0 flex-1 space-y-1">
                <div className="flex items-baseline gap-2">
                  <span
                    className="text-xs font-semibold"
                    style={{ color }}
                  >
                    {speakerLabel(u.speaker)}
                  </span>
                  <span className="font-mono text-[10px] tabular-nums text-ink-mute">
                    {formatTimecode(u.start)}
                  </span>
                </div>
                <p className="whitespace-pre-wrap text-[13px] leading-relaxed text-ink-dim">
                  {u.text}
                </p>
              </div>
            </div>
          )
        })}
      </div>
      <div className="flex gap-2">
        <Button
          variant="subtle"
          size="sm"
          leftIcon={copied ? <Check className="size-3.5" /> : <Copy className="size-3.5" />}
          onClick={handleCopy}
        >
          {copied ? "Copiado" : "Copiar"}
        </Button>
        <Button
          variant="subtle"
          size="sm"
          leftIcon={<Download className="size-3.5" />}
          onClick={() => downloadTextFile("transcripcion-por-hablante.txt", asText)}
        >
          Exportar
        </Button>
      </div>
    </div>
  )
}
