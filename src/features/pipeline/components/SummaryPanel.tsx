import { useState } from "react"
import { Check, FileDown, Pencil, RefreshCw, X } from "lucide-react"
import { Button } from "../../../components/ui/Button"
import { Markdown } from "../../../components/ui/Markdown"
import { StatusDot } from "../../../components/ui/StatusDot"
import { downloadPdf } from "../../../lib/pdf"

export function SummaryPanel({
  summary,
  title,
  regenerating,
  onSave,
  onRegenerate,
}: {
  summary: string
  title: string
  regenerating?: boolean
  onSave?: (text: string) => Promise<void> | void
  onRegenerate?: (instructions?: string) => void
}) {
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState(summary)
  const [saving, setSaving] = useState(false)
  const [promptingRegenerate, setPromptingRegenerate] = useState(false)
  const [instructions, setInstructions] = useState("")

  const startEdit = () => {
    setDraft(summary)
    setEditing(true)
  }

  const save = async () => {
    setSaving(true)
    try {
      await onSave?.(draft)
      setEditing(false)
    } finally {
      setSaving(false)
    }
  }

  const confirmRegenerate = () => {
    onRegenerate?.(instructions)
    setPromptingRegenerate(false)
    setInstructions("")
  }

  return (
    <div className="space-y-3">
      {editing ? (
        <textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          className="min-h-[24rem] w-full resize-y rounded-control border border-hairline-strong bg-canvas-raised p-5 font-mono text-[13px] leading-relaxed text-ink focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"
        />
      ) : (
        <div className="max-h-[32rem] overflow-y-auto rounded-control border border-hairline bg-canvas-raised p-5">
          <Markdown>{summary}</Markdown>
        </div>
      )}

      {promptingRegenerate && (
        <div className="space-y-2 rounded-control border border-signal/30 bg-signal-dim/40 p-3">
          <label className="block font-mono text-[10px] uppercase tracking-[0.1em] text-ink-mute">
            ¿Qué esperas de este resumen? (opcional)
          </label>
          <textarea
            autoFocus
            value={instructions}
            onChange={(e) => setInstructions(e.target.value)}
            placeholder="Ej: enfócate en las decisiones técnicas, hazlo más breve, no menciones nombres…"
            rows={2}
            className="w-full resize-none rounded-control border border-hairline-strong bg-canvas-raised px-3 py-2 text-sm text-ink placeholder:text-ink-mute focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"
          />
          <div className="flex gap-2">
            <Button size="sm" leftIcon={<RefreshCw className="size-3.5" />} onClick={confirmRegenerate}>
              Regenerar
            </Button>
            <Button
              variant="subtle"
              size="sm"
              onClick={() => {
                setPromptingRegenerate(false)
                setInstructions("")
              }}
            >
              Cancelar
            </Button>
          </div>
        </div>
      )}

      <div className="flex flex-wrap items-center gap-2">
        {editing ? (
          <>
            <Button
              size="sm"
              leftIcon={<Check className="size-3.5" />}
              disabled={saving}
              onClick={save}
            >
              {saving ? "Guardando…" : "Guardar"}
            </Button>
            <Button
              variant="subtle"
              size="sm"
              leftIcon={<X className="size-3.5" />}
              onClick={() => setEditing(false)}
            >
              Cancelar
            </Button>
          </>
        ) : (
          <>
            <Button
              variant="subtle"
              size="sm"
              leftIcon={<Pencil className="size-3.5" />}
              onClick={startEdit}
            >
              Editar
            </Button>
            {onRegenerate && !promptingRegenerate && (
              <Button
                variant="subtle"
                size="sm"
                leftIcon={<RefreshCw className={"size-3.5" + (regenerating ? " animate-spin" : "")} />}
                disabled={regenerating}
                onClick={() => setPromptingRegenerate(true)}
              >
                {regenerating ? "Regenerando…" : "Regenerar"}
              </Button>
            )}
            <Button
              variant="subtle"
              size="sm"
              leftIcon={<FileDown className="size-3.5" />}
              onClick={() => downloadPdf(`${title || "resumen"}.pdf`, title || "Resumen", summary)}
            >
              PDF
            </Button>
          </>
        )}
        {regenerating && !editing && (
          <div className="flex items-center gap-2 pl-1">
            <StatusDot tone="signal" pulse />
            <span className="font-mono text-xs text-ink-dim">Regenerando resumen…</span>
          </div>
        )}
      </div>
    </div>
  )
}
