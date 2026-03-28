import type { ComponentType, FormEvent, InputHTMLAttributes, ReactNode, SelectHTMLAttributes, TextareaHTMLAttributes } from 'react'
import { useEffect, useMemo, useState } from 'react'
import clsx from 'clsx'
import {
  AlertTriangle,
  ArrowRight,
  Check,
  Layers3,
  Link2,
  Network,
  Plus,
  ShieldCheck,
  X,
} from 'lucide-react'
import { type CreateObservationInput, useCreateObservationMutation } from '../lib/api'
import {
  effectiveArea,
  effectiveSite,
  effectiveWorkCenter,
  effectiveWorkUnit,
  formatNodeKind,
} from '../lib/format'
import type { SemanticRecord } from '../types'

type BuilderNodeKind = 'site' | 'area' | 'work-center' | 'work-unit' | 'device'

type DraftHardwareIdentity = {
  id: string
  kind: string
  value: string
  label: string
}

type BuilderDraft = {
  nodeKind: BuilderNodeKind
  site: string
  area: string
  workCenter: string
  workCenterKind: string
  workUnit: string
  leaf: string
  enterprise: string
  className: string
  vendor: string
  model: string
  internalIp: string
  externalIp: string
  switchPort: string
  protocols: string
  mac: string
  hardwareRows: DraftHardwareIdentity[]
  urns: string
  unis: string
  aliases: string
}

type HierarchyBuilderProps = {
  records: SemanticRecord[]
  selectedRecord?: SemanticRecord
  onClose: () => void
  onCreated: (record: SemanticRecord) => void
}

type HierarchyStatus = {
  kind: BuilderNodeKind
  label: string
  value: string
  exists: boolean
  required: boolean
}

const nodeKindOrder: BuilderNodeKind[] = ['site', 'area', 'work-center', 'work-unit', 'device']

const workCenterKindOptions = [
  'process-cell',
  'unit',
  'production-line',
  'work-cell',
  'production-unit',
  'storage-zone',
  'storage-unit',
  'work-center',
]

const hardwareKindOptions = [
  'serial-number',
  'dhcp-client-id',
  'x509-subject',
  'x509-san-uri',
  'x509-spki-sha256',
]

const nodeLabelCopy: Record<BuilderNodeKind, string> = {
  site: 'Site name',
  area: 'Area name',
  'work-center': 'Work center name',
  'work-unit': 'Work unit name',
  device: 'Device DNS label',
}

function humanizeKebab(value: string): string {
  return value
    .split('-')
    .map((segment) => segment.charAt(0).toUpperCase() + segment.slice(1))
    .join(' ')
}

function nextNodeKind(kind: BuilderNodeKind): BuilderNodeKind {
  const index = nodeKindOrder.indexOf(kind)
  return nodeKindOrder[Math.min(index + 1, nodeKindOrder.length - 1)]
}

function makeHardwareRow(kind: string): DraftHardwareIdentity {
  return {
    id: crypto.randomUUID(),
    kind,
    value: '',
    label: '',
  }
}

function defaultHardwareRowsForKind(kind: BuilderNodeKind): DraftHardwareIdentity[] {
  return kind === 'device' ? [] : [makeHardwareRow('serial-number')]
}

function trimDraftForKind(draft: BuilderDraft, kind: BuilderNodeKind): BuilderDraft {
  const nextDraft: BuilderDraft = {
    ...draft,
    nodeKind: kind,
  }

  if (kind === 'site') {
    nextDraft.area = ''
    nextDraft.workCenter = ''
    nextDraft.workCenterKind = ''
    nextDraft.workUnit = ''
    nextDraft.leaf = ''
  } else if (kind === 'area') {
    nextDraft.workCenter = ''
    nextDraft.workCenterKind = ''
    nextDraft.workUnit = ''
    nextDraft.leaf = ''
  } else if (kind === 'work-center') {
    nextDraft.workUnit = ''
    nextDraft.leaf = ''
  } else if (kind === 'work-unit') {
    nextDraft.leaf = ''
  }

  if (!nextDraft.mac.trim() && nextDraft.hardwareRows.every((row) => !row.value.trim())) {
    nextDraft.hardwareRows = defaultHardwareRowsForKind(kind)
  }

  return nextDraft
}

function draftFromSelectedRecord(selectedRecord?: SemanticRecord): BuilderDraft {
  const source = selectedRecord
  const sourceKind = source?.node_kind as BuilderNodeKind | undefined
  const inferredNodeKind = sourceKind ? nextNodeKind(sourceKind) : 'site'
  const draft: BuilderDraft = {
    nodeKind: inferredNodeKind,
    site: source ? effectiveSite(source) : '',
    area: source ? effectiveArea(source) : '',
    workCenter: source ? effectiveWorkCenter(source) : '',
    workCenterKind: source?.work_center_kind ?? '',
    workUnit: source ? effectiveWorkUnit(source) : '',
    leaf: '',
    enterprise: source?.enterprise ?? '',
    className: '',
    vendor: '',
    model: '',
    internalIp: '',
    externalIp: '',
    switchPort: '',
    protocols: '',
    mac: '',
    hardwareRows: defaultHardwareRowsForKind(inferredNodeKind),
    urns: '',
    unis: '',
    aliases: '',
  }

  if (sourceKind === 'device') {
    draft.nodeKind = 'device'
  }

  return trimDraftForKind(draft, draft.nodeKind)
}

