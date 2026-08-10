# Plan: Sidebar “solo actividad” + notificación de pipeline + categorías

Handoff para un modelo ejecutor. Sigue las fases **en orden**. Cada fase compila/
corre por sí sola. No refactorices nada fuera de lo listado.

Contexto del flujo actual (ya verificado):
- El sidebar ([src/components/layout/Sidebar.tsx](src/components/layout/Sidebar.tsx)) recibe `meetings: Meeting[]` y hoy muestra `meetings.slice(0, 8)`.
- `Meeting.state` = `"recording" | "processing" | "completed" | "error"`.
- Cuando el pipeline termina, `PipelineOrchestrator` llama `onMeetingUpdated` → `refreshMeetings()` en [src/App.tsx](src/App.tsx), y el `state` de esa reunión pasa a `"completed"`.
- La lista completa vive en la vista Reuniones → [MeetingsBrowser.tsx](src/features/meetings/components/MeetingsBrowser.tsx).

---

## FASE A — Sidebar = solo grabaciones activas + notificación “Pipeline terminado” (20 s)

**Solo frontend. Sin tocar Rust.**

### A1. Nuevo hook `useSidebarActivity`

Crear [src/features/meetings/hooks/useSidebarActivity.ts](src/features/meetings/hooks/useSidebarActivity.ts):

```ts
import { useEffect, useRef, useState } from "react"
import type { Meeting } from "./useMeetings"

const LINGER_MS = 20_000

export interface ActivityItem {
  meeting: Meeting
  justFinished: boolean // mostrar "Pipeline terminado"; se auto-oculta a los 20s
}

/**
 * Deriva qué reuniones mostrar en el sidebar: solo las activas
 * (recording / processing / error) más las que ACABAN de completarse, que
 * permanecen 20s con el aviso "Pipeline terminado" antes de desaparecer del
 * sidebar (siguen en la vista Reuniones).
 */
export function useSidebarActivity(meetings: Meeting[]): ActivityItem[] {
  const prevState = useRef<Map<string, Meeting["state"]>>(new Map())
  const initialized = useRef(false)
  const [finished, setFinished] = useState<Map<string, number>>(new Map())

  // Detectar transiciones *→completed.
  useEffect(() => {
    const prev = prevState.current
    // Primera pasada: sembrar estados sin emitir transiciones (evita marcar
    // como "recién terminadas" reuniones ya completas al abrir la app).
    if (!initialized.current) {
      for (const m of meetings) prev.set(m.id, m.state)
      initialized.current = true
      return
    }
    const newly: string[] = []
    for (const m of meetings) {
      const before = prev.get(m.id)
      if (m.state === "completed" && before && before !== "completed") newly.push(m.id)
      prev.set(m.id, m.state)
    }
    if (newly.length) {
      const now = Date.now()
      setFinished((f) => {
        const next = new Map(f)
        for (const id of newly) next.set(id, now)
        return next
      })
    }
  }, [meetings])

  // Vencer las que ya cumplieron 20s.
  useEffect(() => {
    if (finished.size === 0) return
    const timers = [...finished.entries()].map(([id, at]) =>
      window.setTimeout(() => {
        setFinished((f) => {
          const next = new Map(f)
          next.delete(id)
          return next
        })
      }, Math.max(0, LINGER_MS - (Date.now() - at)))
    )
    return () => timers.forEach(clearTimeout)
  }, [finished])

  const active = meetings.filter(
    (m) => m.state === "recording" || m.state === "processing" || m.state === "error"
  )
  const lingering = meetings.filter((m) => m.state === "completed" && finished.has(m.id))
  return [
    ...active.map((meeting) => ({ meeting, justFinished: false })),
    ...lingering.map((meeting) => ({ meeting, justFinished: true })),
  ]
}
```

Nota: incluimos `error` en “activas” porque es “no completada” y accionable (reintentar).

### A2. Sidebar consume la actividad

