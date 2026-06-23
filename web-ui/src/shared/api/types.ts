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
  data?: T[]
  records?: T[]
  list?: T[]
  total?: number
  totalPages?: number
  totalPage?: number
  page?: number
  size?: number
  pages?: number
  current?: number
  pageSize?: number
}
