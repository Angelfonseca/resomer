import { useCallback, useEffect, useRef, useState, useMemo } from "react"
import { Mic } from "lucide-react"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { AppShell, type View } from "./components/layout/AppShell"
import { Topbar } from "./components/layout/Topbar"
import { Button } from "./components/ui/Button"
import { StudioView } from "./features/studio/components/StudioView"
import { RecordingPanel, type RecordingSource } from "./features/recording"
import { PipelineOrchestrator } from "./features/pipeline"
import { ApiKeySettings } from "./features/settings/components/ApiKeySettings"
import { useApiKeyStatus } from "./features/settings/hooks/useApiKeyStatus"
import { useMeetings, type Meeting } from "./features/meetings/hooks/useMeetings"
import { MeetingsView } from "./features/meetings/components/MeetingsView"
import { AssistantView } from "./features/chat"

const VIEW_META: Record<View, { title: string; subtitle?: string }> = {
  studio: { title: "Studio", subtitle: "Tu espacio de trabajo de reuniones" },
  recording: { title: "Grabar", subtitle: "Configura la fuente y comienza a grabar" },
  pipeline: { title: "Pipeline", subtitle: "Diarización → Transcripción → Resumen" },
  meetings: { title: "Reuniones", subtitle: "Busca y accede a tus reuniones" },
  assistant: { title: "Asistente", subtitle: "Chatea con la IA sobre todas tus reuniones" },
  settings: { title: "Ajustes", subtitle: "Conexión con el proveedor de IA" },
}

