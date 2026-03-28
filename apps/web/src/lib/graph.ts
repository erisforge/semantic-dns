import type { Edge, Node } from '@xyflow/react'
import ELK from 'elkjs/lib/elk.bundled.js'
import type { SemanticRecord } from '../types'
import {
  effectiveArea,
  effectiveLeaf,
  effectiveSite,
  effectiveWorkCenter,
  effectiveWorkUnit,
} from './format'

export type GraphNodeData = {
  kind: 'site' | 'area' | 'work_center' | 'work_unit' | 'device'
  label: string
  secondary: string
  status: string
  count: number
  record?: SemanticRecord
}

export type HierarchyFlowNode = Node<GraphNodeData, 'hierarchy'>

const elk = new ELK()

function groupNodeId(kind: GraphNodeData['kind'], path: string[]): string {
  return `${kind}:${path.join('/')}`
}

function deviceNodeId(record: SemanticRecord): string {
  return `device:${record.device_id}`
}

function statusRank(status: string): number {
  switch (status) {
    case 'quarantined':
      return 3
    case 'expired':
      return 2
    case 'released':
      return 1
    default:
      return 0
  }
}

function summarizeStatus(existing: string, next: string): string {
  return statusRank(next) > statusRank(existing) ? next : existing
}

export async function buildHierarchyGraph(records: SemanticRecord[]): Promise<{
  nodes: HierarchyFlowNode[]
  edges: Edge[]
}> {
  const nodes = new Map<string, HierarchyFlowNode>()
  const edges = new Map<string, Edge>()

  const upsertGroup = (
    id: string,
    kind: GraphNodeData['kind'],
    label: string,
    secondary: string,
    status: string,
  ) => {
    const existing = nodes.get(id)
    if (existing) {
      existing.data.count += 1
      existing.data.status = summarizeStatus(existing.data.status, status)
      nodes.set(id, existing)
      return
    }

    nodes.set(id, {
      id,
      type: 'hierarchy',
      position: { x: 0, y: 0 },
      data: {
        kind,
        label,
        secondary,
        status,
        count: 1,
      },
    })
  }

  const upsertEdge = (source: string, target: string) => {
    const id = `${source}->${target}`
    if (!edges.has(id)) {
      edges.set(id, {
        id,
        source,
        target,
        animated: false,
        style: { stroke: 'rgba(125, 139, 152, 0.42)', strokeWidth: 1.4 },
      })
    }
  }

  for (const record of records) {
    const site = effectiveSite(record)
    const area = effectiveArea(record)
    const workCenter = effectiveWorkCenter(record)
    const workUnit = effectiveWorkUnit(record)
    const leaf = effectiveLeaf(record)

    const siteId = groupNodeId('site', [site])
    const areaId = groupNodeId('area', [site, area])
    const workCenterId = groupNodeId('work_center', [site, area, workCenter])
    const workUnitId = groupNodeId('work_unit', [site, area, workCenter, workUnit])
    const deviceId = deviceNodeId(record)

    upsertGroup(siteId, 'site', site, record.enterprise ?? 'ISA-95 site', record.status)
    upsertGroup(areaId, 'area', area, site, record.status)
    upsertGroup(workCenterId, 'work_center', workCenter, record.work_center_kind ?? area, record.status)
    upsertGroup(workUnitId, 'work_unit', workUnit, leaf, record.status)

    nodes.set(deviceId, {
      id: deviceId,
      type: 'hierarchy',
      position: { x: 0, y: 0 },
      data: {
        kind: 'device',
        label: leaf,
        secondary: `${record.class ?? 'asset'} · ${record.vendor ?? 'unknown vendor'}`,
        status: record.status,
        count: 1,
        record,
      },
    })

    upsertEdge(siteId, areaId)
    upsertEdge(areaId, workCenterId)
    upsertEdge(workCenterId, workUnitId)
    upsertEdge(workUnitId, deviceId)
  }

  const elkGraph = await elk.layout({
    id: 'root',
    layoutOptions: {
      'elk.algorithm': 'layered',
      'elk.direction': 'DOWN',
      'elk.edgeRouting': 'ORTHOGONAL',
      'elk.layered.spacing.nodeNodeBetweenLayers': '80',
      'elk.spacing.nodeNode': '36',
    },
    children: Array.from(nodes.values()).map((node) => ({
      id: node.id,
      width: node.data.kind === 'device' ? 220 : 178,
      height: node.data.kind === 'device' ? 88 : 68,
    })),
    edges: Array.from(edges.values()).map((edge) => ({
      id: edge.id,
      sources: [edge.source],
      targets: [edge.target],
    })),
  })

  const layoutById = new Map(
    (elkGraph.children ?? []).map((child) => [child.id, { x: child.x ?? 0, y: child.y ?? 0 }]),
  )

  const layoutedNodes = Array.from(nodes.values()).map((node) => ({
    ...node,
    position: layoutById.get(node.id) ?? { x: 0, y: 0 },
  }))

  return {
    nodes: layoutedNodes,
    edges: Array.from(edges.values()),
  }
}
