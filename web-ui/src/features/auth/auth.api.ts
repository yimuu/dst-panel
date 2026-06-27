import { api } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { CurrentUser } from '@/shared/types/domain'

export interface LoginPayload {
  username: string
  password: string
}

export interface InitPayload {
  userInfo: {
    username: string
    password: string
    displayName?: string
    photoURL?: string
  }
  dstConfig?: Record<string, unknown>
}

export interface UpdateUserPayload {
  username: string
  displayName: string
  photoURL: string
  password: string
}

export async function getInitStatus(): Promise<ApiEnvelope<unknown>> {
  const response = await api.get<ApiEnvelope<unknown>>('/api/init')
  return response.data
}

export async function initFirstRun(payload: InitPayload): Promise<ApiEnvelope<unknown>> {
  const response = await api.post<ApiEnvelope<unknown>>('/api/init', payload)
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

export async function updateCurrentUser(payload: UpdateUserPayload): Promise<ApiEnvelope<unknown>> {
  const response = await api.post<ApiEnvelope<unknown>>('/api/user', payload)
  return response.data
}

export async function changePassword(newPassword: string): Promise<ApiEnvelope<unknown>> {
  const response = await api.post<ApiEnvelope<unknown>>('/api/change/password', {
    new_password: newPassword,
  })
  return response.data
}
