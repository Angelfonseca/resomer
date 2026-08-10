import { useEffect, useRef, useState } from "react"
import { AlertCircle, MessageCircle, Send, Sparkles } from "lucide-react"
import { motion, AnimatePresence } from "motion/react"
import { useMeetingChat } from "../hooks/useMeetingChat"
import { Panel, PanelHeader, PanelBody } from "../../../components/ui/Panel"
import { IconButton } from "../../../components/ui/IconButton"
import { Markdown } from "../../../components/ui/Markdown"
import { StatusDot } from "../../../components/ui/StatusDot"

const EXAMPLE_PROMPTS = [
  "¿Cuáles fueron las decisiones principales?",
  "¿Qué se acordó sobre el presupuesto?",
  "Resume los pendientes de cada persona",
]

export function MeetingChat({ meetingId }: { meetingId?: string }) {
  const { messages, loadingHistory, sending, error, ask } = useMeetingChat(meetingId)
  const [input, setInput] = useState("")
  const scrollRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: "smooth" })
  }, [messages, sending])

  const handleSend = () => {
    const question = input.trim()
    if (!question || sending) return
    setInput("")
    ask(question)
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault()
      handleSend()
    }
  }

  if (!meetingId) return null

  return (
    <Panel raised>
      <PanelHeader
        eyebrow="Preguntas"
        title="Habla con esta reunión"
        action={
          <div className="flex size-9 items-center justify-center rounded-md bg-signal-dim text-signal">
            <MessageCircle className="size-[18px]" strokeWidth={2} />
          </div>
        }
      />
      <PanelBody className="space-y-4">
        <div
          ref={scrollRef}
          className="max-h-[26rem] min-h-[8rem] space-y-3 overflow-y-auto rounded-control border border-hairline bg-canvas-raised p-4"
        >
          {loadingHistory ? (
            <div className="flex items-center gap-2 py-4">
              <StatusDot tone="signal" pulse />
              <span className="font-mono text-xs text-ink-dim">Cargando conversación…</span>
            </div>
          ) : messages.length === 0 ? (
            <div className="space-y-3 py-2">
              <p className="text-sm text-ink-dim">
                Pregunta lo que quieras sobre esta reunión — el asistente responde basándose
                en la transcripción completa.
              </p>
              <div className="flex flex-wrap gap-2">
                {EXAMPLE_PROMPTS.map((prompt) => (
                  <button
                    key={prompt}
                    onClick={() => ask(prompt)}
                    className="rounded-control border border-hairline-strong bg-panel-hi px-3 py-1.5 text-xs text-ink-dim transition-colors hover:border-signal/40 hover:text-ink"
                  >
                    {prompt}
                  </button>
                ))}
              </div>
            </div>
          ) : (
            <AnimatePresence initial={false}>
              {messages.map((m, i) => (
                <motion.div
                  key={`${m.created_at}-${i}`}
                  initial={{ opacity: 0, y: 6 }}
                  animate={{ opacity: 1, y: 0 }}
                  className={`flex ${m.role === "user" ? "justify-end" : "justify-start"}`}
                >
                  <div
                    className={
                      "max-w-[85%] rounded-control px-3.5 py-2.5 text-sm leading-relaxed " +
                      (m.role === "user"
                        ? "bg-signal-dim text-ink"
                        : "border border-hairline-strong bg-panel-hi text-ink-dim")
                    }
                  >
                    {m.role === "assistant" && (
                      <div className="mb-1 flex items-center gap-1.5 font-mono text-[10px] uppercase tracking-[0.1em] text-signal">
                        <Sparkles className="size-3" />
                        Asistente
                      </div>
                    )}
                    {m.role === "assistant" ? (
                      <Markdown>{m.content}</Markdown>
                    ) : (
                      <p className="whitespace-pre-wrap">{m.content}</p>
                    )}
                  </div>
                </motion.div>
              ))}
            </AnimatePresence>
          )}

          {sending && (
            <div className="flex items-center gap-2 pl-1">
              <StatusDot tone="signal" pulse />
              <span className="font-mono text-xs text-ink-dim">Pensando…</span>
            </div>
          )}
        </div>

        {error && (
          <div className="flex items-start gap-2 rounded-control border border-rec/30 bg-rec-dim px-4 py-3 text-sm text-ink">
            <AlertCircle className="mt-0.5 size-4 shrink-0 text-rec" />
            <span>{error}</span>
          </div>
        )}

        <div className="flex items-end gap-2">
          <textarea
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Escribe una pregunta sobre la reunión…"
            rows={1}
            className="max-h-32 flex-1 resize-none rounded-control border border-hairline-strong bg-canvas-raised px-3 py-2.5 text-sm text-ink placeholder:text-ink-mute transition-colors focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"
          />
          <IconButton
            aria-label="Enviar pregunta"
            icon={<Send className="size-4" />}
            variant="signal"
            disabled={!input.trim() || sending}
            onClick={handleSend}
          />
        </div>
      </PanelBody>
    </Panel>
  )
}
