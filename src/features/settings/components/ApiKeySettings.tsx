import { useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"
import { CheckCircle2, KeyRound, Trash2, XCircle } from "lucide-react"
import { Panel, PanelHeader, PanelBody } from "../../../components/ui/Panel"
import { TextField } from "../../../components/ui/Field"
import { Button } from "../../../components/ui/Button"

interface ApiKeySettingsProps {
  onKeyChange?: () => void
}



export function ApiKeySettings({ onKeyChange }: ApiKeySettingsProps) {
  const [apiKey, setApiKey] = useState("")
  const [maskedKey, setMaskedKey] = useState("")
  const [saving, setSaving] = useState(false)
  const [testing, setTesting] = useState(false)
  const [removing, setRemoving] = useState(false)
  const [result, setResult] = useState<{ tone: "success" | "error"; message: string } | null>(null)

  const loadApiKey = async () => {
    try {
      // El backend nunca expone la clave en claro, solo su estado enmascarado.
      const status = await invoke<{ configured: boolean; masked: string | null }>(
        "get_api_key_status"
      )
      setMaskedKey(status.masked ?? "")
    } catch (err) {
      setResult({ tone: "error", message: `No se pudo leer el estado de la clave: ${err}` })
    }
  }

  useEffect(() => {
    loadApiKey()
  }, [])

  const handleSave = async () => {
    if (!apiKey.trim()) {
      setResult({ tone: "error", message: "La clave no puede estar vacía." })
      return
    }
    setSaving(true)
    setResult(null)
    try {
      await invoke("save_api_key", { apiKey })
      await loadApiKey()
      setApiKey("")
      setResult({ tone: "success", message: "Clave guardada en el llavero del sistema." })
      onKeyChange?.()
    } catch (err) {
      setResult({ tone: "error", message: `Error al guardar: ${err}` })
    } finally {
      setSaving(false)
    }
  }

  const handleTestConnection = async () => {
    setTesting(true)
    setResult(null)
    try {
      // La prueba usa la clave guardada y el endpoint configurado en el
      // backend; el frontend no envía ninguno de los dos.
      const response = await invoke<{ success: boolean; message: string }>("test_connection")
      setResult({ tone: response.success ? "success" : "error", message: response.message })
    } catch (err) {
      setResult({ tone: "error", message: `Prueba fallida: ${err}` })
    } finally {
      setTesting(false)
    }
  }

  const handleDelete = async () => {
    setRemoving(true)
    setResult(null)
    try {
      await invoke("delete_api_key")
      setApiKey("")
      setMaskedKey("")
      setResult({ tone: "success", message: "Clave eliminada del llavero." })
      onKeyChange?.()
    } catch (err) {
      setResult({ tone: "error", message: `Error al eliminar: ${err}` })
    } finally {
      setRemoving(false)
    }
  }

  return (
    <div className="space-y-4">
      <Panel raised>
        <PanelHeader eyebrow="Conexión" title="Clave de API" />
        <PanelBody className="space-y-4">
          {maskedKey ? (
            <div className="flex items-center gap-3">
              <div className="flex flex-1 items-center gap-2 rounded-control border border-hairline-strong bg-canvas-raised px-3 py-2">
                <KeyRound className="size-4 text-ink-mute" />
                <span className="font-mono text-sm text-ink-dim">{maskedKey}</span>
              </div>
              <Button
                variant="outline"
                size="md"
                leftIcon={<Trash2 className="size-4" />}
                loading={removing}
                onClick={handleDelete}
              >
                Quitar
              </Button>
            </div>
          ) : (
            <TextField
              label="Clave"
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder="sk-…"
            />
          )}

          <div className="flex gap-2">
            {!maskedKey && (
              <Button loading={saving} disabled={!apiKey} onClick={handleSave}>
                Guardar
              </Button>
            )}
            <Button variant="subtle" loading={testing} onClick={handleTestConnection}>
              Probar conexión
            </Button>
          </div>

          {result && (
            <div
              className={
                result.tone === "success"
                  ? "flex items-center gap-2 rounded-control border border-done/30 bg-done-dim px-3 py-2.5 text-sm text-ink"
                  : "flex items-center gap-2 rounded-control border border-rec/30 bg-rec-dim px-3 py-2.5 text-sm text-ink"
              }
            >
              {result.tone === "success" ? (
                <CheckCircle2 className="size-4 shrink-0 text-done" />
              ) : (
                <XCircle className="size-4 shrink-0 text-rec" />
              )}
              <span>{result.message}</span>
            </div>
          )}
        </PanelBody>
      </Panel>

      <p className="px-1 text-xs text-ink-mute">
        La clave se guarda de forma segura en el llavero de tu sistema operativo y nunca se expone
        dentro de la aplicación.
      </p>
    </div>
  )
}
