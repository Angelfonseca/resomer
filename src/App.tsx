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
import { DEFAULT_CATEGORIES } from "./lib/meetings"

// Forma de la grabación activa tal como la devuelve el backend (camelCase).
type ActiveRecording = { meetingId: string; audioPath: string; source: RecordingSource }

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
  const {
    meetings,
    createMeeting,
    deleteMeeting,
    setMeetingCategory,
    refreshMeetings,
    error: meetingsError,
  } = useMeetings()
  const [activeMeetingId, setActiveMeetingId] = useState<string | null>(null)
  const [expectedSpeakers, setExpectedSpeakers] = useState<number | null>(null)
  // Id estable para el caso (raro) de grabar sin reunión creada todavía. Antes
  // se generaba `local-${Date.now()}` en cada render, así que el id cambiaba
  // constantemente y la grabación quedaba huérfana.
  const [localMeetingId] = useState(() => `local-${Date.now()}`)
  const [trayError, setTrayError] = useState<string | null>(null)
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
    const set = new Set(DEFAULT_CATEGORIES)
    for (const m of meetings) if (m.category) set.add(m.category)
    return [...set].sort((a, b) => a.localeCompare(b))
  }, [meetings])

  // Refrescar la lista al volver a Studio o a Reuniones, para reflejar
  // cambios de estado ocurridos durante la grabación o el pipeline
  // (procesando → completada).
  useEffect(() => {
    if (currentView === "studio" || currentView === "meetings") refreshMeetings()
  }, [currentView, refreshMeetings])

  // Adopta una grabación que ya corre en el backend: vuelve a la vista de
  // grabación sin crear otra reunión ni reiniciar el grabador.
  const followRecording = useCallback((a: ActiveRecording) => {
    setActiveMeetingId(a.meetingId)
    setExpectedSpeakers(null)
    setTrayError(null)
    // La reunión que se está grabando aún no tiene pipeline; descarta el del
    // audio anterior para no mezclar su id con la ruta vieja.
    setLastRecordingPath(null)
    setMirror({ meetingId: a.meetingId, filePath: a.audioPath, source: a.source })
    setCurrentView("recording")
  }, [])

  const beginNewRecording = useCallback(
    async (source: RecordingSource = "microphone", category: string | null = null) => {
      // Si el backend ya tiene una grabación en curso (no importa si se inició
      // desde el tray o desde esta misma pantalla), seguirla. Crear otra aquí
      // cerraba la actual en la UI y dejaba el grabador corriendo sin control.
      const active = await invoke<ActiveRecording | null>("get_active_recording").catch(() => null)
      if (active) {
        followRecording(active)
        return
      }

      const label = `Reunión ${meetings.length + 1}`
      setExpectedSpeakers(null)
      setRecordingSource(source)
      setMirror(null) // grabación nueva desde la UI, no un reflejo del tray
      // La grabación nueva aún no tiene pipeline: descarta el del audio
      // anterior para que no se muestre (ni se reutilice por error) al
      // procesar la nueva reunión.
      setLastRecordingPath(null)
      try {
        const meeting = await createMeeting(label, category)
        setActiveMeetingId(meeting.id)
      } catch {
        // Sin conexión al backend todavía se puede grabar con un id local
        setActiveMeetingId(`local-${Date.now()}`)
      }
      setCurrentView("recording")
    },
    [meetings.length, createMeeting, followRecording]
  )

  // Navegación desde la barra lateral. Si se entra a "Grabar" sin una
  // grabación en curso, se crea la reunión primero (igual que el botón "Nueva
  // grabación"): así nunca se graba sobre una reunión existente ni con un id
  // inventado en cada render.
  const handleNavigate = useCallback(
    (view: View) => {
      if (view === "recording" && !mirror) {
        void beginNewRecording()
        return
      }
      setCurrentView(view)
    },
    [mirror, beginNewRecording]
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

    const enterMirror = followRecording

    // El backend arrancó una grabación desde el tray → reflejarla en curso.
    register(listen<ActiveRecording>("recording-started", (e) => enterMirror(e.payload)))

    // El backend detuvo la grabación desde el tray → ir al pipeline.
    register(
      listen<ActiveRecording>("recording-stopped", (e) => {
        setMirror(null)
        setTrayError(null)
        setActiveMeetingId(e.payload.meetingId)
        setLastRecordingPath(e.payload.audioPath)
        setCurrentView("pipeline")
        refreshMeetings()
      })
    )

    register(
      listen<string>("recording-error", (e) => {
        // Antes solo se escribía en consola y el usuario no veía nada cuando
        // una grabación lanzada desde el tray fallaba (p. ej. permiso denegado
        // o "ya hay una grabación en curso").
        setTrayError(e.payload)
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
        refreshMeetings()
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
  }, [lastRecordingPath, refreshMeetings, followRecording])

  const meta = VIEW_META[currentView]

  return (
    <AppShell
      currentView={currentView}
      onNavigate={handleNavigate}
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
        {trayError && (
          <div className="mx-auto mb-6 flex w-full max-w-2xl items-start justify-between gap-3 rounded-control border border-rec/30 bg-rec-dim px-4 py-3 text-sm text-ink">
            <span>{trayError}</span>
            <button
              onClick={() => setTrayError(null)}
              className="shrink-0 font-mono text-[11px] uppercase tracking-wide text-ink-dim hover:text-ink"
            >
              Cerrar
            </button>
          </div>
        )}

        {meetingsError && (
          <div className="mx-auto mb-6 w-full max-w-2xl rounded-control border border-rec/30 bg-rec-dim px-4 py-3 text-sm text-ink">
            {meetingsError}
          </div>
        )}

        {currentView === "studio" && (
          <StudioView
            meetings={meetings}
            onStartRecording={() => beginNewRecording()}
            onDeleteMeeting={deleteMeeting}
            onOpenMeeting={openMeeting}
            onSetCategory={setMeetingCategory}
          />
        )}

        {/* El panel se mantiene montado mientras hay una grabación en curso,
            aunque cambies de vista: así el cronómetro, el medidor y el botón de
            detener siguen ahí al volver, en vez de reiniciarse. */}
        {(currentView === "recording" || mirror) && (
          <div className={currentView === "recording" ? "mx-auto w-full max-w-2xl" : "hidden"}>
            <RecordingPanel
              meetingId={mirror?.meetingId ?? activeMeetingId ?? localMeetingId}
              initialSource={mirror?.source ?? recordingSource}
              attach={mirror}
              categories={availableCategories}
              onSetCategory={setMeetingCategory}
              initialCategory={activeMeetingId ? meetings.find((m) => m.id === activeMeetingId)?.category : null}
              onRecordingStarted={setMirror}
              onRecordingComplete={(filePath, speakers) => {
                setMirror(null)
                setLastRecordingPath(filePath)
                setExpectedSpeakers(speakers)
                setCurrentView("pipeline")
                // Reflejar el cambio de estado (grabando → procesando) en la
                // barra lateral sin esperar a cambiar de vista.
                refreshMeetings()
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
              key={activeMeetingId ?? "sin-reunion"}
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
