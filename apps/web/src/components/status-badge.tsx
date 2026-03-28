import clsx from 'clsx'

const toneMap: Record<string, string> = {
  active: 'border-[rgba(114,241,184,0.24)] bg-[rgba(114,241,184,0.1)] text-[var(--ok)]',
  released: 'border-[rgba(255,216,102,0.24)] bg-[rgba(255,216,102,0.1)] text-[var(--warn)]',
  expired: 'border-[rgba(255,216,102,0.24)] bg-[rgba(255,216,102,0.1)] text-[var(--warn)]',
  quarantined: 'border-[rgba(255,94,125,0.24)] bg-[rgba(255,94,125,0.1)] text-[var(--critical)]',
  high: 'border-[rgba(126,249,255,0.24)] bg-[rgba(126,249,255,0.1)] text-[var(--accent-cyan)]',
  medium: 'border-[rgba(126,249,255,0.18)] bg-[rgba(126,249,255,0.08)] text-[var(--accent-cyan)]',
  low: 'border-[rgba(107,90,133,0.34)] bg-[rgba(107,90,133,0.12)] text-[var(--offline)]',
  authoritative: 'border-[rgba(114,241,184,0.24)] bg-[rgba(114,241,184,0.1)] text-[var(--ok)]',
  ok: 'border-[rgba(114,241,184,0.24)] bg-[rgba(114,241,184,0.1)] text-[var(--ok)]',
  degraded: 'border-[rgba(255,216,102,0.24)] bg-[rgba(255,216,102,0.1)] text-[var(--warn)]',
  offline: 'border-[rgba(107,90,133,0.34)] bg-[rgba(107,90,133,0.12)] text-[var(--offline)]',
}

export function StatusBadge({
  label,
  tone,
  compact = false,
}: {
  label: string
  tone?: string
  compact?: boolean
}) {
  const normalized = (tone ?? label).toLowerCase()
  return (
    <span
      className={clsx(
        compact
          ? 'inline-flex items-center gap-1.5 rounded-full border px-2 py-0.5 text-[9px] font-semibold uppercase tracking-[0.16em]'
          : 'inline-flex items-center gap-2 rounded-full border px-2.5 py-1 text-[10px] font-semibold uppercase tracking-[0.18em]',
        toneMap[normalized] ??
          'border-[var(--border)] bg-[rgba(107,90,133,0.12)] text-[var(--text)]',
      )}
    >
      <span className={clsx('rounded-full bg-current', compact ? 'h-1 w-1' : 'h-1.5 w-1.5')} />
      {label}
    </span>
  )
}
