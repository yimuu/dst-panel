import { api } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { CurrentUser } from '@/shared/types/domain'

export interface LoginPayload {
  username: string
  password: string
}

export async function getInitStatus(): Promise<ApiEnvelope<unknown>> {
  const response = await api.get<ApiEnvelope<unknown>>('/api/init')
  return response.data
}

export async function login(payload: LoginPayload): Promise<ApiEnvelope<CurrentUser>> {
  const response = await api.post<ApiEnvelope<CurrentUser>>('/api/login', payload)
  return response.data
}

export async function logout(): Promise<ApiEnvelope<unknown>> {
  const response = await api.post<ApiEnvelope<unknown>>('/api/logout')
  return response.data
}

export async function getCurrentUser(): Promise<ApiEnvelope<CurrentUser>> {
  const response = await api.get<ApiEnvelope<CurrentUser>>('/api/user')
  return response.data
}
