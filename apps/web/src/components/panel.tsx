import clsx from 'clsx'
import type { ReactNode } from 'react'

export function Panel({
  title,
  eyebrow,
  detail,
  actions,
  children,
  className,
}: {
  title: string
  eyebrow?: string
  detail?: string
  actions?: ReactNode
  children: ReactNode
  className?: string
}) {
  return (
    <section
      className={clsx(
        'surface-line rounded-[var(--radius)] border border-[var(--border)]/80 bg-[color:var(--bg-panel)]',
        className,
      )}
    >
      <header className="flex items-start justify-between gap-3 border-b border-[var(--border)] px-3.5 py-2.5">
        <div>
          {eyebrow ? (
            <div className="mono text-[9px] uppercase tracking-[0.28em] text-[var(--text-muted)]">
              {eyebrow}
            </div>
          ) : null}
          <h2 className="mt-1 text-[15px] font-semibold tracking-[0.01em] text-[var(--text-strong)]">
            {title}
          </h2>
          {detail ? (
            <p className="mt-1 text-[11px] leading-[1.125rem] text-[var(--text-muted)]">{detail}</p>
          ) : null}
        </div>
        {actions}
      </header>
      <div className="p-3.5">{children}</div>
    </section>
  )
}
