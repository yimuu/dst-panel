import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export interface ActiveUserStatistics {
  x?: number[]
  y1?: number[]
  y2?: number[]
  [key: string]: unknown
}

export interface TopActiveStatistics {
  id?: number
  count?: number
  name?: string
  kuId?: string
  steamId?: string
  role?: string
  actionDesc?: string
  createdAt?: string
  [key: string]: unknown
}

export interface RoleRateStatistics {
  role?: string
  count?: number
  [key: string]: unknown
}

export function getActiveUsers(): Promise<ApiEnvelope<ActiveUserStatistics>> {
  return apiGet('/api/statistics/active/user')
}

export function getTopActive(): Promise<ApiEnvelope<TopActiveStatistics[]>> {
  return apiGet('/api/statistics/top/active')
}

export function getRoleRate(): Promise<ApiEnvelope<RoleRateStatistics[]>> {
  return apiGet('/api/statistics/rate/role')
}

export function regenerateStatistics(): Promise<ApiEnvelope<null>> {
  return apiGet('/api/statistics/regenerate')
}
