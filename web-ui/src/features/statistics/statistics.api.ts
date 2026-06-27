import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export type StatisticsUnit = 'week' | 'month' | 'day'

export interface ActiveUserPoint {
  date?: string
  active?: number
  login?: number
  [key: string]: unknown
}

export interface RoleRateItem {
  role?: string
  value?: number
  [key: string]: unknown
}

export interface TopActiveUser {
  name?: string
  value?: number
  [key: string]: unknown
}

export function getActiveUsers(unit: StatisticsUnit): Promise<ApiEnvelope<ActiveUserPoint[]>> {
  return apiGet<ApiEnvelope<ActiveUserPoint[]>>(
    `/api/statistics/active/user/?unit=${encodeURIComponent(unit)}`,
  )
}

export function getRoleRates(startDate: string): Promise<ApiEnvelope<RoleRateItem[]>> {
  return apiGet<ApiEnvelope<RoleRateItem[]>>(
    `/api/statistics/rate/role/?&startDate=${encodeURIComponent(startDate)}`,
  )
}

export function getTopActiveUsers(limit: number): Promise<ApiEnvelope<TopActiveUser[]>> {
  return apiGet<ApiEnvelope<TopActiveUser[]>>(`/api/statistics/top/active/?N=${limit}`)
}

export function regenerateStatistics(limit: number): Promise<ApiEnvelope<unknown>> {
  return apiGet<ApiEnvelope<unknown>>(`/api/statistics/regenerate?N=${limit}`)
}
