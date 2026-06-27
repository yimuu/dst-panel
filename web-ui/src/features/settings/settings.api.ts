import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { DstConfig } from '@/shared/types/domain'

export function getDstConfig(): Promise<ApiEnvelope<DstConfig>> {
  return apiGet<ApiEnvelope<DstConfig>>('/api/dst/config')
}

export function saveDstConfig(config: Record<string, unknown>): Promise<ApiEnvelope<unknown>> {
  return apiPost<ApiEnvelope<unknown>, Record<string, unknown>>('/api/dst/config', config)
}
