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

export interface DateRangeParams {
  startDate?: string
  endDate?: string
}

export interface ActiveUserParams extends DateRangeParams {
  unit?: string
}

export interface TopStatisticsParams extends DateRangeParams {
  N?: number | string
}

export interface RegenerateStatistics {
  id?: number
  day?: string
  count?: number
  [key: string]: unknown
}

export interface RegenerateStatisticsParams {
  N?: number | string
}

export function getActiveUsers(
  params?: ActiveUserParams,
): Promise<ApiEnvelope<ActiveUserStatistics>> {
  return apiGet('/api/statistics/active/user', { params })
}

export function getTopActive(
  params?: TopStatisticsParams,
): Promise<ApiEnvelope<TopActiveStatistics[]>> {
  return apiGet('/api/statistics/top/active', { params })
}

export function getRoleRate(params?: DateRangeParams): Promise<ApiEnvelope<RoleRateStatistics[]>> {
  return apiGet('/api/statistics/rate/role', { params })
}

export function regenerateStatistics(
  params?: RegenerateStatisticsParams,
): Promise<ApiEnvelope<RegenerateStatistics[]>> {
  return apiGet('/api/statistics/regenerate', { params })
}
