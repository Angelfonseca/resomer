import { useMemo } from "react"
import { formatTimecode } from "../../../components/ui/Timecode"
import { CHANNEL_COLORS, speakerLabel } from "../speakerColors"
import type { Segment } from "../types"

export function SpeakerTimeline({ segments }: { segments: Segment[] }) {
  const { lanes, totalDuration } = useMemo(() => {
    const speakers = [...new Set(segments.map((s) => s.speaker))].sort()
    const total = segments.reduce((max, s) => Math.max(max, s.end), 0) || 1
    return {
      lanes: speakers.map((speaker, i) => ({
        speaker,
        color: CHANNEL_COLORS[i % CHANNEL_COLORS.length],
        segments: segments.filter((s) => s.speaker === speaker),
      })),
      totalDuration: total,
    }
  }, [segments])

  return (
    <div className="space-y-3">
      {/* Time axis */}
      <div className="flex justify-between font-mono text-[10px] text-ink-mute">
        <span>0:00</span>
        <span>{formatTimecode(totalDuration / 2)}</span>
        <span>{formatTimecode(totalDuration)}</span>
      </div>

      <div className="space-y-2">
        {lanes.map((lane) => (
          <div key={lane.speaker} className="flex items-center gap-3">
            <div className="flex w-28 shrink-0 items-center gap-2">
              <span
                className="size-2 shrink-0 rounded-full"
                style={{ backgroundColor: lane.color }}
              />
              <span className="truncate text-xs font-medium text-ink-dim">
                {speakerLabel(lane.speaker)}
              </span>
            </div>
            <div className="relative h-6 flex-1 overflow-hidden rounded-md bg-canvas-raised">
              {lane.segments.map((seg, i) => (
                <div
                  key={i}
                  className="absolute inset-y-0 rounded-sm opacity-80"
                  style={{
                    left: `${(seg.start / totalDuration) * 100}%`,
                    width: `${Math.max(0.6, ((seg.end - seg.start) / totalDuration) * 100)}%`,
                    backgroundColor: lane.color,
                  }}
                  title={`${formatTimecode(seg.start)} → ${formatTimecode(seg.end)}`}
                />
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
