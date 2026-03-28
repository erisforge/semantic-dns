import { AlertTriangle, ShieldCheck } from 'lucide-react'
import { Panel } from '../components/panel'
import { StatusBadge } from '../components/status-badge'
import {
  useFingerprintsQuery,
  useQuarantineQuery,
  useReconcileMutation,
  useSyncStatusQuery,
  useTemplatesQuery,
} from '../lib/api'
import { formatTimestamp } from '../lib/format'

export function OperationsPage() {
  const syncQuery = useSyncStatusQuery()
  const fingerprintsQuery = useFingerprintsQuery()
  const templatesQuery = useTemplatesQuery()
  const quarantineQuery = useQuarantineQuery()
  const reconcileMutation = useReconcileMutation()

  const sync = syncQuery.data
  const fingerprints = fingerprintsQuery.data ?? []
  const templates = templatesQuery.data ?? []
  const quarantine = quarantineQuery.data ?? []

  return (
    <div className="grid gap-5">
      <div className="grid gap-5 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
      <Panel
        title="DNS Synchronization"
        eyebrow="Runtime"
        detail="Reconciliation resets pending backlog after state settle."
          actions={
            <button
              className="rounded-xl bg-[var(--accent)] px-4 py-2 text-sm font-semibold text-[#18061d] transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60"
              disabled={reconcileMutation.isPending}
              onClick={() => reconcileMutation.mutate()}
              type="button"
            >
              {reconcileMutation.isPending ? 'Reconciling…' : 'Mark Reconciliation'}
            </button>
          }
        >
          <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-4">
            <Metric title="Records synced" value={sync?.dns_records_synced ?? '—'} tone="ok" />
            <Metric title="Observed leases" value={sync?.total_leases ?? '—'} tone="high" />
            <Metric title="Pending updates" value={sync?.pending_updates ?? '—'} tone="medium" />
            <Metric title="Failed updates" value={sync?.failed_updates ?? '—'} tone="critical" />
          </div>
          <div className="mt-3 rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.02)] px-4 py-2.5 text-sm text-[var(--text-muted)]">
            Last reconciliation: {formatTimestamp(sync?.last_reconciliation)}
          </div>
          {reconcileMutation.error ? (
            <div className="mt-3 text-sm text-[var(--critical)]">{reconcileMutation.error.message}</div>
          ) : null}
        </Panel>

        <Panel
        title="Quarantine Queue"
        eyebrow="Exception Flow"
        detail="Endpoints awaiting approval before assignment or publish."
        >
          {quarantine.length > 0 ? (
            <div className="space-y-3">
              {quarantine.map((entry) => (
                <div
                  key={entry.id}
                  className="rounded-lg border border-[rgba(255,94,125,0.2)] bg-[rgba(255,94,125,0.06)] px-4 py-3"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="mono text-xs text-[var(--text-strong)]">{entry.mac}</div>
                      <div className="mt-1 text-sm text-[var(--text)]">{entry.reason}</div>
                    </div>
                    <AlertTriangle className="h-4 w-4 text-[var(--critical)]" />
                  </div>
                  <div className="mt-2 text-xs text-[var(--text-muted)]">
                    Queued {formatTimestamp(entry.queued_at)}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="rounded-lg border border-[rgba(114,241,184,0.18)] bg-[rgba(114,241,184,0.05)] px-4 py-10 text-center text-sm text-[var(--ok)]">
              No quarantine entries are waiting for operator approval.
            </div>
          )}
        </Panel>
      </div>

      <div className="grid gap-5 xl:grid-cols-2">
        <Panel
        title="Fingerprint Rules"
        eyebrow="Classification"
        detail="DHCP fingerprint coverage for class and vendor inference."
        >
          <div className="space-y-3">
            {fingerprints.length > 0 ? (
              fingerprints.map((rule) => (
                <div
                  key={rule.id}
                  className="rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.015)] px-4 py-3"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="font-medium text-[var(--text-strong)]">{rule.name}</div>
                      <div className="mt-1 text-sm text-[var(--text-muted)]">
                        {rule.classification.vendor} / {rule.classification.class} /{' '}
                        {rule.classification.model_family}
                      </div>
                    </div>
                    <StatusBadge
                      label={`${Math.round(rule.classification.confidence * 100)}%`}
                      tone="high"
                    />
                  </div>
                  <div className="mono mt-2 text-xs text-[var(--text-muted)]">
                    OUI {rule.mac_oui ?? 'n/a'} · Option 60 {rule.option_60_glob ?? 'n/a'}
                  </div>
                </div>
              ))
            ) : (
              <EmptySurface message={fingerprintsQuery.isLoading ? 'Loading fingerprint rules…' : 'No fingerprint rules found.'} />
            )}
          </div>
        </Panel>

        <Panel
        title="Role Templates"
        eyebrow="Assignment"
        detail="Classification-to-address and work-context templates."
        >
          <div className="space-y-3">
            {templates.length > 0 ? (
              templates.map((template) => (
                <div
                  key={template.id}
                  className="rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.015)] px-4 py-3"
                >
                  <div className="flex items-start justify-between gap-3">
                    <div>
                      <div className="font-medium text-[var(--text-strong)]">{template.name}</div>
                      <div className="mt-1 text-sm text-[var(--text-muted)]">
                        {template.site_id ?? 'Any site'} / {template.area_id ?? 'Any area'} /{' '}
                        {template.work_center_id ?? template.cell_id}
                      </div>
                    </div>
                    <ShieldCheck className="h-4 w-4 text-[var(--accent)]" />
                  </div>
                  <div className="mt-3 grid grid-cols-3 gap-2 text-xs text-[var(--text-muted)]">
                    <div>{template.assignments.length} assignments</div>
                    <div>{template.unassigned_range.length} unassigned</div>
                    <div>{template.quarantine_range.length} quarantine</div>
                  </div>
                </div>
              ))
            ) : (
              <EmptySurface message={templatesQuery.isLoading ? 'Loading role templates…' : 'No role templates found.'} />
            )}
          </div>
        </Panel>
      </div>
    </div>
  )
}

function Metric({
  title,
  value,
  tone,
}: {
  title: string
  value: string | number
  tone: string
}) {
  return (
    <div className="rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.02)] px-4 py-3">
      <div className="text-[10px] uppercase tracking-[0.18em] text-[var(--text-muted)]">{title}</div>
      <div className="mt-2 flex items-center justify-between gap-2">
        <div className="mono text-lg font-semibold text-[var(--text-strong)]">{value}</div>
        <StatusBadge label={tone} />
      </div>
    </div>
  )
}

function EmptySurface({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-dashed border-[var(--border)]/80 px-4 py-10 text-center text-sm text-[var(--text-muted)]">
      {message}
    </div>
  )
}
