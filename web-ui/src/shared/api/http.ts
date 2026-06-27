import axios, { type AxiosRequestConfig } from 'axios'

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
