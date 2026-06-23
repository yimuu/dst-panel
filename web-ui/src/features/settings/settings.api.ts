import { apiDelete, apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { TaskSummary } from '@/shared/types/domain'

export type SettingsPayload = Record<string, unknown>
export type TaskPayload = Partial<TaskSummary> & Record<string, unknown>
export type AutoCheckPayload = Record<string, unknown>

export function getDstConfig(): Promise<ApiEnvelope<Record<string, unknown>>> {
  return apiGet('/api/dst/config')
}

export function saveDstConfig(
  payload: SettingsPayload,
): Promise<ApiEnvelope<Record<string, unknown> | null>> {
  return apiPost('/api/dst/config', payload)
}

export function listTasks(): Promise<ApiEnvelope<TaskSummary[]>> {
  return apiGet('/api/task')
}

export function saveTask(payload: TaskPayload): Promise<ApiEnvelope<TaskSummary>> {
  return apiPost('/api/task', payload)
}

export function deleteTask(id: number | string): Promise<ApiEnvelope<null>> {
  return apiDelete('/api/task', { params: { id } })
}

export function getAutoCheck(): Promise<ApiEnvelope<Record<string, unknown>>> {
  return apiGet('/api/auto/check2')
}

export function saveAutoCheck(
  payload: AutoCheckPayload,
): Promise<ApiEnvelope<Record<string, unknown> | null>> {
  return apiPost('/api/auto/check2', payload)
}