En [Sidebar.tsx](src/components/layout/Sidebar.tsx):
- Importar `useSidebarActivity` y llamarlo: `const activity = useSidebarActivity(meetings)`.
- Borrar `const recentMeetings = meetings.slice(0, 8)`.
- Renombrar el encabezado de la sección de `"Reuniones"` a **`"En curso"`** (el `<span>` mono y el botón que navega a `meetings`). Mantener el “Ver todas” a la derecha, visible siempre que `meetings.length > 0`.
- Reemplazar el `.map(recentMeetings)` por `.map(activity)`:
  - Item base: `StatusDot` con tono por estado + título + tiempo relativo (reutiliza `relativeTime`, ya existe en el archivo).
  - Si `justFinished`: mostrar en vez del subtítulo de tiempo una fila con check verde y texto `"Pipeline terminado"` (usar clase `text-done`), envuelta en `motion.div` con `initial={{opacity:0}} animate={{opacity:1}}` (motion/react ya se importa). Debe seguir siendo clickeable para abrir la reunión.
  - `processing`: subtítulo `"Procesando…"` en `text-signal`.
  - `error`: subtítulo `"Error · reintentar"` en `text-rec`, clickeable → abre la reunión (cae en el pipeline para reintentar).
  - `recording`: mantener `StatusDot pulse`.
- Empty state nuevo cuando `activity.length === 0`: `"No hay grabaciones activas."`
- `canOpen`: igual que hoy (`onOpenMeeting && audio_path && state !== "recording"`).

**Check A (manual):** graba/procesa una reunión; al completar, su fila muestra “Pipeline terminado” y desaparece del sidebar a los ~20 s, pero sigue en Reuniones. Al reabrir la app, las reuniones ya completas **no** muestran el aviso.

---

## FASE B — Categorías (etiqueta por usuario) + subsecciones por fecha

**Backend Rust + frontend.** Patrón de migración aditiva ya existe en el repo (mismo estilo que `expected_speakers`).

### B1. Backend — columna `category`

1. **Migración** en [storage.rs](src-tauri/src/services/storage.rs) `migrate()`, junto a las otras ~línea 130. Copia exacta del patrón (traga “duplicate column name”):
   ```rust
   if let Err(e) = conn.execute("ALTER TABLE meetings ADD COLUMN category TEXT", []) {
       let msg = e.to_string();
       if !msg.contains("duplicate column name") {
           return Err(ResomerError::Storage(format!("Migration failed: {}", msg)));
       }
   }
   ```
   (No lo pongas en el `CREATE TABLE`; déjalo solo como ALTER para no romper bases existentes; en bases nuevas el ALTER corre igual sobre la tabla recién creada.)

2. **Dominio** [meeting.rs](src-tauri/src/domain/meeting.rs): añade `pub category: Option<String>,` al struct `Meeting` y `category: None,` en `Meeting::new`.

3. **storage.rs `create` (INSERT)** ~línea 800: añade `category` a columnas y a `params!` (pasa de 7 a 8 valores). Usa `&meeting.category`.

4. **storage.rs `get` y `list`** (~820 y ~850): añade `category` al `SELECT` y mapea `category: row.get(N)?` (nuevo índice al final).

5. **Nuevo método en el repo** — replica `set_meeting_title` (storage.rs:778) como `set_meeting_category`:
   ```rust
   pub fn set_meeting_category(&self, meeting_id: &str, category: Option<&str>) -> Result<(), ResomerError> {
       let conn = self.get_connection()?;
       conn.execute(
           "UPDATE meetings SET category = ?1, updated_at = ?2 WHERE id = ?3",
           params![category, chrono::Utc::now().to_rfc3339(), meeting_id],
       )
       .map_err(|e| ResomerError::Storage(format!("Update meeting category failed: {}", e)))?;
       Ok(())
   }
   ```

6. **Comando Tauri** en [lib.rs](src-tauri/src/lib.rs), junto a `update_meeting_title` (~línea 652):
   ```rust
   #[tauri::command] // usa el mismo attr que los comandos vecinos si aplica
   pub async fn update_meeting_category(meeting_id: String, category: Option<String>) -> Result<(), String> {
       let repo = get_database()?;
       repo.set_meeting_category(&meeting_id, category.as_deref())
           .map_err(|e| e.to_string())
   }
   ```
   (Copia el `#[tauri::command]`/atributos exactos de `update_meeting_title` — revisa cómo está anotado ahí.)

