import { z } from 'zod'

export const MetadataFieldSchema = z.object({
  value: z.string(),
  source: z.string(),
  updated_at: z.string(),
})

export const ApplicationIdentitySchema = z.object({
  kind: z.string(),
  value: z.string(),
  label: z.string().nullable().optional(),
})

export const HardwareIdentitySchema = z.object({
  kind: z.string(),
  value: z.string(),
  label: z.string().nullable().optional(),
})

export const SemanticRelationSchema = z.object({
  relation: z.string(),
  target: z.string(),
  label: z.string().nullable().optional(),
})

export const SemanticRecordSchema = z.object({
  device_id: z.string(),
  fqdn: z.string(),
  node_kind: z.string().default('device'),
  external_ip: z.string().nullable().optional(),
  internal_ip: z.string().nullable().optional(),
  class: z.string().nullable().optional(),
  vendor: z.string().nullable().optional(),
  model: z.string().nullable().optional(),
  protocols: z.array(z.string()),
  mac: z.string().nullable().optional(),
  switch_port: z.string().nullable().optional(),
  enterprise: z.string().nullable().optional(),
  site: z.string().nullable().optional(),
  area: z.string().nullable().optional(),
  work_center: z.string().nullable().optional(),
  work_center_kind: z.string().nullable().optional(),
  work_unit: z.string().nullable().optional(),
  facility: z.string().nullable().optional(),
  zone: z.string().nullable().optional(),
  cell: z.string().nullable().optional(),
  process: z.string().nullable().optional(),
  function: z.string().nullable().optional(),
  hardware_identities: z.array(HardwareIdentitySchema).default([]),
  application_identities: z.array(ApplicationIdentitySchema).default([]),
  aliases: z.array(z.string()).default([]),
  relations: z.array(SemanticRelationSchema).default([]),
  status: z.string(),
  updated_at: z.string(),
  field_sources: z.record(z.string(), MetadataFieldSchema),
})

export const SyncStatusSchema = z.object({
  total_leases: z.number(),
  dns_records_synced: z.number(),
  pending_updates: z.number(),
  failed_updates: z.number(),
  last_reconciliation: z.string().nullable().optional(),
})

export const HealthSchema = z.object({
  status: z.string(),
  sync_status: SyncStatusSchema,
})

export const FingerprintClassificationSchema = z.object({
  vendor: z.string(),
  class: z.string(),
  model_family: z.string(),
  confidence: z.number(),
  protocols: z.array(z.string()),
})

export const FingerprintRuleSchema = z.object({
  id: z.string(),
  name: z.string(),
  mac_oui: z.string().nullable().optional(),
  option_60_glob: z.string().nullable().optional(),
  option_55_order: z.array(z.number()).nullable().optional(),
  classification: FingerprintClassificationSchema,
})

export const RoleAssignmentSchema = z.object({
  role: z.string(),
  address: z.string(),
  class_match: z.string().nullable().optional(),
  vendor_match: z.string().nullable().optional(),
  function_match: z.string().nullable().optional(),
  work_unit_id: z.string().nullable().optional(),
  process_area: z.string(),
})

export const RoleTemplateSchema = z.object({
  id: z.string(),
  name: z.string(),
  site_id: z.string().nullable().optional(),
  area_id: z.string().nullable().optional(),
  work_center_id: z.string().nullable().optional(),
  work_center_kind: z.string().nullable().optional(),
  cell_id: z.string(),
  zone_suffix: z.string(),
  assignments: z.array(RoleAssignmentSchema),
  unassigned_range: z.array(z.string()),
  quarantine_range: z.array(z.string()),
})

export const QuarantineEntrySchema = z.object({
  id: z.string(),
  mac: z.string(),
  fingerprint_summary: z.string().nullable().optional(),
  switch_port: z.string().nullable().optional(),
  reason: z.string(),
  queued_at: z.string(),
})

export const AuditEventRecordSchema = z.object({
  id: z.number(),
  event_type: z.string(),
  payload: z.record(z.string(), z.unknown()).or(z.array(z.unknown())).or(z.unknown()),
  created_at: z.string(),
  previous_hash: z.string().nullable().optional(),
  current_hash: z.string(),
})

export type SemanticRecord = z.infer<typeof SemanticRecordSchema>
export type ApplicationIdentity = z.infer<typeof ApplicationIdentitySchema>
export type HardwareIdentity = z.infer<typeof HardwareIdentitySchema>
export type SemanticRelation = z.infer<typeof SemanticRelationSchema>
export type SyncStatus = z.infer<typeof SyncStatusSchema>
export type HealthResponse = z.infer<typeof HealthSchema>
export type FingerprintRule = z.infer<typeof FingerprintRuleSchema>
export type RoleTemplate = z.infer<typeof RoleTemplateSchema>
export type QuarantineEntry = z.infer<typeof QuarantineEntrySchema>
export type AuditEventRecord = z.infer<typeof AuditEventRecordSchema>
