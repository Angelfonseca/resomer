import { useCallback, useEffect, useState } from "react"
import { Mic } from "lucide-react"
import { AppShell, type View } from "./components/layout/AppShell"
import { Topbar } from "./components/layout/Topbar"
import { Button } from "./components/ui/Button"
import { StudioView } from "./features/studio/components/StudioView"
import { RecordingPanel } from "./features/recording"
import { PipelineOrchestrator } from "./features/pipeline"
import { ApiKeySettings } from "./features/settings/components/ApiKeySettings"
import { useApiKeyStatus } from "./features/settings/hooks/useApiKeyStatus"
import { useMeetings } from "./features/meetings/hooks/useMeetings"

const VIEW_META: Record<View, { title: string; subtitle?: string }> = {
  studio: { title: "Studio", subtitle: "Tu espacio de trabajo de reuniones" },
  recording: { title: "Grabar", subtitle: "Configura la fuente y comienza a grabar" },
  pipeline: { title: "Pipeline", subtitle: "Diarización → Transcripción → Resumen" },
  settings: { title: "Ajustes", subtitle: "Conexión con el proveedor de IA" },
}

function App() {
  const [currentView, setCurrentView] = useState<View>("studio")
  const [lastRecordingPath, setLastRecordingPath] = useState<string | null>(null)
  const { meetings, createMeeting, deleteMeeting, refreshMeetings } = useMeetings()
  const [activeMeetingId, setActiveMeetingId] = useState<string | null>(null)
  const { hasKey, refresh: refreshApiKeyStatus } = useApiKeyStatus()

  // Refrescar la lista al volver a Studio, para reflejar cambios de estado
  // ocurridos durante la grabación o el pipeline (procesando → completada).
  useEffect(() => {
    if (currentView === "studio") refreshMeetings()
  }, [currentView, refreshMeetings])

  const beginNewRecording = useCallback(async () => {
    const label = `Reunión ${meetings.length + 1}`
    try {
      const meeting = await createMeeting(label)
      setActiveMeetingId(meeting.id)
    } catch {
      // Sin conexión al backend todavía se puede grabar con un id local
      setActiveMeetingId(`local-${Date.now()}`)
    }
    setCurrentView("recording")
  }, [meetings.length, createMeeting])

  const meta = VIEW_META[currentView]

  return (
    <AppShell
      currentView={currentView}
      onNavigate={setCurrentView}
      meetings={meetings}
      pipelineEnabled={Boolean(lastRecordingPath)}
      hasApiKey={hasKey}
    >
      <Topbar
        title={meta.title}
        subtitle={meta.subtitle}
        action={
          currentView !== "recording" && (
            <Button leftIcon={<Mic className="size-4" />} onClick={beginNewRecording}>
              Nueva grabación
            </Button>
          )
        }
      />

      <main className="flex-1 overflow-y-auto px-8 py-8">
        {currentView === "studio" && (
          <StudioView
            meetings={meetings}
            onStartRecording={beginNewRecording}
            onDeleteMeeting={deleteMeeting}
          />
        )}

        {currentView === "recording" && (
          <div className="mx-auto w-full max-w-2xl">
            <RecordingPanel
              meetingId={activeMeetingId ?? `local-${Date.now()}`}
              onRecordingComplete={(filePath) => {
                setLastRecordingPath(filePath)
                setCurrentView("pipeline")
              }}
            />
          </div>
        )}

        {currentView === "pipeline" && (
          <div className="mx-auto w-full max-w-5xl">
            <PipelineOrchestrator
              audioPath={lastRecordingPath ?? undefined}
              meetingId={activeMeetingId ?? undefined}
            />
          </div>
        )}

        {currentView === "settings" && (
          <div className="mx-auto w-full max-w-2xl">
            <ApiKeySettings onKeyChange={refreshApiKeyStatus} />
          </div>
        )}
      </main>
    </AppShell>
  )
}

export default App
