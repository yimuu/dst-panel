export interface ApiEnvelope<T = unknown> {
  code?: number
  data?: T
  msg?: string
  message?: string
  [key: string]: unknown
}
