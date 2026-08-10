import { MeetingsBrowser } from "./MeetingsBrowser"
import type { Meeting } from "../hooks/useMeetings"

export function MeetingsView({
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
  return (
    <div className="mx-auto w-full max-w-4xl">
      <MeetingsBrowser
        meetings={meetings}
        onDeleteMeeting={onDeleteMeeting}
        onOpenMeeting={onOpenMeeting}
        onSetCategory={onSetCategory}
      />
    </div>
  )
}
