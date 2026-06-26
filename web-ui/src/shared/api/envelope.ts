import { isApiSuccess } from './http'
import type { ApiEnvelope } from './types'

export function readApiData<T>(response: ApiEnvelope<T>, fallbackMessage: string): T {
  if (!isApiSuccess(response)) {
    throw new Error(response.msg || response.message || fallbackMessage)
  }

  return response.data
}

export function assertApiSuccess(
  response: ApiEnvelope<unknown>,
  fallbackMessage = '操作失败',
): void {
  readApiData(response, fallbackMessage)
}

export function getErrorMessage(error: unknown, fallbackMessage: string): string {
  if (error instanceof Error && error.message) {
    return error.message
  }

  if (typeof error === 'string' && error.length > 0) {
    return error
  }

  return fallbackMessage
}
