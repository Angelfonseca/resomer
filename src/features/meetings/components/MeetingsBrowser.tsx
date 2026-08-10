import { useMemo, useState } from "react"
import { ArrowDownAZ, ArrowUpAZ, ListMusic, Search } from "lucide-react"
import { IconButton } from "../../../components/ui/IconButton"
import { EmptyState } from "../../../components/ui/EmptyState"
import { cn } from "../../../lib/cn"
import type { Meeting } from "../hooks/useMeetings"
import { MeetingRow, meetingStateLabel } from "./MeetingRow"

type StatusFilter = "all" | Meeting["state"]

const STATUS_FILTERS: { id: StatusFilter; label: string }[] = [
  { id: "all", label: "Todas" },
  { id: "recording", label: meetingStateLabel.recording },
  { id: "processing", label: meetingStateLabel.processing },
  { id: "completed", label: meetingStateLabel.completed },
  { id: "error", label: meetingStateLabel.error },
]

const UNCATEGORIZED = "Sin categoría"
const DEFAULT_CATEGORIES = ["Clientes", "Interno", "Personal"]
const BUCKET_ORDER = ["Hoy", "Ayer", "Esta semana", "Este mes", "Anteriores"]

function startOfDay(d: Date): number {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime()
}

function dateBucket(iso: string): string {
  const days = Math.floor((startOfDay(new Date()) - startOfDay(new Date(iso))) / 86_400_000)
  if (days <= 0) return "Hoy"
  if (days === 1) return "Ayer"
  if (days < 7) return "Esta semana"
  if (days < 30) return "Este mes"
  return "Anteriores"
}

export function MeetingsBrowser({
  meetings,
  onDeleteMeeting,
  onOpenMeeting,
  onSetCategory,
}: {
  meetings: Meeting[]
  onDeleteMeeting?: (id: string) => void
  onOpenMeeting?: (meeting: Meeting) => void
  onSetCategory?: (id: string, category: string | null) => void
}) {
  const [query, setQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<StatusFilter>("all")
  const [sortAsc, setSortAsc] = useState(false)

  const filteredSorted = useMemo(() => {
    const q = query.trim().toLowerCase()
    const filtered = meetings.filter((m) => {
      const matchesQuery = !q || m.title.toLowerCase().includes(q)
      const matchesStatus = statusFilter === "all" || m.state === statusFilter
      return matchesQuery && matchesStatus
    })
    return [...filtered].sort((a, b) => {
      const diff = new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
      return sortAsc ? diff : -diff
    })
  }, [meetings, query, statusFilter, sortAsc])

  // Categorías disponibles para asignar: las que ya existen entre las
  // reuniones + un puñado de defaults, sin duplicados.
  const availableCategories = useMemo(() => {
    const set = new Set(DEFAULT_CATEGORIES)
    for (const m of meetings) if (m.category) set.add(m.category)
    return [...set].sort((a, b) => a.localeCompare(b))
  }, [meetings])

  // Nivel 1: por categoría (sin categoría al final). Nivel 2 dentro de cada
  // categoría: por antigüedad (Hoy/Ayer/Esta semana/Este mes/Anteriores).
  const grouped = useMemo(() => {
    const byCategory = new Map<string, Meeting[]>()
    for (const m of filteredSorted) {
      const cat = m.category ?? UNCATEGORIZED
      if (!byCategory.has(cat)) byCategory.set(cat, [])
      byCategory.get(cat)!.push(m)
    }
    const categoryNames = [...byCategory.keys()]
      .filter((c) => c !== UNCATEGORIZED)
      .sort((a, b) => a.localeCompare(b))
    if (byCategory.has(UNCATEGORIZED)) categoryNames.push(UNCATEGORIZED)

    return categoryNames.map((category) => {
      const items = byCategory.get(category)!
      const byBucket = new Map<string, Meeting[]>()
      for (const m of items) {
        const bucket = dateBucket(m.created_at)
        if (!byBucket.has(bucket)) byBucket.set(bucket, [])
        byBucket.get(bucket)!.push(m)
      }
      const buckets = BUCKET_ORDER.filter((b) => byBucket.has(b)).map((bucket) => ({
        bucket,
        items: byBucket.get(bucket)!,
      }))
      return { category, count: items.length, buckets }
    })
  }, [filteredSorted])

  const hasAnyMeetings = meetings.length > 0
  const hasResults = filteredSorted.length > 0

  return (
    <div className="space-y-4">
      {hasAnyMeetings && (
        <>
          <div className="flex items-center gap-2">
            <div className="relative flex-1">
              <Search className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-ink-mute" />
              <input
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Buscar reuniones por título…"
                className="w-full rounded-control border border-hairline-strong bg-canvas-raised py-2 pl-9 pr-3 text-sm text-ink placeholder:text-ink-mute transition-colors focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"
              />
            </div>
            <IconButton
              aria-label={sortAsc ? "Ordenar: más antiguas primero" : "Ordenar: más recientes primero"}
              icon={sortAsc ? <ArrowUpAZ /> : <ArrowDownAZ />}
              variant="solid"
              onClick={() => setSortAsc((v) => !v)}
            />
          </div>

          <div className="flex flex-wrap items-center gap-1.5">
            {STATUS_FILTERS.map((f) => (
              <button
                key={f.id}
                onClick={() => setStatusFilter(f.id)}
                className={cn(
                  "rounded-full border px-3 py-1 font-mono text-[11px] transition-colors",
                  statusFilter === f.id
                    ? "border-signal/40 bg-signal-dim text-signal"
                    : "border-hairline text-ink-dim hover:bg-panel-hover hover:text-ink"
                )}
              >
                {f.label}
              </button>
            ))}
          </div>
        </>
      )}

      {hasResults && (
        <div className="space-y-6">
          {grouped.map(({ category, count, buckets }) => (
            <div key={category} className="space-y-3">
              <div className="flex items-baseline gap-2 px-1">
                <h3 className="font-mono text-[11px] uppercase tracking-[0.14em] text-ink-mute">
                  {category}
                </h3>
                <span className="font-mono text-[10px] text-ink-mute">({count})</span>
              </div>
              {buckets.map(({ bucket, items }) => (
                <div key={bucket} className="space-y-2">
                  <p className="px-1 font-mono text-[10px] uppercase tracking-[0.1em] text-ink-mute/70">
                    {bucket}
                  </p>
                  {items.map((meeting) => (
                    <MeetingRow
                      key={meeting.id}
                      meeting={meeting}
                      onDelete={onDeleteMeeting}
                      onOpen={onOpenMeeting}
                      categories={availableCategories}
                      onSetCategory={onSetCategory}
                    />
                  ))}
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      {hasAnyMeetings && !hasResults && (
        <EmptyState
          icon={<Search />}
          title="Sin resultados"
          description={
            query
              ? `Ninguna reunión coincide con "${query}".`
              : "Ninguna reunión coincide con este filtro."
          }
        />
      )}

      {!hasAnyMeetings && (
        <EmptyState
          icon={<ListMusic />}
          title="Aún no hay reuniones"
          description="Cuando grabes tu primera reunión, aparecerá aquí junto con su estado de procesamiento."
        />
      )}
    </div>
  )
}
