import { Panel } from '../components/panel'
import { StatusBadge } from '../components/status-badge'
import { useAuditEventsQuery } from '../lib/api'
import { compactId, formatTimestamp } from '../lib/format'

export function AuditPage() {
  const auditQuery = useAuditEventsQuery()
  const events = auditQuery.data ?? []

  return (
    <Panel
      title="Audit Timeline"
      eyebrow="Ledger"
      detail="Recent signed control-plane events with hash-chain continuity."
      actions={<StatusBadge label={`${events.length} events`} tone="high" />}
    >
      <div className="space-y-2.5">
        {events.length > 0 ? (
          events.map((event) => (
            <article
              key={event.id}
              className="rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.015)] px-4 py-3"
            >
              <div className="flex flex-col gap-2 xl:flex-row xl:items-start xl:justify-between">
                <div>
                  <div className="flex items-center gap-2">
                    <div className="mono text-[10px] uppercase tracking-[0.22em] text-[var(--accent)]">
                      Event #{event.id}
                    </div>
                    <StatusBadge label={event.event_type} tone="high" />
                  </div>
                  <div className="mt-2 text-sm text-[var(--text-muted)]">
                    {formatTimestamp(event.created_at)}
                  </div>
                </div>
                <div className="grid gap-1 text-right text-xs text-[var(--text-muted)]">
                  <div className="mono">prev {compactId(event.previous_hash ?? 'root', 6)}</div>
                  <div className="mono text-[var(--text)]">curr {compactId(event.current_hash, 6)}</div>
                </div>
              </div>

              <pre className="mono mt-3 overflow-x-auto rounded-lg border border-[var(--border)]/80 bg-[color:var(--bg)] p-3 text-xs leading-5 text-[var(--text)]">
                {JSON.stringify(event.payload, null, 2)}
              </pre>
            </article>
          ))
        ) : (
          <div className="rounded-xl border border-dashed border-[var(--border)] px-4 py-12 text-center text-sm text-[var(--text-muted)]">
            {auditQuery.isLoading ? 'Loading audit events...' : auditQuery.isError ? auditQuery.error.message : 'No audit events recorded yet.'}
          </div>
        )}
      </div>
    </Panel>
  )
}
