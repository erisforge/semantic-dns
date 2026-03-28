import { useEffect, useMemo, useState } from 'react'
import { useForm } from 'react-hook-form'
import { NavLink, Outlet, useLocation } from 'react-router-dom'
import {
  Activity,
  Database,
  GitMerge,
  Map,
  Network,
  ScrollText,
  Settings2,
} from 'lucide-react'
import { Panel } from './panel'
import { StatusBadge } from './status-badge'
import { type ApiSettings, useApiSettings, useHealthQuery } from '../lib/api'
import { formatRelative, formatTimestamp } from '../lib/format'

const navItems = [
  { to: '/records', label: 'Records', icon: Database },
  { to: '/graph', label: 'Graph', icon: Map },
  { to: '/operations', label: 'Operations', icon: GitMerge },
  { to: '/audit', label: 'Audit', icon: ScrollText },
]

const routeMeta: Record<string, { title: string; description: string }> = {
  '/records': {
    title: 'Semantic Record Explorer',
    description: 'Canonical assets, naming, provenance, and ISA-95 placement.',
  },
  '/graph': {
    title: 'Hierarchy Graph',
    description: 'Sites, areas, work centers, work units, and attached assets.',
  },
  '/operations': {
    title: 'Operations Surface',
    description: 'Synchronization, quarantine, fingerprinting, and role templates.',
  },
  '/audit': {
    title: 'Audit Ledger',
    description: 'Immutable event history for ingest, import, approval, and reconcile actions.',
  },
}

function Metric({
  label,
  value,
  tone,
}: {
  label: string
  value: string | number
  tone?: string
}) {
  return (
    <div className="rounded-[10px] border border-[var(--border)] bg-[rgba(255,255,255,0.02)] px-3 py-1.5">
      <div className="text-[9px] uppercase tracking-[0.18em] text-[var(--text-muted)]">
        {label}
      </div>
      <div className="mt-1 flex items-center gap-2">
        <div className="mono text-[15px] font-semibold text-[var(--text-strong)]">{value}</div>
        {tone ? <StatusBadge label={tone} /> : null}
      </div>
    </div>
  )
}

