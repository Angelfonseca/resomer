import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { Mic, Square, AlertCircle } from "lucide-react"
import { motion, AnimatePresence } from "motion/react"
import { useRecording, useAudioDevices, type RecordingSource } from "../hooks/useRecording"
import { useAudioMeter } from "../hooks/useAudioMeter"
import { useSyntheticMeter } from "../hooks/useSyntheticMeter"
import { useSystemAudioMeter } from "../hooks/useSystemAudioMeter"
import { Panel, PanelHeader, PanelBody } from "../../../components/ui/Panel"
import { IconButton } from "../../../components/ui/IconButton"
import { SelectField } from "../../../components/ui/Field"
import { StatusDot } from "../../../components/ui/StatusDot"
import { Timecode } from "../../../components/ui/Timecode"
import { Waveform } from "../../../components/ui/Waveform"

interface RecordingPanelProps {
  meetingId: string
  onRecordingComplete?: (filePath: string, expectedSpeakers: number | null) => void
  // Fuente preseleccionada (p. ej. cuando la grabación se lanza desde el
  // menú del ícono de la barra de estado con mic/sistema/ambos ya elegido).
  initialSource?: RecordingSource
  // Grabación ya iniciada por el backend (desde el ícono de la barra de
  // estado). Si viene, el panel la refleja en curso en vez de arrancar una.
  attach?: { meetingId: string; filePath: string; source: RecordingSource } | null
  categories?: string[]
  onSetCategory?: (id: string, category: string | null) => void
  initialCategory?: string | null
}

// "Desconocido" cae al modo auto-detect del motor de diarización; el resto
// fija num_clusters exacto, que es la forma más fiable de evitar que
// aparezcan hablantes fantasma.
const SPEAKER_COUNT_OPTIONS = [1, 2, 3, 4, 5, 6, 7, 8]

const SOURCE_LABEL: Record<RecordingSource, string> = {
  microphone: "Solo micrófono",
  system_audio: "Solo audio del sistema",
  both: "Ambos (estéreo)",
}