function buildPreviewFqdn(draft: BuilderDraft): string {
  const segments =
    draft.nodeKind === 'site'
      ? [draft.site]
      : draft.nodeKind === 'area'
        ? [draft.area, draft.site]
        : draft.nodeKind === 'work-center'
          ? [draft.workCenter, draft.area, draft.site]
          : draft.nodeKind === 'work-unit'
            ? [draft.workUnit, draft.workCenter, draft.area, draft.site]
            : [draft.leaf, draft.workUnit, draft.workCenter, draft.area, draft.site]

  const cleanSegments = segments.map((value) => value.trim())
  return cleanSegments.every(Boolean) ? `${cleanSegments.join('.')}.local` : 'Incomplete hierarchy'
}

function parseList(value: string): string[] {
  return value
    .split(/\r?\n|,/)
    .map((item) => item.trim())
    .filter(Boolean)
}

function matchHierarchy(record: SemanticRecord, kind: BuilderNodeKind, draft: BuilderDraft): boolean {
  if (record.node_kind !== kind) {
    return false
  }

  if (kind === 'site') {
    return effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase()
  }
  if (kind === 'area') {
    return (
      effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase() &&
      effectiveArea(record).toLowerCase() === draft.area.trim().toLowerCase()
    )
  }
  if (kind === 'work-center') {
    return (
      effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase() &&
      effectiveArea(record).toLowerCase() === draft.area.trim().toLowerCase() &&
      effectiveWorkCenter(record).toLowerCase() === draft.workCenter.trim().toLowerCase()
    )
  }
  if (kind === 'work-unit') {
    return (
      effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase() &&
      effectiveArea(record).toLowerCase() === draft.area.trim().toLowerCase() &&
      effectiveWorkCenter(record).toLowerCase() === draft.workCenter.trim().toLowerCase() &&
      effectiveWorkUnit(record).toLowerCase() === draft.workUnit.trim().toLowerCase()
    )
  }

  return (
    effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase() &&
    effectiveArea(record).toLowerCase() === draft.area.trim().toLowerCase() &&
    effectiveWorkCenter(record).toLowerCase() === draft.workCenter.trim().toLowerCase() &&
    effectiveWorkUnit(record).toLowerCase() === draft.workUnit.trim().toLowerCase() &&
    record.fqdn.toLowerCase() === buildPreviewFqdn(draft).toLowerCase()
  )
}

function buildHierarchyStatus(records: SemanticRecord[], draft: BuilderDraft): HierarchyStatus[] {
  return nodeKindOrder
    .filter((kind) => nodeKindOrder.indexOf(kind) <= nodeKindOrder.indexOf(draft.nodeKind))
    .map((kind) => ({
      kind,
      label: formatNodeKind(kind),
      value:
        kind === 'site'
          ? draft.site.trim()
          : kind === 'area'
            ? draft.area.trim()
            : kind === 'work-center'
              ? draft.workCenter.trim()
              : kind === 'work-unit'
                ? draft.workUnit.trim()
                : draft.leaf.trim(),
      exists: records.some((record) => matchHierarchy(record, kind, draft)),
      required: kind !== draft.nodeKind,
    }))
}

function buildObservationInput(draft: BuilderDraft): CreateObservationInput {
  const hardwareIdentities = [
    ...(
      draft.mac.trim()
        ? [
            {
              kind: 'mac-address',
              value: draft.mac.trim(),
              label: 'operator',
            },
          ]
        : []
    ),
    ...draft.hardwareRows
      .map((row) => ({
        kind: row.kind,
        value: row.value.trim(),
        label: row.label.trim() || undefined,
      }))
      .filter((row) => row.value),
  ]

  const applicationIdentities = [
    ...parseList(draft.urns).map((value) => ({
      kind: 'urn',
      value,
      label: null,
    })),
    ...parseList(draft.unis).map((value) => ({
      kind: 'uni',
      value,
      label: null,
    })),
  ]

  const site = draft.site.trim() || undefined
  const area = draft.area.trim() || undefined
  const workCenter = draft.workCenter.trim() || undefined
  const workUnit = draft.workUnit.trim() || undefined

  return {
    id: crypto.randomUUID(),
    device_id: crypto.randomUUID(),
    observed_at: new Date().toISOString(),
    source: 'manual-api',
    node_kind: draft.nodeKind,
    external_ip: draft.externalIp.trim() || null,
    internal_ip: draft.internalIp.trim() || null,
    class: draft.className.trim() || null,
    vendor: draft.vendor.trim() || null,
    model: draft.model.trim() || null,
    protocols: parseList(draft.protocols),
    mac: draft.mac.trim() || null,
    switch_port: draft.switchPort.trim() || null,
    enterprise: draft.enterprise.trim() || null,
    site: site ?? null,
    area: area ?? null,
    work_center: workCenter ?? null,
    work_center_kind: draft.workCenterKind.trim() || null,
    work_unit: workUnit ?? null,
    facility: site ?? null,
    zone: area ?? null,
    cell: workCenter ?? null,
    process: workUnit ?? null,
    function: draft.nodeKind === 'device' ? draft.leaf.trim() || null : null,
    hardware_identities: hardwareIdentities.length > 0 ? hardwareIdentities : null,
    application_identities: applicationIdentities.length > 0 ? applicationIdentities : null,
    aliases: parseList(draft.aliases),
    relations: null,
    status: 'active',
  }
}

