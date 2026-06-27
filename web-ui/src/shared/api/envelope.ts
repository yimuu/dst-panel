import type { ApiEnvelope } from './types'

export function isApiSuccess(response: ApiEnvelope<unknown>): boolean {
  return response.code === 200 || response.code === 0
}

export function readApiData<T>(response: ApiEnvelope<T>, fallbackMessage = '操作失败'): T {
  if (!isApiSuccess(response)) {
    throw new Error(getErrorMessage(response, fallbackMessage))
  }

  return response.data as T
}

export function assertApiSuccess<T>(response: ApiEnvelope<T>): T {
  if (!isApiSuccess(response)) {
    throw new Error(getErrorMessage(response))
  }

  return readApiData(response)
}

export function getErrorMessage(error: unknown, fallback = '请求失败'): string {
  if (typeof error === 'string' && error.trim()) {
    return error
  }

  if (typeof error === 'object' && error !== null) {
    const record = error as Record<string, unknown>
    const response = record.response as { data?: Record<string, unknown> } | undefined
    const data = response?.data ?? record
    const msg = data.msg
    const message = data.message

    if (typeof msg === 'string' && msg.trim()) {
      return msg
    }

    if (typeof message === 'string' && message.trim()) {
      return message
    }
  }

  return fallback
}
