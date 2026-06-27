import axios, { type AxiosRequestConfig } from 'axios'

import type { ApiEnvelope, ApiError } from './types'

export const api = axios.create({
  baseURL: '/',
  withCredentials: true,
})

export const http = api

export function setClusterHeader(cluster: string | undefined): void {
  if (cluster) {
    api.defaults.headers.common.Cluster = cluster
    return
  }

  delete api.defaults.headers.common.Cluster
}

export function isApiSuccess(envelope: Pick<ApiEnvelope<unknown>, 'code'>): boolean {
  return envelope.code === 0 || envelope.code === 200
}

export function normalizeApiError(error: unknown): ApiError {
  const response = extractErrorResponse(error)
  const data = response?.data
  const status = response?.status
  const code = readNumber(data, 'code')
  const message =
    readString(data, 'msg') ??
    readString(data, 'message') ??
    readString(error, 'message') ??
    '请求失败'

  return {
    ...(status === undefined ? {} : { status }),
    ...(code === undefined ? {} : { code }),
    message,
    ...(data === undefined ? {} : { details: data }),
  }
}

export async function apiGet<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
  const response = await api.get<T>(url, config)
  return response.data
}

export async function apiPost<T, P = unknown>(
  url: string,
  payload?: P,
  config?: AxiosRequestConfig,
): Promise<T> {
  const response = await api.post<T>(url, payload, config)
  return response.data
}

export async function apiPut<T, P = unknown>(
  url: string,
  payload?: P,
  config?: AxiosRequestConfig,
): Promise<T> {
  const response = await api.put<T>(url, payload, config)
  return response.data
}

export async function apiDelete<T>(url: string, config?: AxiosRequestConfig): Promise<T> {
  const response = await api.delete<T>(url, config)
  return response.data
}

interface ErrorResponse {
  status?: number
  data?: unknown
}

function extractErrorResponse(error: unknown): ErrorResponse | undefined {
  if (axios.isAxiosError(error)) {
    return error.response
  }

  if (!isRecord(error)) {
    return undefined
  }

  const response = error.response
  if (!isRecord(response)) {
    return undefined
  }

  return {
    status: readNumber(response, 'status'),
    data: response.data,
  }
}

function readString(value: unknown, key: string): string | undefined {
  if (!isRecord(value)) {
    return undefined
  }

  const field = value[key]
  return typeof field === 'string' ? field : undefined
}

function readNumber(value: unknown, key: string): number | undefined {
  if (!isRecord(value)) {
    return undefined
  }

  const field = value[key]
  return typeof field === 'number' ? field : undefined
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}
