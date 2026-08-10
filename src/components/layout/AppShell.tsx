import type { ReactNode } from "react"
import { Sidebar, type View } from "./Sidebar"
import type { Meeting } from "../../features/meetings/hooks/useMeetings"

export function AppShell({
  currentView,
  onNavigate,
  meetings,
  pipelineEnabled,
  hasApiKey,
  onOpenMeeting,
  children,
}: {
  currentView: View
  onNavigate: (view: View) => void
  meetings: Meeting[]
  pipelineEnabled: boolean
  hasApiKey: boolean | null
  onOpenMeeting?: (meeting: Meeting) => void
  children: ReactNode
}) {
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-canvas text-ink">
      <Sidebar
        currentView={currentView}
        onNavigate={onNavigate}
        meetings={meetings}
        pipelineEnabled={pipelineEnabled}
        hasApiKey={hasApiKey}
        onOpenMeeting={onOpenMeeting}
      />
      <div className="flex min-w-0 flex-1 flex-col">{children}</div>
    </div>
  )
}

export type { View }