function App() {
  const [currentView, setCurrentView] = useState<View>("studio")
  const [lastRecordingPath, setLastRecordingPath] = useState<string | null>(null)
  const { meetings, createMeeting, deleteMeeting, setMeetingCategory, refreshMeetings } = useMeetings()
  const [activeMeetingId, setActiveMeetingId] = useState<string | null>(null)
  const [expectedSpeakers, setExpectedSpeakers] = useState<number | null>(null)
  // Fuente elegida en el menú del ícono de la barra de estado; preselecciona
  // el modo de grabación al abrir el panel.
  const [recordingSource, setRecordingSource] = useState<RecordingSource>("microphone")
  // Grabación iniciada desde el ícono de la barra de estado (el backend ya la
  // arrancó): la reflejamos en el panel en vez de arrancar otra.
  const [mirror, setMirror] = useState<{
    meetingId: string
    filePath: string
    source: RecordingSource
  } | null>(null)
  // Espejo de `mirror` accesible desde listeners sin recrearlos en cada cambio.
  const mirrorRef = useRef(mirror)
  mirrorRef.current = mirror
  const { hasKey, refresh: refreshApiKeyStatus } = useApiKeyStatus()

  const availableCategories = useMemo(() => {
    const set = new Set(["Clientes", "Interno", "Personal"])
    for (const m of meetings) if (m.category) set.add(m.category)
    return [...set].sort((a, b) => a.localeCompare(b))
  }, [meetings])

  // Refrescar la lista al volver a Studio o a Reuniones, para reflejar
  // cambios de estado ocurridos durante la grabación o el pipeline
  // (procesando → completada).
  useEffect(() => {
    if (currentView === "studio" || currentView === "meetings") refreshMeetings()
  }, [currentView, refreshMeetings])

  const beginNewRecording = useCallback(
    async (source: RecordingSource = "microphone", category: string | null = null) => {
      const label = `Reunión ${meetings.length + 1}`
      setExpectedSpeakers(null)
      setRecordingSource(source)
      setMirror(null) // grabación nueva desde la UI, no un reflejo del tray
      try {
        const meeting = await createMeeting(label, category)
        setActiveMeetingId(meeting.id)
      } catch {
        // Sin conexión al backend todavía se puede grabar con un id local
        setActiveMeetingId(`local-${Date.now()}`)
      }
      setCurrentView("recording")
    },
    [meetings.length, createMeeting]
  )

  // Abrir una reunión existente: muestra sus resultados guardados, o
  // permite ejecutar/reintentar el pipeline si aún no los tiene.
  const openMeeting = useCallback((meeting: Meeting) => {
    if (!meeting.audio_path) return
    setActiveMeetingId(meeting.id)
    setLastRecordingPath(meeting.audio_path)
    setExpectedSpeakers(meeting.expected_speakers)
    setCurrentView("pipeline")
  }, [])

  // Puente para las citas del asistente global, que solo conocen el id de
  // la reunión (no el objeto completo) — lo resuelve contra la lista ya
  // cargada en memoria antes de reusar el flujo normal de apertura.
  const openMeetingById = useCallback(
    (meetingId: string) => {
      const meeting = meetings.find((m) => m.id === meetingId)
      if (meeting) openMeeting(meeting)
    },
    [meetings, openMeeting]
  )

  // Acciones lanzadas desde el ícono de la barra de estado de macOS. La
  // grabación la controla el backend (arranca/detiene el grabador y actualiza
  // el indicador "REC"); aquí solo reflejamos ese estado en la UI cuando la
  // app está abierta.
  useEffect(() => {
    let cancelled = false
    const unlisteners: Array<() => void> = []
    const register = (promise: Promise<() => void>) => {
      promise.then((un) => (cancelled ? un() : unlisteners.push(un)))
    }

    type ActiveRecording = { meetingId: string; audioPath: string; source: RecordingSource }

    const enterMirror = (a: ActiveRecording) => {
      setActiveMeetingId(a.meetingId)
      setExpectedSpeakers(null)
      setMirror({ meetingId: a.meetingId, filePath: a.audioPath, source: a.source })
      setCurrentView("recording")
    }

    // El backend arrancó una grabación desde el tray → reflejarla en curso.
    register(listen<ActiveRecording>("recording-started", (e) => enterMirror(e.payload)))

    // El backend detuvo la grabación desde el tray → ir al pipeline.
    register(
      listen<ActiveRecording>("recording-stopped", (e) => {
        setMirror(null)
        setActiveMeetingId(e.payload.meetingId)
        setLastRecordingPath(e.payload.audioPath)
        setCurrentView("pipeline")
      })
    )

    register(
      listen<string>("recording-error", (e) => {
        console.error("Grabación (barra de estado):", e.payload)
      })
    )

    register(
      listen("tray-run-pipeline", () => {
        // Solo tiene sentido si ya hay audio grabado listo para procesar.
        setCurrentView((v) => (lastRecordingPath ? "pipeline" : v))
      })
    )
    register(
      listen("tray-open-assistant", () => {
        setCurrentView("assistant")
      })
    )

    // Al abrir/reactivar la app, reconciliar con el estado real del backend:
    // el tray pudo iniciar o detener una grabación mientras el webview estaba
    // suspendido (ventana oculta), perdiéndose el evento correspondiente.
    const syncActive = async () => {
      let active: ActiveRecording | null = null
      try {
        active = await invoke<ActiveRecording | null>("get_active_recording")
      } catch {
        return
      }
      if (cancelled) return
      const current = mirrorRef.current
      if (active) {
        // Reflejar solo si aún no lo estábamos, para no arrancar de vuelta a la
        // vista de grabación si el usuario navegó a otra a propósito.
        if (!current || current.meetingId !== active.meetingId) enterMirror(active)
      } else if (current) {
        // La grabación reflejada se detuvo estando la ventana oculta.
        setMirror(null)
        setActiveMeetingId(current.meetingId)
        setLastRecordingPath(current.filePath)
        setCurrentView("pipeline")
      }
    }

    // Sondeo periódico: es el mecanismo fiable de reflejo. Al mostrar una
    // ventana oculta, macOS no dispara `focus`/`visibilitychange` del DOM de
    // forma consistente en el webview, así que un intervalo (que solo corre
    // cuando la ventana está visible y su JS activo) garantiza que la UI se
    // ponga al día con el estado real del backend en ~1.5s. Los eventos de
    // arriba dan respuesta inmediata cuando la app ya está en primer plano.
    syncActive()
    const onFocus = () => syncActive()
    const onVisible = () => {
      if (!document.hidden) syncActive()
    }
    window.addEventListener("focus", onFocus)
    document.addEventListener("visibilitychange", onVisible)
    const interval = window.setInterval(syncActive, 1500)

    return () => {
      cancelled = true
      unlisteners.forEach((un) => un())
      window.removeEventListener("focus", onFocus)
      document.removeEventListener("visibilitychange", onVisible)
      window.clearInterval(interval)
    }
  }, [lastRecordingPath])

  const meta = VIEW_META[currentView]

  return (
    <AppShell
      currentView={currentView}
      onNavigate={setCurrentView}
      meetings={meetings}
      pipelineEnabled={Boolean(lastRecordingPath)}
      hasApiKey={hasKey}
      onOpenMeeting={openMeeting}
    >
      <Topbar
        title={meta.title}
        subtitle={meta.subtitle}
        action={
          currentView !== "recording" && (
            <Button leftIcon={<Mic className="size-4" />} onClick={() => beginNewRecording()}>
              Nueva grabación
            </Button>
          )
        }
      />

      <main className="flex-1 overflow-y-auto px-8 py-8">
        {currentView === "studio" && (
          <StudioView
            meetings={meetings}
            onStartRecording={() => beginNewRecording()}
            onDeleteMeeting={deleteMeeting}
            onOpenMeeting={openMeeting}
            onSetCategory={setMeetingCategory}
          />
        )}

        {currentView === "recording" && (
          <div className="mx-auto w-full max-w-2xl">
            <RecordingPanel
              meetingId={mirror?.meetingId ?? activeMeetingId ?? `local-${Date.now()}`}
              initialSource={mirror?.source ?? recordingSource}
              attach={mirror}
              categories={availableCategories}
              onSetCategory={setMeetingCategory}
              initialCategory={activeMeetingId ? meetings.find((m) => m.id === activeMeetingId)?.category : null}
              onRecordingComplete={(filePath, speakers) => {
                setMirror(null)
                setLastRecordingPath(filePath)
                setExpectedSpeakers(speakers)
                setCurrentView("pipeline")
              }}
            />
          </div>
        )}

        {/* El pipeline se mantiene montado aunque cambies de pestaña: así el
            procesamiento en curso (y su progreso) no se pierde. Solo se oculta
            visualmente cuando la vista activa es otra. */}
        {lastRecordingPath && (
          <div className={currentView === "pipeline" ? "mx-auto w-full max-w-5xl" : "hidden"}>
            <PipelineOrchestrator
              audioPath={lastRecordingPath}
              meetingId={activeMeetingId ?? undefined}
              meetingTitle={meetings.find((m) => m.id === activeMeetingId)?.title}
              meetingCategory={meetings.find((m) => m.id === activeMeetingId)?.category}
              expectedSpeakers={expectedSpeakers}
              categories={availableCategories}
              onSetCategory={setMeetingCategory}
              onMeetingUpdated={refreshMeetings}
            />
          </div>
        )}

        {currentView === "meetings" && (
          <MeetingsView
            meetings={meetings}
            onDeleteMeeting={deleteMeeting}
            onOpenMeeting={openMeeting}
            onSetCategory={setMeetingCategory}
          />
        )}

        {currentView === "assistant" && (
          <AssistantView onOpenMeetingById={openMeetingById} />
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
