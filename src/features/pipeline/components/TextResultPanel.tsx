import { useState } from "react"
import { Check, Copy, Download } from "lucide-react"
import { Button } from "../../../components/ui/Button"
import { Markdown } from "../../../components/ui/Markdown"
import { downloadTextFile } from "../../../lib/download"

export function TextResultPanel({
  text,
  filename,
  markdown = false,
}: {
  text: string
  filename: string
  markdown?: boolean
}) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    await navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 1800)
  }

  return (
    <div className="space-y-3">
      <div className="max-h-[32rem] overflow-y-auto rounded-control border border-hairline bg-canvas-raised p-5">
        {markdown ? (
          <Markdown>{text}</Markdown>
        ) : (
          <p className="whitespace-pre-wrap font-mono text-[13px] leading-relaxed text-ink-dim">
            {text}
          </p>
        )}
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
          onClick={() => downloadTextFile(filename, text)}
        >
          Exportar
        </Button>
      </div>
    </div>
  )
}