7. **Registrar** `update_meeting_category` en `generate_handler!` de [main.rs:451](src-tauri/src/main.rs#L451).

**Check B1:** `cargo build` en `src-tauri` compila sin warnings de campos faltantes.

### B2. Frontend — modelo y mutación

1. [useMeetings.ts](src/features/meetings/hooks/useMeetings.ts): añade `category: string | null` a la interfaz `Meeting`. Añade acción:
   ```ts
   const setMeetingCategory = useCallback(async (id: string, category: string | null) => {
     setMeetings((prev) => prev.map((m) => (m.id === id ? { ...m, category } : m)))
     try {
       await invoke("update_meeting_category", { meetingId: id, category })
     } catch (err) {
       setError(err instanceof Error ? err.message : String(err))
     }
   }, [])
   ```
   Exponerla en el `return`.

2. Propagar `onSetCategory` por [App.tsx](src/App.tsx) → `MeetingsView` → `MeetingsBrowser` → `MeetingRow` (mismo camino que `onDeleteMeeting`).

### B3. Frontend — agrupación y UI en `MeetingsBrowser`

En [MeetingsBrowser.tsx](src/features/meetings/components/MeetingsBrowser.tsx):

1. Helper de bucket por fecha (fuera del componente):
   ```ts
   function dateBucket(iso: string): string {
     const d = new Date(iso)
     const now = new Date()
     const days = Math.floor((now.setHours(0,0,0,0) - new Date(d).setHours(0,0,0,0)) / 86_400_000)
     if (days <= 0) return "Hoy"
     if (days === 1) return "Ayer"
     if (days < 7) return "Esta semana"
     if (days < 30) return "Este mes"
     return "Anteriores"
   }
   const BUCKET_ORDER = ["Hoy", "Ayer", "Esta semana", "Este mes", "Anteriores"]
   const UNCATEGORIZED = "Sin categoría"
   ```

2. Tras aplicar búsqueda + filtro de estado + orden (lógica actual `filteredSorted`), agrupa en dos niveles:
   - Nivel 1: por `meeting.category ?? UNCATEGORIZED`. Orden: categorías con nombre primero (alfabético), `Sin categoría` al final.
   - Nivel 2 dentro de cada categoría: por `dateBucket(created_at)`, en `BUCKET_ORDER`.

3. Render: por cada categoría, un encabezado sticky (`font-mono text-[11px] uppercase tracking` estilo del eyebrow existente) con el nombre y el conteo `(n)`. Dentro, por cada bucket no vacío, un subencabezado más tenue, y debajo los `MeetingRow`.

4. Las categorías disponibles para asignar = unión de `{categorías distintas existentes}` + defaults `["Clientes", "Interno", "Personal"]`, ordenadas y sin duplicados.

En [MeetingRow.tsx](src/features/meetings/components/MeetingRow.tsx):
- Añade un control de categoría a la derecha (antes del estado): un `<select>` simple o un pequeño menú con las categorías disponibles + opción `"Sin categoría"` + `"+ Nueva…"` (esta última hace `prompt()` para el nombre — lazy, sin modal nuevo). Al elegir, llama `onSetCategory(meeting.id, value | null)`. `stopPropagation` en el control para no disparar `onOpen`.
- Recibe `categories: string[]` y `onSetCategory` por props.

**Check B (manual):** asigna una categoría a una reunión → persiste tras recargar la app; la vista Reuniones muestra encabezados de categoría y, dentro, subsecciones por fecha; “Sin categoría” agrupa el resto.

---

## Orden de ejecución y verificación

1. Fase A completa → probar en la app (grabar → completar → ver aviso 20s).
2. Fase B1 (Rust) → `cd src-tauri && cargo build`.
3. Fase B2 + B3 (TS) → `pnpm build` / correr la app; asignar categorías.

## Fuera de alcance (no hacer salvo que se pida)
- Editar/renombrar/borrar categorías como entidad (se derivan de las reuniones).
- Persistir el estado “recién terminado” entre reinicios (es efímero a propósito).
- Filtro por categoría en el buscador (los encabezados ya seccionan).
