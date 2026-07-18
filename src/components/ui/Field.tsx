import { type InputHTMLAttributes, type SelectHTMLAttributes, type ReactNode, forwardRef } from "react"
import { cn } from "../../lib/cn"

const controlBase =
  "w-full rounded-control border border-hairline-strong bg-canvas-raised px-3 py-2 text-sm text-ink placeholder:text-ink-mute transition-colors focus:border-signal focus:outline-none focus:ring-1 focus:ring-signal/40"

function FieldShell({
  label,
  hint,
  children,
}: {
  label?: string
  hint?: string
  children: ReactNode
}) {
  return (
    <div className="space-y-1.5">
      {label && (
        <label className="block font-mono text-[11px] uppercase tracking-[0.1em] text-ink-dim">
          {label}
        </label>
      )}
      {children}
      {hint && <p className="text-xs text-ink-mute">{hint}</p>}
    </div>
  )
}

interface TextFieldProps extends InputHTMLAttributes<HTMLInputElement> {
  label?: string
  hint?: string
}

export const TextField = forwardRef<HTMLInputElement, TextFieldProps>(
  ({ label, hint, className, ...props }, ref) => (
    <FieldShell label={label} hint={hint}>
      <input ref={ref} className={cn(controlBase, className)} {...props} />
    </FieldShell>
  )
)
TextField.displayName = "TextField"

interface SelectFieldProps extends SelectHTMLAttributes<HTMLSelectElement> {
  label?: string
  hint?: string
}

export const SelectField = forwardRef<HTMLSelectElement, SelectFieldProps>(
  ({ label, hint, className, children, ...props }, ref) => (
    <FieldShell label={label} hint={hint}>
      <select
        ref={ref}
        className={cn(controlBase, "appearance-none bg-[length:16px] bg-[right_0.75rem_center] bg-no-repeat cursor-pointer", className)}
        style={{
          backgroundImage:
            "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='%239A968C' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='6 9 12 15 18 9'%3E%3C/polyline%3E%3C/svg%3E\")",
        }}
        {...props}
      >
        {children}
      </select>
    </FieldShell>
  )
)
SelectField.displayName = "SelectField"