export function AppShell() {
  const location = useLocation()
  const { settings, updateSettings } = useApiSettings()
  const healthQuery = useHealthQuery()
  const [settingsOpen, setSettingsOpen] = useState(false)
  const {
    register,
    handleSubmit,
    reset,
    formState: { isDirty },
  } = useForm<ApiSettings>({ defaultValues: settings })

  useEffect(() => {
    reset(settings)
  }, [reset, settings])

  const meta = routeMeta[location.pathname] ?? routeMeta['/records']

  const connectionTone = healthQuery.isError
    ? 'offline'
    : healthQuery.data?.status === 'ok'
      ? 'ok'
      : 'degraded'

  const sync = healthQuery.data?.sync_status

  const systemMetrics = useMemo(
    () => [
      { label: 'DNS Records', value: sync?.dns_records_synced ?? '—' },
      { label: 'Leases', value: sync?.total_leases ?? '—' },
      { label: 'Pending', value: sync?.pending_updates ?? '—' },
      { label: 'Failed', value: sync?.failed_updates ?? '—' },
    ],
    [sync],
  )

  return (
    <div className="min-h-screen bg-transparent text-[var(--text)]">
      <div className="mx-auto grid min-h-screen max-w-[1700px] grid-cols-1 gap-px bg-[var(--border)] lg:grid-cols-[196px_minmax(0,1fr)_276px]">
        <aside className="bg-[color:var(--bg-rail)] px-3.5 py-4">
          <div className="border-b border-[var(--border)] pb-3.5">
            <div className="mono text-[10px] uppercase tracking-[0.32em] text-[var(--accent)]">
              Semantic DNS
            </div>
            <div className="mt-1.5 text-[18px] font-semibold text-[var(--text-strong)]">
              Operator Console
            </div>
            <p className="mt-1.5 text-[11px] leading-[1.125rem] text-[var(--text-muted)]">
              Semantic context, exposed as an operator surface.
            </p>
          </div>

          <nav className="mt-5 space-y-1">
            {navItems.map((item) => {
              const Icon = item.icon
              return (
                <NavLink
                  key={item.to}
                  to={item.to}
                  className={({ isActive }) =>
                    [
                      'flex items-center justify-between rounded-[10px] border px-3 py-2 transition-colors',
                      isActive
                        ? 'border-[rgba(255,106,193,0.28)] bg-[rgba(255,106,193,0.12)] text-[var(--text-strong)]'
                        : 'border-transparent bg-transparent text-[var(--text-muted)] hover:border-[var(--border)] hover:bg-[rgba(126,249,255,0.06)] hover:text-[var(--text)]',
                    ].join(' ')
                  }
                >
                  <span className="flex items-center gap-3">
                    <Icon className="h-4 w-4" />
                    <span className="font-medium">{item.label}</span>
                  </span>
                  <span className="mono text-[10px] uppercase tracking-[0.2em]">
                    {item.label.slice(0, 3)}
                  </span>
                </NavLink>
              )
            })}
          </nav>

          <div className="mt-4 border-t border-[var(--border)] pt-3.5">
            <div className="flex items-center justify-between">
              <div className="text-sm font-semibold text-[var(--text-strong)]">Connection</div>
              <StatusBadge label={connectionTone} />
            </div>
            <div className="mt-2.5 space-y-2 text-[11px] text-[var(--text-muted)]">
              <div className="flex items-center justify-between gap-3">
                <span>API origin</span>
                <span className="mono text-right text-xs text-[var(--text)]">
                  {settings.baseUrl || 'proxy /api'}
                </span>
              </div>
              <div className="flex items-center justify-between gap-3">
                <span>Token</span>
                <span className="mono text-right text-xs text-[var(--text)]">
                  {settings.token ? `${settings.token.slice(0, 6)}…` : 'missing'}
                </span>
              </div>
            </div>
            <button
              className="mt-3 flex w-full items-center justify-center gap-2 rounded-[10px] border border-[var(--border)] bg-[rgba(255,255,255,0.02)] px-3 py-2 text-[13px] font-medium text-[var(--text)] transition hover:border-[var(--border-strong)] hover:bg-[rgba(126,249,255,0.05)]"
              onClick={() => setSettingsOpen(true)}
              type="button"
            >
              <Settings2 className="h-4 w-4" />
              Configure API
            </button>
          </div>
        </aside>

        <main className="bg-[color:var(--bg)] px-3.5 py-3.5 lg:px-3.5 lg:py-3.5">
          <header className="surface-line rounded-[12px] border border-[var(--border)]/80 bg-[linear-gradient(180deg,rgba(15,13,22,0.96),rgba(9,7,15,0.99))] px-4 py-3.5">
            <div className="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
              <div className="max-w-3xl">
                <div className="mono text-[10px] uppercase tracking-[0.32em] text-[var(--accent)]">
                  Control Surface
                </div>
                <h1 className="mt-1 text-[1.8rem] font-semibold tracking-[0.01em] text-[var(--text-strong)]">
                  {meta.title}
                </h1>
                <p className="mt-1 max-w-2xl text-[11px] leading-[1.125rem] text-[var(--text-muted)]">
                  {meta.description}
                </p>
              </div>

              <div className="grid grid-cols-2 gap-2 xl:grid-cols-4">
                {systemMetrics.map((metric) => (
                  <Metric key={metric.label} label={metric.label} value={metric.value} />
                ))}
              </div>
            </div>
          </header>

          <div className="mt-3.5">
            <Outlet />
          </div>
        </main>

        <aside className="hidden bg-[color:var(--bg-rail)] px-3.5 py-4 lg:block">
          <Panel
            title="System Summary"
            eyebrow="Status Rail"
            detail="Health, freshness, and semantic coverage."
          >
            <div className="grid gap-3">
              <div className="flex items-center justify-between rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.02)] px-3.5 py-3">
                <div>
                  <div className="text-sm font-medium text-[var(--text-strong)]">API heartbeat</div>
                  <div className="text-xs text-[var(--text-muted)]">
                    {healthQuery.isError ? 'Connection requires attention' : 'Control plane responding'}
                  </div>
                </div>
                <Activity className="h-5 w-5 text-[var(--accent)]" />
              </div>
              <div className="flex items-center justify-between rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.02)] px-3.5 py-3">
                <div>
                  <div className="text-sm font-medium text-[var(--text-strong)]">Last reconciliation</div>
                  <div className="text-xs text-[var(--text-muted)]">
                    {formatTimestamp(sync?.last_reconciliation)}
                  </div>
                </div>
                <GitMerge className="h-5 w-5 text-[var(--accent)]" />
              </div>
              <div className="rounded-lg border border-[var(--border)]/80 bg-[rgba(255,255,255,0.02)] px-3.5 py-3">
                <div className="flex items-center justify-between">
                  <div className="text-sm font-medium text-[var(--text-strong)]">Data freshness</div>
                  <Network className="h-5 w-5 text-[var(--accent)]" />
                </div>
                <div className="mt-2 text-xs text-[var(--text-muted)]">
                  {sync?.last_reconciliation
                    ? `${formatRelative(sync.last_reconciliation)} from last successful settle`
                    : 'No reconciliation event has been recorded yet.'}
                </div>
              </div>
            </div>
          </Panel>

          <Panel
            title="Operator Notes"
            eyebrow="Bring-up"
            detail="Defaults to the local Rust API and admin token."
            className="mt-3.5"
          >
            <ul className="space-y-2.5 text-xs leading-5 text-[var(--text-muted)]">
              <li>Validate naming in `Records` before trusting placement.</li>
              <li>Use `Operations` to clear DHCP and quarantine drift.</li>
              <li>Use `Audit` to confirm import, ingest, and operator actions.</li>
            </ul>
          </Panel>
        </aside>
      </div>

      {settingsOpen ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-[rgba(5,8,13,0.8)] p-4 backdrop-blur-sm">
          <form
            className="w-full max-w-xl rounded-[24px] border border-[var(--border-strong)] bg-[color:var(--bg-panel)] shadow-[var(--shadow)]"
            onSubmit={handleSubmit((values) => {
              updateSettings({
                baseUrl: values.baseUrl.trim(),
                token: values.token.trim(),
              })
              setSettingsOpen(false)
            })}
          >
            <div className="border-b border-[var(--border)] px-6 py-5">
              <div className="mono text-[10px] uppercase tracking-[0.28em] text-[var(--accent)]">
                API Settings
              </div>
              <h2 className="mt-2 text-xl font-semibold text-[var(--text-strong)]">
                Configure frontend connectivity
              </h2>
              <p className="mt-2 text-sm text-[var(--text-muted)]">
                Leave the base URL empty to use the local Vite proxy. The default development token is
                <span className="mono ml-1 text-[var(--text)]">semantic-admin-token</span>.
              </p>
            </div>
            <div className="space-y-5 px-6 py-5">
              <label className="block">
                <span className="mb-2 block text-xs uppercase tracking-[0.2em] text-[var(--text-muted)]">
                  API Base URL
                </span>
                <input
                  {...register('baseUrl')}
                  className="mono w-full rounded-xl border border-[var(--border)] bg-[color:var(--bg)] px-4 py-3 text-sm text-[var(--text-strong)] outline-none transition focus:border-[var(--accent)]"
                  placeholder="http://127.0.0.1:8088"
                />
              </label>
              <label className="block">
                <span className="mb-2 block text-xs uppercase tracking-[0.2em] text-[var(--text-muted)]">
                  Bearer Token
                </span>
                <input
                  {...register('token')}
                  className="mono w-full rounded-xl border border-[var(--border)] bg-[color:var(--bg)] px-4 py-3 text-sm text-[var(--text-strong)] outline-none transition focus:border-[var(--accent)]"
                  placeholder="semantic-admin-token"
                />
              </label>
            </div>
            <div className="flex items-center justify-between border-t border-[var(--border)] px-6 py-4">
              <button
                className="rounded-xl border border-[var(--border)] px-4 py-2 text-sm text-[var(--text)] transition hover:border-[var(--border-strong)]"
                onClick={() => setSettingsOpen(false)}
                type="button"
              >
                Cancel
              </button>
              <button
                className="rounded-xl bg-[var(--accent)] px-4 py-2 text-sm font-semibold text-[#18061d] transition hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-60"
                disabled={!isDirty}
                type="submit"
              >
                Save Settings
              </button>
            </div>
          </form>
        </div>
      ) : null}
    </div>
  )
}
