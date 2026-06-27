import { apiGet } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'

export interface DateRangeQuery {
  startDate?: string
  endDate?: string
}

export interface ActiveUserQuery extends DateRangeQuery {
  unit?: string
}

export interface TopStatisticsQuery extends DateRangeQuery {
  limit?: number | string
}

export interface LimitQuery {
  limit?: number | string
}

export interface ActiveUserAxis {
  x: number[] | null
  y1: number[] | null
  y2: number[] | null
}

export interface RoleRateStatistics {
  role: string
  count: number
}

export interface TopStatistics {
  id: number
  count: number
  name: string
  kuId: string
  steamId: string
  role: string
  actionDesc: string
  createdAt: string
}

export interface RegenerateRecord {
  ID: number
  CreatedAt: string | null
  UpdatedAt: string | null
  DeletedAt: string | null
  clusterName: string
}

export function getActiveUsers(query: ActiveUserQuery = {}): Promise<ApiEnvelope<ActiveUserAxis>> {
  return apiGet<ApiEnvelope<ActiveUserAxis>>(
    withQuery('/api/statistics/active/user', {
      unit: query.unit,
      startDate: query.startDate,
      endDate: query.endDate,
    }),
  )
}

export function getRoleRates(
  query: DateRangeQuery = {},
): Promise<ApiEnvelope<RoleRateStatistics[]>> {
  return apiGet<ApiEnvelope<RoleRateStatistics[]>>(
    withQuery('/api/statistics/rate/role', {
      startDate: query.startDate,
      endDate: query.endDate,
    }),
  )
}

export function getTopActiveUsers(
  query: TopStatisticsQuery = {},
): Promise<ApiEnvelope<TopStatistics[]>> {
  return apiGet<ApiEnvelope<TopStatistics[]>>(
    withQuery('/api/statistics/top/active', {
      N: query.limit,
      startDate: query.startDate,
      endDate: query.endDate,
    }),
  )
}

export function regenerateStatistics(
  query: LimitQuery = {},
): Promise<ApiEnvelope<RegenerateRecord[]>> {
  return apiGet<ApiEnvelope<RegenerateRecord[]>>(
    withQuery('/api/statistics/regenerate', { N: query.limit }),
  )
}

function withQuery(path: string, params: Record<string, string | number | undefined>): string {
  const search = new URLSearchParams()

  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined) {
      search.set(key, String(value))
    }
  }

  const query = search.toString()
  return query ? `${path}?${query}` : path
}
