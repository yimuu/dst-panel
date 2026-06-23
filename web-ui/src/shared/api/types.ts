export interface ApiEnvelope<T> {
  code: number
  msg?: string
  message?: string
  data: T
}

export interface ApiError {
  status?: number
  code?: number
  message: string
  details?: unknown
}

export interface PageResult<T> {
  records?: T[]
  list?: T[]
  total?: number
  page?: number
  size?: number
  pages?: number
  current?: number
  pageSize?: number
}