export function RecordingPanel({
  meetingId,
  onRecordingComplete,
  initialSource = "microphone",
  attach = null,
  categories = [],
  onSetCategory,
  initialCategory = null,
}: RecordingPanelProps) {
  const { recording, isLoading, error, startRecording, stopRecording, attachRecording } =
    useRecording()
  const { devices, isLoading: devicesLoading } = useAudioDevices()
  const [selectedSource, setSelectedSource] = useState<RecordingSource>(initialSource)
  const [durationSeconds, setDurationSeconds] = useState(0)
  const [expectedSpeakers, setExpectedSpeakers] = useState<number | null>(null)
  const [selectedCategory, setSelectedCategory] = useState<string | null>(initialCategory)
  const [isCreatingCategory, setIsCreatingCategory] = useState(false)
  const [newCategoryName, setNewCategoryName] = useState("")

  // Sincronizar initialCategory con el estado si cambia desde fuera
  // (por ejemplo, cuando se crea la reunión y el backend devuelve la info real)
  useEffect(() => {
    if (initialCategory !== undefined) {
      setSelectedCategory(initialCategory)
    }
  }, [initialCategory])

  const isRecording = recording?.state === "recording"

  // Para "solo audio del sistema" y "ambos", el medidor usa el nivel REAL
  // capturado por el helper (llega vía evento Tauri), no un proxy del mic.
  // Para "solo micrófono" seguimos con el medidor local (getUserMedia).
  const isSystemSource = selectedSource !== "microphone"

  const liveMeter = useAudioMeter(isRecording && !isSystemSource)
  const syntheticLevels = useSyntheticMeter(isRecording && !isSystemSource && liveMeter.errored)
  const { systemLevels, micLevels } = useSystemAudioMeter(isRecording && isSystemSource)

  const levels = liveMeter.errored ? syntheticLevels : liveMeter.levels

  useEffect(() => {
    if (!isRecording) return
    const interval = setInterval(() => setDurationSeconds((prev) => prev + 1), 1000)
    return () => clearInterval(interval)
  }, [isRecording])

  const handleStart = async () => {
    setDurationSeconds(0)
    const timestamp = new Date().toISOString().replace(/[:.]/g, "-")
    const outputPath = `recordings/${meetingId}-${timestamp}.wav`
    // No bloqueante: si el meetingId es local (sin fila en BD todavía), esto
    // falla en silencio — el valor sigue viajando en memoria hacia el pipeline.
    invoke("set_expected_speakers", { meetingId, expectedSpeakers }).catch(() => {})
    await startRecording(meetingId, outputPath, selectedSource)
  }

  const handleStop = async () => {
    const filePath = recording?.filePath
    await stopRecording()
    if (filePath) onRecordingComplete?.(filePath, expectedSpeakers)
  }

  // Si la grabación fue iniciada desde el ícono de la barra de estado, el
  // grabador ya está corriendo en el backend: adoptamos su estado (cronómetro
  // + botón de detener) sin arrancar otro. El indicador "REC" del tray lo
  // gestiona el backend, no este componente.
  useEffect(() => {
    if (attach) attachRecording(attach)
    // Reflejar una sola vez por grabación (identificada por su meetingId).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [attach?.meetingId])

  const defaultDevice = devices.find((d) => d.isDefault) ?? devices[0]

  return (
    <Panel raised>
      <PanelHeader
        eyebrow="Grabación"
        title="Nueva reunión"
        action={
          <div className="flex items-center gap-2">
            <StatusDot tone={isRecording ? "rec" : "idle"} pulse={isRecording} />
            <span className="font-mono text-[11px] uppercase tracking-[0.1em] text-ink-dim">
              {isRecording ? "Grabando" : "En espera"}
            </span>
          </div>
        }
      />

      <PanelBody className="space-y-6">
        {error && (
          <div className="flex items-start gap-2 rounded-control border border-rec/30 bg-rec-dim px-4 py-3 text-sm text-ink">
            <AlertCircle className="mt-0.5 size-4 shrink-0 text-rec" />
            <span>{error}</span>
          </div>
        )}

        {/* Console screen: waveform + timecode */}
        <div className="rounded-control border border-hairline-strong bg-canvas-raised px-6 py-8">
          <div className="mb-2 h-20">
            {isSystemSource ? (
              <Waveform systemLevels={systemLevels} micLevels={micLevels} active={isRecording} />
            ) : (
              <Waveform levels={levels} active={isRecording} />
            )}
          </div>
          {isRecording && (
            <p className="mb-4 flex items-center justify-center gap-3 text-center font-mono text-[10px] uppercase tracking-[0.1em] text-ink-mute">
              {isSystemSource ? (
                <>
                  <span className="flex items-center gap-1.5">
                    <span
                      className="size-1.5 rounded-full"
                      style={{ backgroundColor: "var(--color-channel-2)" }}
                    />
                    Sistema
                  </span>
                  <span className="flex items-center gap-1.5">
                    <span className="size-1.5 rounded-full bg-signal" />
                    Micrófono
                  </span>
                </>
              ) : (
                "Vista previa del micrófono"
              )}
            </p>
          )}
          <div className="flex items-center justify-center">
            <Timecode seconds={durationSeconds} size="xl" />
          </div>
        </div>

        {/* Input source */}
        <div className="flex items-center justify-between rounded-control border border-hairline bg-panel-hi px-4 py-3">
          <div className="flex items-center gap-3">
            <div className="flex size-8 items-center justify-center rounded-md bg-signal-dim text-signal">
              <Mic className="size-4" />
            </div>
            <div>
              <p className="text-sm font-medium text-ink">
                {devicesLoading ? "Detectando dispositivo…" : defaultDevice?.name ?? "Sin dispositivo"}
              </p>
              <p className="font-mono text-[11px] text-ink-mute">
                {defaultDevice
                  ? `${defaultDevice.sampleRates?.[0] ?? "—"} Hz · ${defaultDevice.channels} ch`
                  : "—"}
              </p>
            </div>
          </div>
        </div>

        <SelectField
          label="Modo de grabación"
          value={selectedSource}
          disabled={isRecording}
          onChange={(e) => setSelectedSource(e.target.value as RecordingSource)}
        >
          {(Object.keys(SOURCE_LABEL) as RecordingSource[]).map((source) => (
            <option key={source} value={source}>
              {SOURCE_LABEL[source]}
            </option>
          ))}
        </SelectField>

        <SelectField
          label="Participantes"
          hint="Indicarlo mejora mucho la fiabilidad de la diarización."
          value={expectedSpeakers ?? ""}
          disabled={isRecording}
          onChange={(e) =>
            setExpectedSpeakers(e.target.value === "" ? null : Number(e.target.value))
          }
        >
          <option value="">No lo sé (automático)</option>
          {SPEAKER_COUNT_OPTIONS.map((n) => (
            <option key={n} value={n}>
              {n} {n === 1 ? "participante" : "participantes"}
            </option>
          ))}
        </SelectField>

        {isCreatingCategory ? (
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-ink">Nueva Categoría</label>
            <div className="flex items-center gap-2">
              <input
                autoFocus
                type="text"
                placeholder="Escribe y presiona Enter…"
                value={newCategoryName}
                onChange={(e) => setNewCategoryName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newCategoryName.trim()) {
                    e.preventDefault()
                    const name = newCategoryName.trim()
                    setSelectedCategory(name)
                    onSetCategory?.(meetingId, name)
                    setIsCreatingCategory(false)
                    setNewCategoryName("")
                  }
                  if (e.key === "Escape") {
                    setIsCreatingCategory(false)
                    setNewCategoryName("")
                  }
                }}
                onBlur={() => {
                  if (newCategoryName.trim()) {
                    const name = newCategoryName.trim()
                    setSelectedCategory(name)
                    onSetCategory?.(meetingId, name)
                  }
                  setIsCreatingCategory(false)
                  setNewCategoryName("")
                }}
                className="flex-1 rounded-control border border-signal/40 bg-canvas-raised px-3 py-2 text-sm text-ink placeholder:text-ink-mute focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"
              />
            </div>
            <p className="text-[11px] text-ink-mute">Presiona Enter para confirmar o Esc para cancelar.</p>
          </div>
        ) : (
          <SelectField
            label="Categoría"
            hint="Asigna una categoría para organizar tus reuniones."
            value={selectedCategory ?? ""}
            onChange={(e) => {
              const value = e.target.value
              if (value === "__new__") {
                setIsCreatingCategory(true)
                return
              }
              setSelectedCategory(value || null)
              onSetCategory?.(meetingId, value || null)
            }}
          >
            <option value="">Sin categoría</option>
            {categories.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
            <option value="__new__">+ Nueva categoría…</option>
          </SelectField>
        )}

        {/* Transport */}
        <div className="flex items-center justify-center pt-2">
          <AnimatePresence mode="wait">
            {!isRecording ? (
              <motion.div
                key="record"
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.9 }}
              >
                <IconButton
                  aria-label="Iniciar grabación"
                  icon={<Mic />}
                  variant="signal"
                  size="xl"
                  disabled={isLoading}
                  onClick={handleStart}
                />
              </motion.div>
            ) : (
              <motion.div
                key="stop"
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.9 }}
              >
                <IconButton
                  aria-label="Detener grabación"
                  icon={<Square className="fill-current" />}
                  variant="solid"
                  size="xl"
                  disabled={isLoading}
                  onClick={handleStop}
                  className="border-rec/40 text-rec hover:bg-rec-dim"
                />
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </PanelBody>
    </Panel>
  )
}
