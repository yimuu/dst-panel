import { apiGet, apiPost } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import type { InitRequest, LoginRequest, UserProfile } from '@/shared/types/domain'

export function getInitStatus(): Promise<ApiEnvelope<boolean | Record<string, unknown>>> {
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

export function updateUser(
  payload: Partial<UserProfile> & Record<string, unknown>,
): Promise<ApiEnvelope<UserProfile>> {
  return apiPost('/api/user', payload)
}
