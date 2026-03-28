import type { SemanticRecord } from '../types'

const nodeKindLabels: Record<string, string> = {
  site: 'Site',
  area: 'Area',
  'work-center': 'Work Center',
  'work-unit': 'Work Unit',
  device: 'Device',
}

const hardwareIdentityLabels: Record<string, string> = {
  'mac-address': 'MAC Address',
  'serial-number': 'Serial Number',
  'dhcp-client-id': 'DHCP Client ID',
  'x509-subject': 'X.509 Subject',
  'x509-san-uri': 'X.509 SAN URI',
  'x509-spki-sha256': 'X.509 SPKI SHA-256',
}

const applicationIdentityLabels: Record<string, string> = {
  urn: 'URN',
  uni: 'UNI',
}

export function formatTimestamp(value?: string | null): string {
  if (!value) {
    return 'Unavailable'
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value))
}

export function formatRelative(value?: string | null): string {
  if (!value) {
    return 'Never'
  }

  const diffMs = Date.now() - new Date(value).getTime()
  const minutes = Math.round(diffMs / 60_000)

  if (minutes < 1) {
    return 'Just now'
  }
  if (minutes < 60) {
    return `${minutes}m ago`
  }

  const hours = Math.round(minutes / 60)
  if (hours < 48) {
    return `${hours}h ago`
  }

  const days = Math.round(hours / 24)
  return `${days}d ago`
}

export function effectiveSite(record: SemanticRecord): string {
  return record.site ?? record.facility ?? 'Unmapped site'
}

export function effectiveArea(record: SemanticRecord): string {
  return record.area ?? record.zone ?? 'Unmapped area'
}

export function effectiveWorkCenter(record: SemanticRecord): string {
  return record.work_center ?? record.cell ?? 'Work center'
}

export function effectiveWorkUnit(record: SemanticRecord): string {
  return record.work_unit ?? record.process ?? 'Work unit'
}

export function effectiveLeaf(record: SemanticRecord): string {
  return record.function ?? record.class ?? 'Device'
}

export function formatNodeKind(value?: string | null): string {
  if (!value) {
    return 'Device'
  }
  return nodeKindLabels[value] ?? value
}

export function formatHardwareIdentityKind(value?: string | null): string {
  if (!value) {
    return 'Hardware ID'
  }
  return hardwareIdentityLabels[value] ?? value
}

export function formatApplicationIdentityKind(value?: string | null): string {
  if (!value) {
    return 'Application ID'
  }
  return applicationIdentityLabels[value] ?? value.toUpperCase()
}

export function recordConfidence(record: SemanticRecord): string {
  const sources = new Set(Object.values(record.field_sources).map((field) => field.source))

  if (sources.has('manual-api')) {
    return 'authoritative'
  }
  if (sources.has('protocol-analysis') || sources.has('switch-intelligence')) {
    return 'high'
  }
  if (sources.has('dhcp-fingerprint') || sources.has('replacement-inference')) {
    return 'medium'
  }
  return 'low'
}

export function compactId(value: string, size = 8): string {
  if (value.length <= size * 2) {
    return value
  }
  return `${value.slice(0, size)}…${value.slice(-size)}`
}

export function countDistinct(values: string[]): number {
  return new Set(values.filter(Boolean)).size
}
