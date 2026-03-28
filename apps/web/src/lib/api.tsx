/* eslint-disable react-refresh/only-export-components */

import {
  type ReactNode,
  createContext,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react'
import {
  type UseMutationResult,
  type UseQueryResult,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import type { ZodType } from 'zod'
import {
  type ApplicationIdentity,
  AuditEventRecordSchema,
  type AuditEventRecord,
  FingerprintRuleSchema,
  type FingerprintRule,
  HealthSchema,
  type HardwareIdentity,
  type HealthResponse,
  QuarantineEntrySchema,
  type QuarantineEntry,
  RoleTemplateSchema,
  type RoleTemplate,
  SemanticRecordSchema,
  type SemanticRecord,
  type SemanticRelation,
  SyncStatusSchema,
  type SyncStatus,
} from '../types'

const API_SETTINGS_KEY = 'semantic-dns-api-settings'

export type ApiSettings = {
  baseUrl: string
  token: string
}

export type CreateObservationInput = {
  id: string
  device_id: string
  observed_at: string
  source: 'manual-api'
  node_kind: string
  external_ip?: string | null
  internal_ip?: string | null
  class?: string | null
  vendor?: string | null
  model?: string | null
  protocols?: string[] | null
  mac?: string | null
  switch_port?: string | null
  enterprise?: string | null
  site?: string | null
  area?: string | null
  work_center?: string | null
  work_center_kind?: string | null
  work_unit?: string | null
  facility?: string | null
  zone?: string | null
  cell?: string | null
  process?: string | null
  function?: string | null
  hardware_identities?: HardwareIdentity[] | null
  application_identities?: ApplicationIdentity[] | null
  aliases?: string[] | null
  relations?: SemanticRelation[] | null
  status?: string | null
}

type ApiSettingsContextValue = {
  settings: ApiSettings
  updateSettings: (next: ApiSettings) => void
}

class ApiResponseError extends Error {
  status: number

  constructor(message: string, status: number) {
    super(message)
    this.name = 'ApiResponseError'
    this.status = status
  }
}

const defaultSettings: ApiSettings = {
  baseUrl: import.meta.env.VITE_API_BASE_URL ?? '',
  token: import.meta.env.VITE_API_TOKEN ?? 'semantic-admin-token',
}

const ApiSettingsContext = createContext<ApiSettingsContextValue | null>(null)

function parseStoredSettings(): ApiSettings {
  if (typeof window === 'undefined') {
    return defaultSettings
  }

  const raw = window.localStorage.getItem(API_SETTINGS_KEY)
  if (!raw) {
    return defaultSettings
  }

  try {
    const parsed = JSON.parse(raw) as Partial<ApiSettings>
    return {
      baseUrl: parsed.baseUrl ?? defaultSettings.baseUrl,
      token: parsed.token ?? defaultSettings.token,
    }
  } catch {
    return defaultSettings
  }
}

export function ApiSettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<ApiSettings>(() => parseStoredSettings())

  useEffect(() => {
    if (typeof window === 'undefined') {
      return
    }

    window.localStorage.setItem(API_SETTINGS_KEY, JSON.stringify(settings))
  }, [settings])

  const value = useMemo<ApiSettingsContextValue>(
    () => ({
      settings,
      updateSettings: setSettings,
    }),
    [settings],
  )

  return (
    <ApiSettingsContext.Provider value={value}>
      {children}
    </ApiSettingsContext.Provider>
  )
}

export function useApiSettings(): ApiSettingsContextValue {
  const context = useContext(ApiSettingsContext)
  if (!context) {
    throw new Error('useApiSettings must be used inside ApiSettingsProvider')
  }

  return context
}

function buildUrl(baseUrl: string, path: string): string {
  const normalizedBase = baseUrl.trim().replace(/\/$/, '')
  return normalizedBase ? `${normalizedBase}${path}` : path
}

async function requestJson<T>({
  settings,
  path,
  schema,
  auth = true,
  init,
}: {
  settings: ApiSettings
  path: string
  schema: ZodType<T>
  auth?: boolean
  init?: RequestInit
}): Promise<T> {
  const headers = new Headers(init?.headers)
  if (auth) {
    headers.set('Authorization', `Bearer ${settings.token}`)
  }
  headers.set('Accept', 'application/json')

  const response = await fetch(buildUrl(settings.baseUrl, path), {
    ...init,
    headers,
  })

  if (!response.ok) {
    let message = `Request failed with status ${response.status}`
    try {
      const json = (await response.json()) as { error?: { message?: string } }
      message = json.error?.message ?? message
    } catch {
      // Ignore parse failures for non-JSON error bodies.
    }
    throw new ApiResponseError(message, response.status)
  }

  const payload = await response.json()
  return schema.parse(payload)
}

export function useHealthQuery(): UseQueryResult<HealthResponse> {
  const { settings } = useApiSettings()
  return useQuery({
    queryKey: ['health', settings.baseUrl],
    queryFn: () =>
      requestJson({
        settings,
        path: '/api/v1/health',
        schema: HealthSchema,
        auth: false,
      }),
    refetchInterval: 15_000,
  })
}

export function useRecordsQuery(): UseQueryResult<SemanticRecord[]> {
  const { settings } = useApiSettings()
  return useQuery({
    queryKey: ['records', settings.baseUrl, settings.token],
    queryFn: () =>
      requestJson({
        settings,
        path: '/api/v1/dns/query',
        schema: SemanticRecordSchema.array(),
      }),
  })
}

export function useSyncStatusQuery(): UseQueryResult<SyncStatus> {
  const { settings } = useApiSettings()
  return useQuery({
    queryKey: ['sync-status', settings.baseUrl, settings.token],
    queryFn: () =>
      requestJson({
        settings,
        path: '/api/v1/dhcp/dns/sync-status',
        schema: SyncStatusSchema,
      }),
  })
}

export function useFingerprintsQuery(): UseQueryResult<FingerprintRule[]> {
  const { settings } = useApiSettings()
  return useQuery({
    queryKey: ['fingerprints', settings.baseUrl, settings.token],
    queryFn: () =>
      requestJson({
        settings,
        path: '/api/v1/dhcp/fingerprints',
        schema: FingerprintRuleSchema.array(),
      }),
  })
}

export function useTemplatesQuery(): UseQueryResult<RoleTemplate[]> {
  const { settings } = useApiSettings()
  return useQuery({
    queryKey: ['templates', settings.baseUrl, settings.token],
    queryFn: () =>
      requestJson({
        settings,
        path: '/api/v1/dhcp/templates',
        schema: RoleTemplateSchema.array(),
      }),
  })
}

export function useQuarantineQuery(): UseQueryResult<QuarantineEntry[]> {
  const { settings } = useApiSettings()
  return useQuery({
    queryKey: ['quarantine', settings.baseUrl, settings.token],
    queryFn: () =>
      requestJson({
        settings,
        path: '/api/v1/dhcp/quarantine',
        schema: QuarantineEntrySchema.array(),
      }),
  })
}

export function useAuditEventsQuery(limit = 150): UseQueryResult<AuditEventRecord[]> {
  const { settings } = useApiSettings()
  return useQuery({
    queryKey: ['audit', settings.baseUrl, settings.token, limit],
    queryFn: () =>
      requestJson({
        settings,
        path: `/api/v1/audit/events?limit=${limit}`,
        schema: AuditEventRecordSchema.array(),
      }),
  })
}

export function useCreateObservationMutation(): UseMutationResult<SemanticRecord, Error, CreateObservationInput> {
  const { settings } = useApiSettings()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: (observation) =>
      requestJson({
        settings,
        path: '/api/v1/observations',
        schema: SemanticRecordSchema,
        init: {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify(observation),
        },
      }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['records'] }),
        queryClient.invalidateQueries({ queryKey: ['health'] }),
        queryClient.invalidateQueries({ queryKey: ['sync-status'] }),
        queryClient.invalidateQueries({ queryKey: ['audit'] }),
      ])
    },
  })
}

export function useReconcileMutation(): UseMutationResult<SyncStatus, Error, void> {
  const { settings } = useApiSettings()
  const queryClient = useQueryClient()

  return useMutation({
    mutationFn: () =>
      requestJson({
        settings,
        path: '/api/v1/dhcp/dns/reconcile',
        schema: SyncStatusSchema,
        init: {
          method: 'POST',
        },
      }),
    onSuccess: async () => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['health'] }),
        queryClient.invalidateQueries({ queryKey: ['sync-status'] }),
        queryClient.invalidateQueries({ queryKey: ['audit'] }),
      ])
    },
  })
}