function advanceDraftAfterCreate(record: SemanticRecord, draft: BuilderDraft): BuilderDraft {
  const kind = record.node_kind as BuilderNodeKind
  if (kind === 'device') {
    return {
      ...draft,
      leaf: '',
      className: '',
      vendor: '',
      model: '',
      internalIp: '',
      externalIp: '',
      switchPort: '',
      protocols: '',
      mac: '',
      hardwareRows: defaultHardwareRowsForKind('device'),
      urns: '',
      unis: '',
      aliases: '',
    }
  }

  return trimDraftForKind(
    {
      ...draft,
      nodeKind: nextNodeKind(kind),
      site: effectiveSite(record),
      area: effectiveArea(record),
      workCenter: effectiveWorkCenter(record),
      workCenterKind: record.work_center_kind ?? draft.workCenterKind,
      workUnit: effectiveWorkUnit(record),
      leaf: '',
      className: '',
      vendor: '',
      model: '',
      internalIp: '',
      externalIp: '',
      switchPort: '',
      protocols: '',
      mac: '',
      hardwareRows: defaultHardwareRowsForKind(nextNodeKind(kind)),
      urns: '',
      unis: '',
      aliases: '',
    },
    nextNodeKind(kind),
  )
}

function BuilderSection({
  title,
  detail,
  icon,
  children,
}: {
  title: string
  detail: string
  icon: ComponentType<{ className?: string }>
  children: ReactNode
}) {
  const Icon = icon
  return (
    <section className="rounded-[10px] border border-[var(--border)] bg-[color:var(--bg-panel)]">
      <div className="flex items-start gap-3 border-b border-[var(--border)] px-3.5 py-2.5">
        <div className="flex h-8 w-8 items-center justify-center rounded-[8px] border border-[var(--border)] bg-[rgba(255,255,255,0.03)]">
          <Icon className="h-4 w-4 text-[var(--accent-cyan)]" />
        </div>
        <div>
          <div className="text-[12px] font-semibold text-[var(--text-strong)]">{title}</div>
          <div className="mt-0.5 text-[11px] leading-[1.1rem] text-[var(--text-muted)]">{detail}</div>
        </div>
      </div>
      <div className="p-3.5">{children}</div>
    </section>
  )
}

function Field({
  label,
  detail,
  children,
}: {
  label: string
  detail?: string
  children: ReactNode
}) {
  return (
    <label className="block space-y-1.5">
      <div>
        <div className="text-[10px] uppercase tracking-[0.18em] text-[var(--text-muted)]">{label}</div>
        {detail ? <div className="mt-0.5 text-[11px] text-[var(--text-dim)]">{detail}</div> : null}
      </div>
      {children}
    </label>
  )
}

function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={clsx(
        'w-full rounded-[9px] border border-[var(--border)] bg-[color:var(--bg)] px-3 py-2 text-[12px] text-[var(--text-strong)] outline-none transition placeholder:text-[var(--text-dim)] focus:border-[var(--border-strong)] focus:bg-[rgba(255,255,255,0.02)]',
        props.className,
      )}
    />
  )
}

function Select(props: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select
      {...props}
      className={clsx(
        'w-full rounded-[9px] border border-[var(--border)] bg-[color:var(--bg)] px-3 py-2 text-[12px] text-[var(--text-strong)] outline-none transition focus:border-[var(--border-strong)] focus:bg-[rgba(255,255,255,0.02)]',
        props.className,
      )}
    />
  )
}

function Textarea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return (
    <textarea
      {...props}
      className={clsx(
        'min-h-[84px] w-full rounded-[9px] border border-[var(--border)] bg-[color:var(--bg)] px-3 py-2 text-[12px] text-[var(--text-strong)] outline-none transition placeholder:text-[var(--text-dim)] focus:border-[var(--border-strong)] focus:bg-[rgba(255,255,255,0.02)]',
        props.className,
      )}
    />
  )
}

