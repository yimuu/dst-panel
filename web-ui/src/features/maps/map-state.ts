import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export function applyPreinstall(name: string): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(
    `/api/game/preinstall?${new URLSearchParams({ name }).toString()}`,
  )
}
