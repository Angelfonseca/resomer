import { useCallback, useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"

export function useApiKeyStatus() {
  const [hasKey, setHasKey] = useState<boolean | null>(null)

  const refresh = useCallback(async () => {
    try {
      const status = await invoke<{ configured: boolean }>("get_api_key_status")
      setHasKey(status.configured)
    } catch {
      setHasKey(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  return { hasKey, refresh }
}
