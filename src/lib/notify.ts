import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification"

// Evita repetir el prompt del sistema si el usuario ya lo denegó una vez en
// esta sesión.
let askedThisSession = false

async function ensurePermission(): Promise<boolean> {
  if (await isPermissionGranted()) return true
  if (askedThisSession) return false
  askedThisSession = true
  return (await requestPermission()) === "granted"
}

/** Notificación nativa de macOS con el resultado final del pipeline. */
export async function notifyPipelineResult(title: string, body: string): Promise<void> {
  if (!(await ensurePermission())) return
  sendNotification({ title, body })
}
