import { useDeferredValue, useEffect, useMemo, useState, useTransition } from 'react'
import {
  Background,
  Controls,
  type Edge,
  MiniMap,
  type NodeProps,
  ReactFlow,
} from '@xyflow/react'
import { Search } from 'lucide-react'
import { Panel } from '../components/panel'
import { StatusBadge } from '../components/status-badge'
import { useRecordsQuery } from '../lib/api'
import {
  buildHierarchyGraph,
  type HierarchyFlowNode,
} from '../lib/graph'
import {
  effectiveArea,
  effectiveSite,
  effectiveWorkCenter,
  effectiveWorkUnit,
  formatTimestamp,
} from '../lib/format'
import type { SemanticRecord } from '../types'

const nodeTypes = {
  hierarchy: HierarchyNode,
}

export function GraphPage() {
  const recordsQuery = useRecordsQuery()
  const records = recordsQuery.data
  const [search, setSearch] = useState('')
  const [selectedNode, setSelectedNode] = useState<HierarchyFlowNode | null>(null)
  const [nodes, setNodes] = useState<HierarchyFlowNode[]>([])
  const [edges, setEdges] = useState<Edge[]>([])
  const [isPending, startTransition] = useTransition()
  const deferredSearch = useDeferredValue(search)

  const filteredRecords = useMemo(() => {
    const sourceRecords = records ?? []
    const needle = deferredSearch.trim().toLowerCase()
    if (!needle) {
      return sourceRecords
    }

    return sourceRecords.filter((record) =>
      [
        record.fqdn,
        record.class,
        record.vendor,
        record.model,
        effectiveSite(record),
        effectiveArea(record),
        effectiveWorkCenter(record),
        effectiveWorkUnit(record),
      ]
        .filter(Boolean)
        .some((value) => value?.toLowerCase().includes(needle)),
    )
  }, [deferredSearch, records])

  useEffect(() => {
    let cancelled = false

    async function run() {
      const graph = await buildHierarchyGraph(filteredRecords)
      if (cancelled) {
        return
      }
      startTransition(() => {
        setNodes(graph.nodes)
        setEdges(graph.edges)
        setSelectedNode((current) =>
          current ? graph.nodes.find((node) => node.id === current.id) ?? null : graph.nodes[0] ?? null,
        )
      })
    }

    void run()

    return () => {
      cancelled = true
    }
  }, [filteredRecords])

  return (
    <div className="grid gap-4 xl:grid-cols-[minmax(0,1.7fr)_356px]">
      <Panel
        title="ISA-95 Hierarchy"
        eyebrow="Graph"
        detail="Read-only semantic placement graph from site to device."
        actions={
          <div className="flex items-center gap-2">
            <StatusBadge label={`${filteredRecords.length} assets`} tone="high" />
            <StatusBadge label={isPending ? 'layouting' : 'stable'} tone={isPending ? 'medium' : 'ok'} />
          </div>
        }
      >
          <div className="space-y-3">
          <label className="flex items-center gap-3 rounded-[10px] border border-[var(--border)] bg-[color:var(--bg)] px-3.5 py-2.5">
            <Search className="h-4 w-4 text-[var(--text-muted)]" />
            <input
              className="w-full bg-transparent text-[12px] text-[var(--text-strong)] outline-none placeholder:text-[var(--text-muted)]"
              onChange={(event) => startTransition(() => setSearch(event.target.value))}
              placeholder="Filter graph by name, class, vendor, or location"
              value={search}
            />
          </label>

          <div className="panel-grid h-[720px] overflow-hidden rounded-[10px] border border-[var(--border)]/80">
            <ReactFlow
              edges={edges}
              fitView
              nodeTypes={nodeTypes}
              nodes={nodes}
              nodesConnectable={false}
              nodesDraggable={false}
              onNodeClick={(_, node) => setSelectedNode(node)}
            >
              <Background gap={24} size={1} />
              <MiniMap
                nodeColor={(node) =>
                  node.data.status === 'quarantined'
                    ? '#ff5e7d'
                    : node.data.status === 'released' || node.data.status === 'expired'
                      ? '#ffd866'
                      : '#7ef9ff'
                }
                pannable
                zoomable
              />
              <Controls showInteractive={false} />
            </ReactFlow>
          </div>
        </div>
      </Panel>

      <Panel
        title={selectedNode?.data.label ?? 'Node Inspector'}
        eyebrow="Selection"
        detail="Read-only selection detail for placement and health."
      >
        {selectedNode ? (
          <NodeInspector node={selectedNode} />
        ) : (
          <div className="rounded-xl border border-dashed border-[var(--border)] px-4 py-12 text-center text-sm text-[var(--text-muted)]">
            {recordsQuery.isLoading
              ? 'Loading graph data...'
              : 'Select a node to inspect its role in the hierarchy.'}
          </div>
        )}
      </Panel>
    </div>
  )
}