export function HierarchyBuilder({
  records,
  selectedRecord,
  onClose,
  onCreated,
}: HierarchyBuilderProps) {
  const createObservation = useCreateObservationMutation()
  const [draft, setDraft] = useState<BuilderDraft>(() => draftFromSelectedRecord(selectedRecord))
  const [successMessage, setSuccessMessage] = useState<string>()

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === 'Escape') {
        onClose()
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [onClose])

  const siteOptions = useMemo(
    () =>
      Array.from(new Set(records.filter((record) => record.node_kind === 'site').map((record) => effectiveSite(record))))
        .filter(Boolean)
        .sort((left, right) => left.localeCompare(right)),
    [records],
  )

  const areaOptions = useMemo(
    () =>
      Array.from(
        new Set(
          records
            .filter(
              (record) =>
                record.node_kind === 'area' &&
                (!draft.site.trim() || effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase()),
            )
            .map((record) => effectiveArea(record)),
        ),
      )
        .filter(Boolean)
        .sort((left, right) => left.localeCompare(right)),
    [draft.site, records],
  )

  const workCenterOptions = useMemo(
    () =>
      Array.from(
        new Set(
          records
            .filter(
              (record) =>
                record.node_kind === 'work-center' &&
                (!draft.site.trim() || effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase()) &&
                (!draft.area.trim() || effectiveArea(record).toLowerCase() === draft.area.trim().toLowerCase()),
            )
            .map((record) => effectiveWorkCenter(record)),
        ),
      )
        .filter(Boolean)
        .sort((left, right) => left.localeCompare(right)),
    [draft.area, draft.site, records],
  )

  const workUnitOptions = useMemo(
    () =>
      Array.from(
        new Set(
          records
            .filter(
              (record) =>
                record.node_kind === 'work-unit' &&
                (!draft.site.trim() || effectiveSite(record).toLowerCase() === draft.site.trim().toLowerCase()) &&
                (!draft.area.trim() || effectiveArea(record).toLowerCase() === draft.area.trim().toLowerCase()) &&
                (!draft.workCenter.trim() ||
                  effectiveWorkCenter(record).toLowerCase() === draft.workCenter.trim().toLowerCase()),
            )
            .map((record) => effectiveWorkUnit(record)),
        ),
      )
        .filter(Boolean)
        .sort((left, right) => left.localeCompare(right)),
    [draft.area, draft.site, draft.workCenter, records],
  )

  const previewFqdn = useMemo(() => buildPreviewFqdn(draft), [draft])
  const hierarchyStatus = useMemo(() => buildHierarchyStatus(records, draft), [draft, records])
  const duplicateRecord = useMemo(
    () =>
      previewFqdn !== 'Incomplete hierarchy'
        ? records.find((record) => record.fqdn.toLowerCase() === previewFqdn.toLowerCase())
        : undefined,
    [previewFqdn, records],
  )
  const hasHardwareAnchor =
    Boolean(draft.mac.trim()) || draft.hardwareRows.some((row) => row.value.trim().length > 0)
  const missingParents = hierarchyStatus.filter((entry) => entry.required && entry.value && !entry.exists)
  const missingLabels = hierarchyStatus.filter((entry) => !entry.value)
  const readyToCreate =
    missingLabels.length === 0 && hasHardwareAnchor && missingParents.length === 0 && !duplicateRecord

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    setSuccessMessage(undefined)

    const payload = buildObservationInput(draft)
    const record = await createObservation.mutateAsync(payload)
    onCreated(record)
    setSuccessMessage(`Created ${record.fqdn}.`)
    setDraft((current) => advanceDraftAfterCreate(record, current))
  }

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-[rgba(4,5,10,0.74)] backdrop-blur-[2px]">
      <div className="flex h-full w-full max-w-[1120px] flex-col border-l border-[var(--border)] bg-[linear-gradient(180deg,rgba(11,9,17,0.98),rgba(7,6,12,0.99))]">
        <header className="border-b border-[var(--border)] px-4 py-3">
          <div className="flex items-start justify-between gap-4">
            <div>
              <div className="mono text-[10px] uppercase tracking-[0.28em] text-[var(--accent-cyan)]">
                Hierarchy Builder
              </div>
              <h2 className="mt-1 text-[18px] font-semibold text-[var(--text-strong)]">
                Create resolvable ISA-95 records
              </h2>
              <p className="mt-1 max-w-3xl text-[12px] leading-[1.2rem] text-[var(--text-muted)]">
                Create sites, areas, work centers, work units, and devices with a live DNS preview, hardware anchor,
                and application IDs. Parent levels must already exist before child levels can be created.
              </p>
            </div>
            <button
              className="inline-flex h-9 w-9 items-center justify-center rounded-[9px] border border-[var(--border)] text-[var(--text-muted)] transition hover:border-[var(--border-strong)] hover:text-[var(--text-strong)]"
              onClick={onClose}
              type="button"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </header>

        <form className="min-h-0 flex-1 overflow-auto" onSubmit={handleSubmit}>
          <div className="grid gap-4 p-4 xl:grid-cols-[minmax(0,1.3fr)_320px]">
            <div className="space-y-4">
              <BuilderSection
                detail="Pick the ISA-95 level you are creating, then fill the path from top to bottom."
                icon={Layers3}
                title="1. Placement and naming"
              >
                <div className="space-y-3">
                  <div className="inline-flex overflow-hidden rounded-[10px] border border-[var(--border)] bg-[rgba(255,255,255,0.02)]">
                    {nodeKindOrder.map((kind) => (
                      <button
                        key={kind}
                        className={clsx(
                          'relative inline-flex h-8 items-center border-r border-[var(--border)] px-3 text-[10px] font-medium uppercase tracking-[0.14em] last:border-r-0',
                          draft.nodeKind === kind
                            ? 'bg-[rgba(126,249,255,0.09)] text-[var(--accent-cyan)]'
                            : 'text-[var(--text-dim)] hover:bg-[rgba(255,255,255,0.03)] hover:text-[var(--text)]',
                        )}
                        onClick={() => {
                          setDraft((current) => trimDraftForKind(current, kind))
                          setSuccessMessage(undefined)
                        }}
                        type="button"
                      >
                        {draft.nodeKind === kind ? <span className="absolute inset-x-0 top-0 h-px bg-current/80" /> : null}
                        {formatNodeKind(kind)}
                      </button>
                    ))}
                  </div>

                  <div className="rounded-[10px] border border-[var(--border)] bg-[rgba(255,255,255,0.02)] px-3 py-2.5">
                    <div className="text-[10px] uppercase tracking-[0.18em] text-[var(--text-muted)]">Live FQDN</div>
                    <div className="mono mt-1 text-[14px] text-[var(--text-strong)]">{previewFqdn}</div>
                  </div>

                  <div className="grid gap-3 md:grid-cols-2">
                    <Field label="Site" detail={draft.nodeKind === 'site' ? 'Top-level plant or campus' : 'Select an existing site'}>
                      {draft.nodeKind === 'site' ? (
                        <Input
                          onChange={(event) => setDraft((current) => ({ ...current, site: event.target.value }))}
                          placeholder="Milwaukee"
                          value={draft.site}
                        />
                      ) : (
                        <Select
                          onChange={(event) =>
                            setDraft((current) => trimDraftForKind({ ...current, site: event.target.value, area: '', workCenter: '', workUnit: '' }, current.nodeKind))
                          }
                          value={draft.site}
                        >
                          <option value="">Select site</option>
                          {siteOptions.map((option) => (
                            <option key={option} value={option}>
                              {option}
                            </option>
                          ))}
                        </Select>
                      )}
                    </Field>

                    <Field
                      label={nodeLabelCopy[draft.nodeKind]}
                      detail={
                        draft.nodeKind === 'device'
                          ? 'The first DNS label, such as CaseRobot'
                          : 'This becomes the resolvable label for the selected ISA-95 level'
                      }
                    >
                      {draft.nodeKind === 'area' ? (
                        <Input
                          onChange={(event) => setDraft((current) => ({ ...current, area: event.target.value }))}
                          placeholder="Zone4"
                          value={draft.area}
                        />
                      ) : draft.nodeKind === 'work-center' ? (
                        <Input
                          onChange={(event) => setDraft((current) => ({ ...current, workCenter: event.target.value }))}
                          placeholder="Packout"
                          value={draft.workCenter}
                        />
                      ) : draft.nodeKind === 'work-unit' ? (
                        <Input
                          onChange={(event) => setDraft((current) => ({ ...current, workUnit: event.target.value }))}
                          placeholder="Palletizer"
                          value={draft.workUnit}
                        />
                      ) : draft.nodeKind === 'device' ? (
                        <Input
                          onChange={(event) => setDraft((current) => ({ ...current, leaf: event.target.value }))}
                          placeholder="CaseRobot"
                          value={draft.leaf}
                        />
                      ) : (
                        <Input
                          onChange={(event) => setDraft((current) => ({ ...current, site: event.target.value }))}
                          placeholder="Milwaukee"
                          value={draft.site}
                        />
                      )}
                    </Field>

                    {draft.nodeKind !== 'site' ? (
                      <Field
                        label="Area"
                        detail={draft.nodeKind === 'area' ? 'New area label' : 'Existing parent area'}
                      >
                        {draft.nodeKind === 'area' ? (
                          <Input
                            onChange={(event) => setDraft((current) => ({ ...current, area: event.target.value }))}
                            placeholder="Zone4"
                            value={draft.area}
                          />
                        ) : (
                          <Select
                            onChange={(event) =>
                              setDraft((current) =>
                                trimDraftForKind(
                                  { ...current, area: event.target.value, workCenter: '', workUnit: '' },
                                  current.nodeKind,
                                ),
                              )
                            }
                            value={draft.area}
                          >
                            <option value="">Select area</option>
                            {areaOptions.map((option) => (
                              <option key={option} value={option}>
                                {option}
                              </option>
                            ))}
                          </Select>
                        )}
                      </Field>
                    ) : null}

                    {nodeKindOrder.indexOf(draft.nodeKind) >= nodeKindOrder.indexOf('work-center') ? (
                      <Field
                        label="Work center"
                        detail={draft.nodeKind === 'work-center' ? 'New line, cell, or work center' : 'Existing parent work center'}
                      >
                        {draft.nodeKind === 'work-center' ? (
                          <Input
                            onChange={(event) => setDraft((current) => ({ ...current, workCenter: event.target.value }))}
                            placeholder="Packout"
                            value={draft.workCenter}
                          />
                        ) : (
                          <Select
                            onChange={(event) =>
                              setDraft((current) =>
                                trimDraftForKind({ ...current, workCenter: event.target.value, workUnit: '' }, current.nodeKind),
                              )
                            }
                            value={draft.workCenter}
                          >
                            <option value="">Select work center</option>
                            {workCenterOptions.map((option) => (
                              <option key={option} value={option}>
                                {option}
                              </option>
                            ))}
                          </Select>
                        )}
                      </Field>
                    ) : null}

                    {nodeKindOrder.indexOf(draft.nodeKind) >= nodeKindOrder.indexOf('work-center') ? (
                      <Field label="Work center kind" detail="Optional ISA-95 work center type">
                        <Select
                          onChange={(event) => setDraft((current) => ({ ...current, workCenterKind: event.target.value }))}
                          value={draft.workCenterKind}
                        >
                          <option value="">Unspecified</option>
                          {workCenterKindOptions.map((option) => (
                            <option key={option} value={option}>
                              {humanizeKebab(option)}
                            </option>
                          ))}
                        </Select>
                      </Field>
                    ) : null}

                    {nodeKindOrder.indexOf(draft.nodeKind) >= nodeKindOrder.indexOf('work-unit') ? (
                      <Field
                        label="Work unit"
                        detail={draft.nodeKind === 'work-unit' ? 'New machine or process unit' : 'Existing parent work unit'}
                      >
                        {draft.nodeKind === 'work-unit' ? (
                          <Input
                            onChange={(event) => setDraft((current) => ({ ...current, workUnit: event.target.value }))}
                            placeholder="Palletizer"
                            value={draft.workUnit}
                          />
                        ) : (
                          <Select
                            onChange={(event) => setDraft((current) => ({ ...current, workUnit: event.target.value }))}
                            value={draft.workUnit}
                          >
                            <option value="">Select work unit</option>
                            {workUnitOptions.map((option) => (
                              <option key={option} value={option}>
                                {option}
                              </option>
                            ))}
                          </Select>
                        )}
                      </Field>
                    ) : null}
                  </div>
                </div>
              </BuilderSection>

              <BuilderSection
                detail="Bind the DNS node to actual network or hardware reality so the record can survive renames."
                icon={ShieldCheck}
                title="2. Hardware anchor and addressing"
              >
                <div className="grid gap-3 md:grid-cols-2">
                  <Field label="MAC address" detail="Preferred when DHCP or switch data can supply the same MAC">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, mac: event.target.value }))}
                      placeholder="ac:de:48:00:99:11"
                      value={draft.mac}
                    />
                  </Field>
                  <Field label="Enterprise" detail="Plant, division, or company name">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, enterprise: event.target.value }))}
                      placeholder="Butterbones"
                      value={draft.enterprise}
                    />
                  </Field>
                  <Field label="Internal IP" detail="Primary control network address">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, internalIp: event.target.value }))}
                      placeholder="192.168.3.42"
                      value={draft.internalIp}
                    />
                  </Field>
                  <Field label="External IP" detail="Routed or plant-wide address if present">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, externalIp: event.target.value }))}
                      placeholder="10.50.3.42"
                      value={draft.externalIp}
                    />
                  </Field>
                  <Field label="Class" detail="Function or asset class, such as robot or line-switch">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, className: event.target.value }))}
                      placeholder={draft.nodeKind === 'device' ? 'robot' : 'line-switch'}
                      value={draft.className}
                    />
                  </Field>
                  <Field label="Vendor" detail="Maker or platform owner">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, vendor: event.target.value }))}
                      placeholder="FANUC"
                      value={draft.vendor}
                    />
                  </Field>
                  <Field label="Model" detail="Optional model or family">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, model: event.target.value }))}
                      placeholder="M-20iD"
                      value={draft.model}
                    />
                  </Field>
                  <Field label="Switch port" detail="Observed or planned port">
                    <Input
                      onChange={(event) => setDraft((current) => ({ ...current, switchPort: event.target.value }))}
                      placeholder="sw-zone4-01 Gi1/0/12"
                      value={draft.switchPort}
                    />
                  </Field>
                  <Field
                    label="Protocols"
                    detail="Comma or line-separated list, such as ethernet-ip, opc-ua, modbus-tcp"
                  >
                    <Textarea
                      className="min-h-[72px]"
                      onChange={(event) => setDraft((current) => ({ ...current, protocols: event.target.value }))}
                      placeholder="ethernet-ip, opc-ua"
                      value={draft.protocols}
                    />
                  </Field>
                </div>

                <div className="mt-4 rounded-[10px] border border-[var(--border)] bg-[rgba(255,255,255,0.015)] p-3">
                  <div className="flex items-center justify-between gap-3">
                    <div>
                      <div className="text-[11px] font-semibold text-[var(--text-strong)]">Additional hardware identities</div>
                      <div className="mt-0.5 text-[11px] text-[var(--text-muted)]">
                        Use serials, DHCP client IDs, or future certificate anchors when MAC is not the only binding.
                      </div>
                    </div>
                    <button
                      className="inline-flex items-center gap-1 rounded-[8px] border border-[var(--border)] px-2.5 py-1 text-[10px] uppercase tracking-[0.14em] text-[var(--text-muted)] transition hover:border-[var(--border-strong)] hover:text-[var(--text)]"
                      onClick={() =>
                        setDraft((current) => ({
                          ...current,
                          hardwareRows: [...current.hardwareRows, makeHardwareRow('serial-number')],
                        }))
                      }
                      type="button"
                    >
                      <Plus className="h-3 w-3" />
                      Add ID
                    </button>
                  </div>
                  <div className="mt-3 space-y-2">
                    {draft.hardwareRows.length > 0 ? (
                      draft.hardwareRows.map((row) => (
                        <div key={row.id} className="grid gap-2 md:grid-cols-[170px_minmax(0,1fr)_160px_auto]">
                          <Select
                            onChange={(event) =>
                              setDraft((current) => ({
                                ...current,
                                hardwareRows: current.hardwareRows.map((entry) =>
                                  entry.id === row.id ? { ...entry, kind: event.target.value } : entry,
                                ),
                              }))
                            }
                            value={row.kind}
                          >
                            {hardwareKindOptions.map((option) => (
                              <option key={option} value={option}>
                                {humanizeKebab(option)}
                              </option>
                            ))}
                          </Select>
                          <Input
                            onChange={(event) =>
                              setDraft((current) => ({
                                ...current,
                                hardwareRows: current.hardwareRows.map((entry) =>
                                  entry.id === row.id ? { ...entry, value: event.target.value } : entry,
                                ),
                              }))
                            }
                            placeholder="RTR-MKE-CORE-01"
                            value={row.value}
                          />
                          <Input
                            onChange={(event) =>
                              setDraft((current) => ({
                                ...current,
                                hardwareRows: current.hardwareRows.map((entry) =>
                                  entry.id === row.id ? { ...entry, label: event.target.value } : entry,
                                ),
                              }))
                            }
                            placeholder="inventory"
                            value={row.label}
                          />
                          <button
                            className="inline-flex items-center justify-center rounded-[8px] border border-[var(--border)] px-2 text-[var(--text-dim)] transition hover:border-[var(--critical)] hover:text-[var(--critical)]"
                            onClick={() =>
                              setDraft((current) => ({
                                ...current,
                                hardwareRows: current.hardwareRows.filter((entry) => entry.id !== row.id),
                              }))
                            }
                            type="button"
                          >
                            <X className="h-4 w-4" />
                          </button>
                        </div>
                      ))
                    ) : (
                      <div className="text-[11px] text-[var(--text-dim)]">
                        No additional hardware IDs yet. MAC alone is fine when that is your anchor.
                      </div>
                    )}
                  </div>
                </div>
              </BuilderSection>

              <BuilderSection
                detail="Stable application IDs stay with the asset even if the DNS name changes later."
                icon={Link2}
                title="3. Application identities"
              >
                <div className="grid gap-3 md:grid-cols-3">
                  <Field label="URNs" detail="One per line. Must start with urn:">
                    <Textarea
                      onChange={(event) => setDraft((current) => ({ ...current, urns: event.target.value }))}
                      placeholder={'urn:mes:asset:case-robot-204\nurn:cmms:asset:84721'}
                      value={draft.urns}
                    />
                  </Field>
                  <Field label="UNIs" detail="One per line. Good for app-local stable IDs">
                    <Textarea
                      onChange={(event) => setDraft((current) => ({ ...current, unis: event.target.value }))}
                      placeholder={'uni://packout/palletizer/caserobot'}
                      value={draft.unis}
                    />
                  </Field>
                  <Field label="Aliases" detail="Operator handles, legacy names, or search shortcuts">
                    <Textarea
                      onChange={(event) => setDraft((current) => ({ ...current, aliases: event.target.value }))}
                      placeholder={'case-robot\npackout-robot-204'}
                      value={draft.aliases}
                    />
                  </Field>
                </div>
              </BuilderSection>
            </div>

            <div className="space-y-4">
              <section className="rounded-[10px] border border-[var(--border)] bg-[color:var(--bg-panel)]">
                <div className="border-b border-[var(--border)] px-3.5 py-2.5">
                  <div className="text-[11px] font-semibold text-[var(--text-strong)]">Ready check</div>
                  <div className="mt-0.5 text-[11px] text-[var(--text-muted)]">
                    Validate the chain before the record hits the authoritative store.
                  </div>
                </div>
                <div className="space-y-2 p-3.5 text-[12px]">
                  <StatusLine
                    ok={missingLabels.length === 0}
                    text={
                      missingLabels.length === 0
                        ? 'Required ISA-95 labels are filled in.'
                        : `Missing ${missingLabels.map((entry) => entry.label.toLowerCase()).join(', ')}.`
                    }
                  />
                  <StatusLine
                    ok={hasHardwareAnchor}
                    text={
                      hasHardwareAnchor
                        ? 'Hardware anchor present.'
                        : 'Add a MAC address or another hardware identity.'
                    }
                  />
                  <StatusLine
                    ok={missingParents.length === 0}
                    text={
                      missingParents.length === 0
                        ? 'Parent hierarchy is resolvable.'
                        : `Missing ${missingParents.map((entry) => entry.label.toLowerCase()).join(', ')} record.`
                    }
                  />
                  <StatusLine
                    ok={!duplicateRecord}
                    text={
                      duplicateRecord
                        ? `A record already exists at ${duplicateRecord.fqdn}.`
                        : 'No duplicate FQDN detected.'
                    }
                  />
                </div>
              </section>

              <section className="rounded-[10px] border border-[var(--border)] bg-[color:var(--bg-panel)]">
                <div className="border-b border-[var(--border)] px-3.5 py-2.5">
                  <div className="text-[11px] font-semibold text-[var(--text-strong)]">Hierarchy chain</div>
                  <div className="mt-0.5 text-[11px] text-[var(--text-muted)]">
                    Every parent below the current node has to resolve first.
                  </div>
                </div>
                <div className="space-y-2 p-3.5">
                  {hierarchyStatus.map((entry) => (
                    <div
                      key={entry.kind}
                      className="rounded-[9px] border border-[var(--border)] bg-[rgba(255,255,255,0.015)] px-3 py-2"
                    >
                      <div className="flex items-center justify-between gap-3">
                        <div>
                          <div className="text-[10px] uppercase tracking-[0.16em] text-[var(--text-muted)]">
                            {entry.label}
                          </div>
                          <div className="mt-1 text-[13px] text-[var(--text-strong)]">
                            {entry.value || 'Not set'}
                          </div>
                        </div>
                        <span
                          className={clsx(
                            'mono rounded-full border px-2 py-0.5 text-[9px] uppercase tracking-[0.16em]',
                            entry.exists
                              ? 'border-[rgba(114,241,184,0.24)] bg-[rgba(114,241,184,0.1)] text-[var(--ok)]'
                              : entry.required
                                ? 'border-[rgba(255,94,125,0.24)] bg-[rgba(255,94,125,0.1)] text-[var(--critical)]'
                                : 'border-[rgba(126,249,255,0.24)] bg-[rgba(126,249,255,0.08)] text-[var(--accent-cyan)]',
                          )}
                        >
                          {entry.exists ? 'ready' : entry.required ? 'missing' : 'new'}
                        </span>
                      </div>
                      {!entry.exists && entry.required && entry.value ? (
                        <button
                          className="mt-2 inline-flex items-center gap-1 text-[11px] text-[var(--accent-cyan)] transition hover:text-[var(--text-strong)]"
                          onClick={() => setDraft((current) => trimDraftForKind(current, entry.kind))}
                          type="button"
                        >
                          Create {entry.label.toLowerCase()}
                          <ArrowRight className="h-3.5 w-3.5" />
                        </button>
                      ) : null}
                    </div>
                  ))}
                </div>
              </section>

              {createObservation.isError ? (
                <section className="rounded-[10px] border border-[rgba(255,94,125,0.24)] bg-[rgba(255,94,125,0.08)] px-3.5 py-3 text-[12px] text-[var(--critical)]">
                  <div className="flex items-start gap-2">
                    <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" />
                    <div>
                      <div className="font-semibold">Create failed</div>
                      <div className="mt-1 text-[11px] leading-[1.15rem] text-[rgba(255,94,125,0.92)]">
                        {createObservation.error.message}
                      </div>
                    </div>
                  </div>
                </section>
              ) : null}

              {successMessage ? (
                <section className="rounded-[10px] border border-[rgba(114,241,184,0.24)] bg-[rgba(114,241,184,0.08)] px-3.5 py-3 text-[12px] text-[var(--ok)]">
                  <div className="flex items-start gap-2">
                    <Check className="mt-0.5 h-4 w-4 shrink-0" />
                    <div>
                      <div className="font-semibold">Create complete</div>
                      <div className="mt-1 text-[11px] leading-[1.15rem] text-[rgba(114,241,184,0.9)]">
                        {successMessage} The builder is ready for the next level.
                      </div>
                    </div>
                  </div>
                </section>
              ) : null}

              <section className="rounded-[10px] border border-[var(--border)] bg-[color:var(--bg-panel)] p-3.5">
                <div className="text-[11px] font-semibold text-[var(--text-strong)]">Operator guidance</div>
                <ul className="mt-2 space-y-2 text-[11px] leading-[1.15rem] text-[var(--text-muted)]">
                  <li>Sites often map to a core router or routed plant edge, not a physical building label only.</li>
                  <li>Use work centers for lines or cells, then work units for machines or process islands beneath them.</li>
                  <li>Use URNs and UNIs for MES, CMMS, historians, and applications. Keep DNS focused on resolvable placement.</li>
                </ul>
              </section>
            </div>
          </div>

          <footer className="flex items-center justify-between gap-3 border-t border-[var(--border)] px-4 py-3">
            <div className="text-[11px] text-[var(--text-muted)]">
              {readyToCreate ? 'Record is ready for authoritative create.' : 'Resolve the highlighted gaps before create.'}
            </div>
            <div className="flex items-center gap-2">
              <button
                className="rounded-[9px] border border-[var(--border)] px-3 py-2 text-[12px] text-[var(--text-muted)] transition hover:border-[var(--border-strong)] hover:text-[var(--text)]"
                onClick={onClose}
                type="button"
              >
                Close
              </button>
              <button
                className={clsx(
                  'inline-flex items-center gap-2 rounded-[9px] border px-3.5 py-2 text-[12px] font-semibold uppercase tracking-[0.14em] transition',
                  readyToCreate
                    ? 'border-[rgba(255,106,193,0.3)] bg-[rgba(255,106,193,0.12)] text-[var(--text-strong)] hover:border-[var(--border-strong)] hover:bg-[rgba(255,106,193,0.18)]'
                    : 'cursor-not-allowed border-[var(--border)] bg-[rgba(255,255,255,0.03)] text-[var(--text-dim)]',
                )}
                disabled={!readyToCreate || createObservation.isPending}
                type="submit"
              >
                <Network className="h-4 w-4" />
                {createObservation.isPending ? 'Creating…' : `Create ${formatNodeKind(draft.nodeKind)}`}
              </button>
            </div>
          </footer>
        </form>
      </div>
    </div>
  )
}

function StatusLine({
  ok,
  text,
}: {
  ok: boolean
  text: string
}) {
  return (
    <div className="flex items-start gap-2">
      {ok ? (
        <Check className="mt-0.5 h-4 w-4 shrink-0 text-[var(--ok)]" />
      ) : (
        <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-[var(--warn)]" />
      )}
      <div className={clsx('leading-[1.15rem]', ok ? 'text-[var(--text)]' : 'text-[var(--warn)]')}>{text}</div>
    </div>
  )
}
