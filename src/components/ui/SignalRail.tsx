import { Fragment } from "react"
import { motion } from "motion/react"
import { cn } from "../../lib/cn"

export interface RailStep {
  key: string
  label: string
}

export function SignalRail({
  steps,
  currentIndex,
}: {
  steps: RailStep[]
  /** -1 = nothing started, steps.length = all done */
  currentIndex: number
}) {
  return (
    <div className="flex items-start">
      {steps.map((step, i) => {
        const isDone = i < currentIndex
        const isActive = i === currentIndex

        return (
          <Fragment key={step.key}>
            <div className="flex shrink-0 flex-col items-center gap-2">
              <span
                className={cn(
                  "relative flex size-2.5 items-center justify-center rounded-full",
                  isDone || isActive ? "bg-signal" : "bg-hairline-strong"
                )}
              >
                {isActive && (
                  <span className="absolute inset-0 animate-ping rounded-full bg-signal" />
                )}
              </span>
              <span
                className={cn(
                  "font-mono text-[10px] uppercase tracking-[0.1em]",
                  isDone || isActive ? "text-ink" : "text-ink-mute"
                )}
              >
                {step.label}
              </span>
            </div>
            {i < steps.length - 1 && (
              <div className="relative mx-2 mt-[5px] h-px flex-1 overflow-hidden bg-hairline-strong">
                <motion.div
                  className="absolute inset-y-0 left-0 bg-signal"
                  initial={false}
                  animate={{ width: isDone ? "100%" : "0%" }}
                  transition={{ duration: 0.45, ease: "easeInOut" }}
                />
              </div>
            )}
          </Fragment>
        )
      })}
    </div>
  )
}
