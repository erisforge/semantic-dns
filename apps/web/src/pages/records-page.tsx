import { type ReactNode, useDeferredValue, useMemo, useState, useTransition } from 'react'
import type { ColumnDef } from '@tanstack/react-table'
import { Plus, Search, X } from 'lucide-react'
import { type DataTableColumnMeta, DataTable } from '../components/data-table'
import { HierarchyBuilder } from '../components/hierarchy-builder'
import { Panel } from '../components/panel'
import { StatusBadge } from '../components/status-badge'
import { useRecordsQuery } from '../lib/api'
import {
  compactId,
  effectiveArea,
  effectiveLeaf,
  effectiveSite,
  effectiveWorkCenter,
  effectiveWorkUnit,
  formatApplicationIdentityKind,
  formatHardwareIdentityKind,
  formatNodeKind,
  formatTimestamp,
  recordConfidence,
} from '../lib/format'
import type { SemanticRecord } from '../types'

const EMPTY_RECORDS: SemanticRecord[] = []

function SearchInput({
  value,
  onChange,
  pending,
}: {
  value: string
  onChange: (next: string) => void
  pending: boolean
}) {
  return (
    <label className="flex items-center gap-3 rounded-[10px] border border-[var(--border)] bg-[color:var(--bg)] px-3.5 py-2.5">
      <Search className="h-4 w-4 text-[var(--text-muted)]" />
      <input
        className="w-full bg-transparent text-[12px] text-[var(--text-strong)] outline-none placeholder:text-[var(--text-muted)]"
        onChange={(event) => onChange(event.target.value)}
        placeholder="Search FQDN, kind, class, vendor, site, or work center"
        value={value}
      />
      {pending ? <span className="mono text-[10px] uppercase text-[var(--accent)]">scan</span> : null}
    </label>
  )
}

function SelectFilter({
  label,
  value,
  options,
  formatOption,
  onChange,
}: {
  label: string
  value: string
  options: string[]
  formatOption?: (option: string) => string
  onChange: (next: string) => void
}) {
  return (
    <label className="flex min-w-[150px] items-center gap-2 rounded-[10px] border border-[var(--border)] bg-[color:var(--bg)] px-3 py-2">
      <span className="mono text-[9px] uppercase tracking-[0.18em] text-[var(--text-muted)]">
        {label}
      </span>
      <select
        className="w-full bg-transparent text-[11px] text-[var(--text-strong)] outline-none"
        onChange={(event) => onChange(event.target.value)}
        value={value}
      >
        <option value="all">All</option>
        {options.map((option) => (
          <option key={option} value={option}>
            {formatOption ? formatOption(option) : option}
          </option>
        ))}
      </select>
    </label>
  )
}

function StatusSegment({
  label,
  count,
  active,
  tone,
  onClick,
}: {
  label: string
  count: number
  active: boolean
  tone?: string
  onClick: () => void
}) {
  const activeTone =
    tone === 'ok'
      ? 'text-[var(--ok)]'
      : tone === 'warn'
        ? 'text-[var(--warn)]'
        : tone === 'critical'
          ? 'text-[var(--critical)]'
          : 'text-[var(--accent-cyan)]'

  return (
    <button
      className={[
        'relative inline-flex h-7 items-center gap-1 border-r border-[var(--border)] px-3 text-[9px] font-medium uppercase tracking-[0.16em] transition last:border-r-0',
        active
          ? `${activeTone} bg-[rgba(255,255,255,0.03)]`
          : 'text-[var(--text-dim)] hover:bg-[rgba(255,255,255,0.02)] hover:text-[var(--text)]',
      ].join(' ')}
      onClick={onClick}
      type="button"
    >
      {active ? <span className="absolute inset-x-0 top-0 h-px bg-current/80" /> : null}
      <span>{label}</span>
      <span className="mono text-[8px] text-current/65">{count}</span>
    </button>
  )
}