function HierarchyNode({ data, selected }: NodeProps<HierarchyFlowNode>) {
  const kindTone =
    data.kind === 'device'
      ? 'border-[rgba(126,249,255,0.18)] bg-[rgba(9,7,15,0.98)]'
      : 'border-[var(--border)] bg-[rgba(15,13,22,0.94)]'

  return (
    <div
      className={[
        'min-w-[164px] rounded-[10px] border px-3.5 py-2.5 shadow-[var(--shadow)] transition',
        kindTone,
        selected ? 'ring-2 ring-[rgba(255,106,193,0.42)]' : '',
      ].join(' ')}
    >
      <div className="flex items-center justify-between gap-3">
        <div className="mono text-[9px] uppercase tracking-[0.2em] text-[var(--accent)]">
          {data.kind.replace('_', ' ')}
        </div>
        <StatusBadge label={data.status} />
      </div>
      <div className="mt-1.5 text-[13px] font-semibold text-[var(--text-strong)]">{data.label}</div>
      <div className="mt-1 text-[10px] leading-4 text-[var(--text-muted)]">{data.secondary}</div>
      <div className="mono mt-1.5 text-[9px] uppercase tracking-[0.18em] text-[var(--text-muted)]">
        {data.count} mapped
      </div>
    </div>
  )
}

function NodeInspector({ node }: { node: HierarchyFlowNode }) {
  const record = node.data.record

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-2">
        <InfoChip label="Kind" value={node.data.kind.replace('_', ' ')} />
        <InfoChip label="Mapped count" value={String(node.data.count)} />
      </div>
      <div className="rounded-[10px] border border-[var(--border)]/80 bg-[color:var(--bg)] p-3">
        <div className="text-[10px] uppercase tracking-[0.18em] text-[var(--text-muted)]">Summary</div>
        <div className="mt-2.5 space-y-2 text-[13px] text-[var(--text)]">
          <div>{node.data.secondary}</div>
          <div>
            <StatusBadge label={node.data.status} />
          </div>
        </div>
      </div>

      {record ? <RecordDetails record={record} /> : null}
    </div>
  )
}

function RecordDetails({ record }: { record: SemanticRecord }) {
  return (
    <div className="rounded-[10px] border border-[var(--border)]/80 bg-[rgba(255,255,255,0.015)] p-3 text-[13px]">
      <div className="text-[10px] uppercase tracking-[0.18em] text-[var(--text-muted)]">Bound device</div>
      <div className="mt-2.5 space-y-2">
        <InspectorLine label="FQDN" value={record.fqdn} />
        <InspectorLine label="Class / Vendor" value={`${record.class ?? 'unknown'} / ${record.vendor ?? 'unknown'}`} />
        <InspectorLine
          label="Location"
          value={`${effectiveSite(record)} / ${effectiveArea(record)} / ${effectiveWorkCenter(record)} / ${effectiveWorkUnit(record)}`}
        />
        <InspectorLine label="Internal IP" value={record.internal_ip ?? 'Unavailable'} />
        <InspectorLine label="Updated" value={formatTimestamp(record.updated_at)} />
      </div>
    </div>
  )
}

function InfoChip({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-[10px] border border-[var(--border)]/80 bg-[rgba(255,255,255,0.02)] px-3 py-2.5">
      <div className="text-[9px] uppercase tracking-[0.18em] text-[var(--text-muted)]">{label}</div>
      <div className="mt-1.5 text-[13px] font-medium text-[var(--text-strong)]">{value}</div>
    </div>
  )
}

function InspectorLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[88px_minmax(0,1fr)] gap-3">
      <div className="text-[9px] uppercase tracking-[0.16em] text-[var(--text-muted)]">{label}</div>
      <div className="text-[var(--text-strong)]">{value}</div>
    </div>
  )
}
