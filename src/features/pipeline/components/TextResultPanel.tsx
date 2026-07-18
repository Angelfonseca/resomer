import { useState } from "react"
import { Check, Copy, Download } from "lucide-react"
import ReactMarkdown, { type Components } from "react-markdown"
import remarkGfm from "remark-gfm"
import { Button } from "../../../components/ui/Button"
import { downloadTextFile } from "../../../lib/download"

const markdownComponents: Components = {
  h1: ({ children }) => (
    <h1 className="mb-3 font-display text-lg font-semibold text-ink first:mt-0">{children}</h1>
  ),
  h2: ({ children }) => (
    <h2 className="mb-2 mt-5 border-l-2 border-signal/50 pl-2.5 font-display text-base font-semibold text-ink first:mt-0">
      {children}
    </h2>
  ),
  h3: ({ children }) => (
    <h3 className="mb-1.5 mt-4 font-mono text-[11px] font-semibold uppercase tracking-[0.08em] text-signal first:mt-0">
      {children}
    </h3>
  ),
  p: ({ children }) => (
    <p className="mb-3 text-sm leading-relaxed text-ink-dim last:mb-0">{children}</p>
  ),
  ul: ({ children }) => (
    <ul className="mb-3 list-disc space-y-1.5 pl-5 marker:text-signal">{children}</ul>
  ),
  ol: ({ children }) => (
    <ol className="mb-3 list-decimal space-y-1.5 pl-5 marker:font-mono marker:text-signal">
      {children}
    </ol>
  ),
  li: ({ children }) => <li className="text-sm leading-relaxed text-ink-dim">{children}</li>,
  strong: ({ children }) => <strong className="font-semibold text-ink">{children}</strong>,
  em: ({ children }) => <em className="italic text-ink-dim">{children}</em>,
  code: ({ children }) => (
    <code className="rounded bg-panel-hi px-1.5 py-0.5 font-mono text-[12px] text-signal">
      {children}
    </code>
  ),
  pre: ({ children }) => (
    <pre className="mb-3 overflow-x-auto rounded-control border border-hairline bg-canvas p-3 font-mono text-[12px] text-ink-dim">
      {children}
    </pre>
  ),
  blockquote: ({ children }) => (
    <blockquote className="my-3 border-l-2 border-signal/40 pl-3 italic text-ink-dim">
      {children}
    </blockquote>
  ),
  hr: () => <hr className="my-4 border-hairline" />,
  a: ({ children, href }) => (
    <a
      href={href}
      target="_blank"
      rel="noreferrer"
      className="text-signal underline underline-offset-2 hover:text-signal-hi"
    >
      {children}
    </a>
  ),
  table: ({ children }) => (
    <div className="mb-3 overflow-x-auto">
      <table className="w-full border-collapse text-sm">{children}</table>
    </div>
  ),
  th: ({ children }) => (
    <th className="border-b border-hairline-strong px-2 py-1.5 text-left font-mono text-[11px] uppercase tracking-wide text-ink-mute">
      {children}
    </th>
  ),
  td: ({ children }) => (
    <td className="border-b border-hairline px-2 py-1.5 text-ink-dim">{children}</td>
  ),
  input: ({ checked, disabled }) => (
    <input
      type="checkbox"
      checked={checked ?? false}
      disabled={disabled}
      readOnly
      className="mr-1.5 accent-signal"
    />
  ),
}

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
          <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
            {text}
          </ReactMarkdown>
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
