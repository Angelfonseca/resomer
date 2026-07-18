import { useEffect, useState } from "react"
import { Mic, Square, AlertCircle } from "lucide-react"
import { motion, AnimatePresence } from "motion/react"
import { useRecording, useAudioDevices, type RecordingSource } from "../hooks/useRecording"
import { useAudioMeter } from "../hooks/useAudioMeter"
import { useSyntheticMeter } from "../hooks/useSyntheticMeter"
import { Panel, PanelHeader, PanelBody } from "../../../components/ui/Panel"
import { IconButton } from "../../../components/ui/IconButton"
import { SelectField } from "../../../components/ui/Field"
import { StatusDot } from "../../../components/ui/StatusDot"
import { Timecode } from "../../../components/ui/Timecode"
import { Waveform } from "../../../components/ui/Waveform"

interface RecordingPanelProps {
  meetingId: string
  onRecordingComplete?: (filePath: string) => void
}

const SOURCE_LABEL: Record<RecordingSource, string> = {
  microphone: "Solo micrófono",
  system_audio: "Solo audio del sistema",
  both: "Ambos (estéreo)",
}

export function RecordingPanel({ meetingId, onRecordingComplete }: RecordingPanelProps) {
  const { recording, isLoading, error, startRecording, stopRecording } = useRecording()
  const { devices, isLoading: devicesLoading } = useAudioDevices()
  const [selectedSource, setSelectedSource] = useState<RecordingSource>("microphone")
  const [durationSeconds, setDurationSeconds] = useState(0)

  const isRecording = recording?.state === "recording"

  const liveMeter = useAudioMeter(isRecording)
  const syntheticLevels = useSyntheticMeter(isRecording && liveMeter.errored)
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
    await startRecording(meetingId, outputPath, selectedSource)
  }

  const handleStop = async () => {
    const filePath = recording?.filePath
    await stopRecording()
    if (filePath) onRecordingComplete?.(filePath)
  }

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
          <div className="mb-6 h-20">
            <Waveform levels={levels} active={isRecording} />
          </div>
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
