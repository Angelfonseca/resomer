import { useCallback, useEffect, useState } from "react"
import { invoke } from "@tauri-apps/api/core"

export function useApiKeyStatus() {
  const [hasKey, setHasKey] = useState<boolean | null>(null)

  const refresh = useCallback(async () => {
    try {
      const key = await invoke<string | null>("get_api_key")
      setHasKey(Boolean(key))
    } catch {
      setHasKey(false)
    }
  }, [])

  useEffect(() => {
    refresh()
  }, [refresh])

  return { hasKey, refresh }
}
