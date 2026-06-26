import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { InitRequest, LoginRequest, UserProfile } from '@/shared/types/domain'

export interface UpdateUserRequest {
  username: string
  displayName: string
  photoURL: string
  password: string
}

export interface ChangePasswordRequest {
  newPassword: string
}

export function getInitStatus(): Promise<ApiEnvelope<boolean | Record<string, unknown> | null>> {
  return apiGet('/api/init')
}

export function initialize(payload: InitRequest): Promise<ApiEnvelope<UserProfile | null>> {
  return apiPost('/api/init', payload)
}

export function login(payload: LoginRequest): Promise<ApiEnvelope<UserProfile>> {
  return apiPost('/api/login', payload)
}

export function logout(): Promise<ApiEnvelope<null>> {
  return apiGet('/api/logout')
}

export function getUser(): Promise<ApiEnvelope<UserProfile>> {
  return apiGet('/api/user')
}

export function updateUser(payload: UpdateUserRequest): Promise<ApiEnvelope<null>> {
  return apiPost('/api/user', payload)
}

export function changePassword(payload: ChangePasswordRequest): Promise<ApiEnvelope<null>> {
  return apiPost('/api/change/password', payload)
}