export function RecordsPage() {
  const recordsQuery = useRecordsQuery()
  const records = recordsQuery.data ?? EMPTY_RECORDS
  const [search, setSearch] = useState('')
  const [statusFilter, setStatusFilter] = useState('all')
  const [siteFilter, setSiteFilter] = useState('all')
  const [classFilter, setClassFilter] = useState('all')
  const [nodeKindFilter, setNodeKindFilter] = useState('all')
  const [selectedId, setSelectedId] = useState<string>()
  const [builderOpen, setBuilderOpen] = useState(false)
  const [builderContextRecord, setBuilderContextRecord] = useState<SemanticRecord>()
  const [isPending, startTransition] = useTransition()
  const deferredSearch = useDeferredValue(search)

  const statusCounts = useMemo(() => {
    return records.reduce<Record<string, number>>((accumulator, record) => {
      accumulator[record.status] = (accumulator[record.status] ?? 0) + 1
      return accumulator
    }, {})
  }, [records])

  const siteOptions = useMemo(
    () =>
      Array.from(new Set(records.map((record) => effectiveSite(record)).filter(Boolean))).sort((left, right) =>
        left.localeCompare(right),
      ),
    [records],
  )

  const classOptions = useMemo(
    () =>
      Array.from(new Set(records.map((record) => record.class ?? 'Unclassified'))).sort((left, right) =>
        left.localeCompare(right),
      ),
    [records],
  )

  const nodeKindOptions = useMemo(
    () =>
      Array.from(new Set(records.map((record) => record.node_kind)))
        .filter(Boolean)
        .sort((left, right) => left.localeCompare(right)),
    [records],
  )

  const columns = useMemo<ColumnDef<SemanticRecord>[]>(() => {
    const statusOptions = ['active', 'released', 'quarantined']
    return [
      {
        id: 'fqdn',
        header: 'Asset',
        accessorKey: 'fqdn',
        meta: {
          sticky: 'left',
          filterVariant: 'text',
          filterPlaceholder: 'Filter asset',
        } satisfies DataTableColumnMeta,
        cell: ({ row }) => (
          <div className="min-w-0">
            <div
              className="truncate text-[13px] font-medium text-[var(--text-strong)]"
              title={row.original.fqdn}
            >
              {row.original.fqdn}
            </div>
            <div className="mono mt-0.5 text-[11px] text-[var(--text-muted)]">
              {compactId(row.original.device_id)}
            </div>
          </div>
        ),
      },
      {
        id: 'node-kind',
        header: 'Kind',
        accessorKey: 'node_kind',
        meta: {
          filterVariant: 'select',
          filterOptions: nodeKindOptions,
        } satisfies DataTableColumnMeta,
        cell: ({ row }) => (
          <div className="space-y-1">
            <StatusBadge compact label={formatNodeKind(row.original.node_kind)} tone="high" />
            <div className="text-[11px] text-[var(--text-muted)]">
              {row.original.work_center_kind ?? 'hierarchy'}
            </div>
          </div>
        ),
      },
      {
        id: 'class',
        header: 'Class',
        accessorFn: (row) => row.class ?? 'Unclassified',
        meta: {
          filterVariant: 'select',
          filterOptions: classOptions,
        } satisfies DataTableColumnMeta,
        cell: ({ row }) => (
          <div>
            <div className="text-[13px]">{row.original.class ?? 'Unclassified'}</div>
            <div className="text-[11px] text-[var(--text-muted)]">{row.original.vendor ?? 'Unknown vendor'}</div>
          </div>
        ),
      },
      {
        id: 'path',
        header: 'ISA-95 Path',
        accessorFn: (row) => `${effectiveSite(row)} ${effectiveArea(row)} ${effectiveWorkCenter(row)}`,
        meta: {
          filterVariant: 'text',
          filterPlaceholder: 'Filter path',
        } satisfies DataTableColumnMeta,
        cell: ({ row }) => (
          <div className="min-w-0 text-[11px] leading-4 text-[var(--text)]">
            <div className="truncate">{effectiveSite(row.original)}</div>
            <div className="truncate text-[var(--text-muted)]">
              {effectiveArea(row.original)} / {effectiveWorkCenter(row.original)} / {effectiveWorkUnit(row.original)}
            </div>
          </div>
        ),
      },
      {
        id: 'addressing',
        header: 'Addressing',
        accessorFn: (row) => row.internal_ip ?? row.external_ip ?? '',
        meta: {
          filterVariant: 'text',
          filterPlaceholder: 'Filter IP',
        } satisfies DataTableColumnMeta,
        cell: ({ row }) => (
          <div className="mono text-[11px] leading-4">
            <div>{row.original.internal_ip ?? 'No internal IP'}</div>
            <div className="text-[var(--text-muted)]">{row.original.external_ip ?? 'No external IP'}</div>
          </div>
        ),
      },
      {
        id: 'status',
        header: 'Status',
        accessorKey: 'status',
        meta: {
          sticky: 'right',
          filterVariant: 'select',
          filterOptions: statusOptions,
        } satisfies DataTableColumnMeta,
        cell: ({ row }) => <StatusBadge compact label={row.original.status} />,
      },
    ]
  }, [classOptions, nodeKindOptions])

  const filteredRecords = useMemo(() => {
    const needle = deferredSearch.trim().toLowerCase()
    return records.filter((record) => {
      if (statusFilter !== 'all' && record.status !== statusFilter) {
        return false
      }
      if (siteFilter !== 'all' && effectiveSite(record) !== siteFilter) {
        return false
      }
      if (classFilter !== 'all' && (record.class ?? 'Unclassified') !== classFilter) {
        return false
      }
      if (nodeKindFilter !== 'all' && record.node_kind !== nodeKindFilter) {
        return false
      }
      if (!needle) {
        return true
      }

      return [
        record.fqdn,
        record.class,
        record.vendor,
        record.model,
        record.node_kind,
        effectiveSite(record),
        effectiveArea(record),
        effectiveWorkCenter(record),
        effectiveWorkUnit(record),
        effectiveLeaf(record),
        record.internal_ip,
        record.external_ip,
        ...record.hardware_identities.map((identity) => identity.value),
        ...record.application_identities.map((identity) => identity.value),
        ...record.aliases,
      ]
        .filter(Boolean)
        .some((value) => value?.toLowerCase().includes(needle))
    })
  }, [classFilter, deferredSearch, nodeKindFilter, records, siteFilter, statusFilter])

  const activeFilterCount =
    Number(statusFilter !== 'all') +
    Number(siteFilter !== 'all') +
    Number(classFilter !== 'all') +
    Number(nodeKindFilter !== 'all') +
    Number(Boolean(search.trim()))

  const activeSelectedId =
    selectedId && filteredRecords.some((record) => record.device_id === selectedId)
      ? selectedId
      : filteredRecords[0]?.device_id

  const selectedRecord = filteredRecords.find((record) => record.device_id === activeSelectedId)

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1.82fr)_340px]">
      <Panel
        title="Canonical Records"
        eyebrow="Explorer"
        detail="Semantic record index with naming, placement, and provenance."
        actions={
          <div className="flex items-center gap-2">
            <button
              className="inline-flex items-center gap-1 rounded-[10px] border border-[rgba(255,106,193,0.22)] bg-[rgba(255,106,193,0.09)] px-2.5 py-1.5 text-[10px] font-semibold uppercase tracking-[0.14em] text-[var(--text-strong)] transition hover:border-[var(--border-strong)] hover:bg-[rgba(255,106,193,0.16)]"
              onClick={() => {
                setBuilderContextRecord(selectedRecord)
                setBuilderOpen(true)
              }}
              type="button"
            >
              <Plus className="h-3.5 w-3.5" />
              Create record
            </button>
            <StatusBadge label={`${filteredRecords.length} visible`} tone="high" />
            <StatusBadge label={`${records?.length ?? 0} total`} tone="ok" />
          </div>
        }
      >
        <div className="space-y-3">
          <div className="grid gap-2 xl:grid-cols-[minmax(0,1fr)_auto_auto_auto]">
            <SearchInput onChange={(next) => startTransition(() => setSearch(next))} pending={isPending} value={search} />
            <SelectFilter label="Site" onChange={setSiteFilter} options={siteOptions} value={siteFilter} />
            <SelectFilter label="Class" onChange={setClassFilter} options={classOptions} value={classFilter} />
            <SelectFilter
              formatOption={formatNodeKind}
              label="Kind"
              onChange={setNodeKindFilter}
              options={nodeKindOptions}
              value={nodeKindFilter}
            />
          </div>

          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="inline-flex overflow-hidden rounded-[10px] border border-[var(--border)] bg-[rgba(255,255,255,0.015)]">
              <StatusSegment active={statusFilter === 'all'} count={records.length} label="All" onClick={() => setStatusFilter('all')} />
              <StatusSegment active={statusFilter === 'active'} count={statusCounts.active ?? 0} label="Active" onClick={() => setStatusFilter('active')} tone="ok" />
              <StatusSegment active={statusFilter === 'released'} count={statusCounts.released ?? 0} label="Released" onClick={() => setStatusFilter('released')} tone="warn" />
              <StatusSegment active={statusFilter === 'quarantined'} count={statusCounts.quarantined ?? 0} label="Quarantined" onClick={() => setStatusFilter('quarantined')} tone="critical" />
            </div>
            {activeFilterCount > 0 ? (
              <button
                className="inline-flex items-center gap-1 rounded-[10px] border border-[var(--border)] px-2.5 py-1 text-[9px] uppercase tracking-[0.14em] text-[var(--text-muted)] transition hover:bg-[var(--interactive-hover)] hover:text-[var(--text)]"
                onClick={() => {
                  setSearch('')
                  setStatusFilter('all')
                  setSiteFilter('all')
                  setClassFilter('all')
                  setNodeKindFilter('all')
                }}
                type="button"
              >
                <X className="h-3 w-3" />
                Clear filters
              </button>
            ) : null}
          </div>

          <DataTable
            columns={columns}
            data={filteredRecords}
            emptyState={
              recordsQuery.isLoading
                ? 'Loading semantic records...'
                : recordsQuery.isError
                  ? recordsQuery.error.message
                  : 'No records match the current search.'
            }
            getRowId={(record) => record.device_id}
            onRowClick={(record) => setSelectedId(record.device_id)}
            selectedRowId={activeSelectedId}
          />
        </div>
      </Panel>

      <Panel
        title={selectedRecord ? selectedRecord.fqdn : 'Record Inspector'}
        eyebrow="Inspector"
        detail={
          selectedRecord
            ? 'Placement, identities, and provenance for the selected record.'
            : 'Select a record to inspect node kind, hardware anchors, and UNI/URN data.'
        }
      >
        {selectedRecord ? (
          <div className="space-y-3">
            <div className="grid grid-cols-2 gap-2">
              <Metric label="Kind" value={<StatusBadge compact label={formatNodeKind(selectedRecord.node_kind)} tone="high" />} />
              <Metric label="Status" value={<StatusBadge compact label={selectedRecord.status} />} />
              <Metric
                label="Confidence"
                value={<StatusBadge compact label={recordConfidence(selectedRecord)} />}
              />
              <Metric label="Class" value={selectedRecord.class ?? 'Unclassified'} />
              <Metric label="Model" value={selectedRecord.model ?? 'Unknown'} />
            </div>

            <div className="grid gap-2 rounded-[10px] border border-[var(--border)]/80 bg-[color:var(--bg)] p-3 text-[13px]">
              <InspectorRow label="FQDN" value={<span className="mono break-all">{selectedRecord.fqdn}</span>} />
              <InspectorRow label="Device ID" value={<span className="mono">{selectedRecord.device_id}</span>} />
              <InspectorRow label="MAC" value={<span className="mono">{selectedRecord.mac ?? 'Unavailable'}</span>} />
              <InspectorRow
                label="Internal / External"
                value={
                  <div className="mono space-y-1">
                    <div>{selectedRecord.internal_ip ?? 'Unavailable'}</div>
                    <div className="text-[var(--text-muted)]">
                      {selectedRecord.external_ip ?? 'Unavailable'}
                    </div>
                  </div>
                }
              />
              <InspectorRow
                label="ISA-95 Path"
                value={`${effectiveSite(selectedRecord)} / ${effectiveArea(selectedRecord)} / ${effectiveWorkCenter(selectedRecord)} / ${effectiveWorkUnit(selectedRecord)}`}
              />
              <InspectorRow label="Leaf Function" value={effectiveLeaf(selectedRecord)} />
              <InspectorRow
                label="Updated"
                value={`${formatTimestamp(selectedRecord.updated_at)} (${Object.keys(selectedRecord.field_sources).length} sourced fields)`}
              />
            </div>

            <IdentitySection
              detail="MAC, serials, DHCP client IDs, or future certificate anchors."
              emptyState="No hardware identities have been attached to this record."
              title="Hardware Identities"
            >
              {selectedRecord.hardware_identities.map((identity) => (
                <IdentityCard
                  key={`${identity.kind}-${identity.value}`}
                  label={formatHardwareIdentityKind(identity.kind)}
                  value={identity.value}
                  meta={identity.label ?? 'semantic-record'}
                />
              ))}
            </IdentitySection>

            <IdentitySection
              detail="Stable application-level IDs for MES, CMMS, historians, and other systems."
              emptyState="No URNs or UNIs have been attached to this record."
              title="Application Identities"
            >
              {selectedRecord.application_identities.map((identity) => (
                <IdentityCard
                  key={`${identity.kind}-${identity.value}`}
                  label={formatApplicationIdentityKind(identity.kind)}
                  value={identity.value}
                  meta={identity.label ?? 'semantic-record'}
                />
              ))}
            </IdentitySection>

            <IdentitySection
              detail="Operator-friendly and legacy lookup handles."
              emptyState="No aliases registered."
              title="Aliases"
            >
              {selectedRecord.aliases.map((alias) => (
                <IdentityCard key={alias} label="Alias" value={alias} meta="lookup" />
              ))}
            </IdentitySection>

            <IdentitySection
              detail="Context links that live beside DNS, not inside the hostname."
              emptyState="No semantic relations recorded."
              title="Relations"
            >
              {selectedRecord.relations.map((relation) => (
                <IdentityCard
                  key={`${relation.relation}-${relation.target}`}
                  label={relation.relation}
                  value={relation.target}
                  meta={relation.label ?? 'graph edge'}
                />
              ))}
            </IdentitySection>

            <div>
              <div className="mb-2 text-[10px] uppercase tracking-[0.2em] text-[var(--text-muted)]">
                Field Provenance
              </div>
              <div className="space-y-1.5">
                {Object.entries(selectedRecord.field_sources).map(([field, source]) => (
                  <div
                    key={field}
                    className="rounded-[10px] border border-[var(--border)]/80 bg-[rgba(255,255,255,0.015)] px-3 py-2"
                  >
                    <div className="flex items-center justify-between gap-3">
                      <div className="font-medium text-[var(--text-strong)]">{field}</div>
                      <StatusBadge compact label={source.source} tone={source.source} />
                    </div>
                    <div className="mono mt-1 text-[11px] text-[var(--text)]">{source.value}</div>
                    <div className="mt-0.5 text-[10px] text-[var(--text-muted)]">
                      {formatTimestamp(source.updated_at)}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : (
          <div className="rounded-xl border border-dashed border-[var(--border)] px-4 py-12 text-center text-sm text-[var(--text-muted)]">
            {recordsQuery.isLoading
              ? 'Loading records...'
              : 'Pick a row from the explorer to open the record inspector.'}
          </div>
        )}
      </Panel>

      {builderOpen ? (
        <HierarchyBuilder
          key={builderContextRecord?.device_id ?? selectedRecord?.device_id ?? 'hierarchy-builder'}
          onClose={() => setBuilderOpen(false)}
          onCreated={(record) => {
            setSelectedId(record.device_id)
            setBuilderContextRecord(record)
          }}
          records={records}
          selectedRecord={builderContextRecord ?? selectedRecord}
        />
      ) : null}
    </div>
  )
}

function Metric({
  label,
  value,
}: {
  label: string
  value: ReactNode
}) {
  return (
    <div className="rounded-[10px] border border-[var(--border)]/80 bg-[rgba(255,255,255,0.02)] px-3 py-2.5">
      <div className="text-[9px] uppercase tracking-[0.18em] text-[var(--text-muted)]">{label}</div>
      <div className="mt-1 text-[13px] font-medium text-[var(--text-strong)]">{value}</div>
    </div>
  )
}

function InspectorRow({
  label,
  value,
}: {
  label: string
  value: ReactNode
}) {
  return (
    <div className="grid grid-cols-[104px_minmax(0,1fr)] gap-3">
      <div className="text-[9px] uppercase tracking-[0.16em] text-[var(--text-muted)]">{label}</div>
      <div className="text-[var(--text-strong)]">{value}</div>
    </div>
  )
}

function IdentitySection({
  title,
  detail,
  emptyState,
  children,
}: {
  title: string
  detail: string
  emptyState: string
  children: ReactNode
}) {
  const items = Array.isArray(children) ? children.filter(Boolean) : children ? [children] : []

  return (
    <div>
      <div className="mb-2">
        <div className="text-[10px] uppercase tracking-[0.2em] text-[var(--text-muted)]">{title}</div>
        <div className="mt-1 text-[11px] text-[var(--text-dim)]">{detail}</div>
      </div>
      {items.length > 0 ? (
        <div className="space-y-1.5">{items}</div>
      ) : (
        <div className="rounded-[10px] border border-dashed border-[var(--border)] px-3 py-3 text-[11px] text-[var(--text-dim)]">
          {emptyState}
        </div>
      )}
    </div>
  )
}

function IdentityCard({
  label,
  value,
  meta,
}: {
  label: string
  value: string
  meta?: string
}) {
  return (
    <div className="rounded-[10px] border border-[var(--border)]/80 bg-[rgba(255,255,255,0.015)] px-3 py-2">
      <div className="flex items-center justify-between gap-3">
        <div className="text-[10px] uppercase tracking-[0.18em] text-[var(--text-muted)]">{label}</div>
        {meta ? <div className="text-[10px] text-[var(--text-dim)]">{meta}</div> : null}
      </div>
      <div className="mono mt-1 break-all text-[12px] text-[var(--text-strong)]">{value}</div>
    </div>
  )
}
