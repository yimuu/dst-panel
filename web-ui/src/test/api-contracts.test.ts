import { describe, expect, it } from 'vitest'

import { apiContracts } from '@/features/contracts/api-contracts'

describe('API contract inventory', () => {
  it('documents more than twenty API contracts', () => {
    expect(apiContracts.length).toBeGreaterThan(20)
  })

  it('keeps every path under the Rust API prefix', () => {
    expect(apiContracts.every(({ path }) => path.startsWith('/api/'))).toBe(true)
  })

  it('includes representative endpoints from each major area', () => {
    const endpoints = apiContracts.map(({ method, path }) => ({ method, path }))

    expect(endpoints).toContainEqual({ method: 'POST', path: '/api/login' })
    expect(endpoints).toContainEqual({ method: 'GET', path: '/api/cluster' })
    expect(endpoints).toContainEqual({ method: 'GET', path: '/api/cluster/level' })
    expect(endpoints).toContainEqual({ method: 'GET', path: '/api/game/8level/status' })
    expect(endpoints).toContainEqual({ method: 'GET', path: '/api/game/backup' })
    expect(endpoints).toContainEqual({ method: 'GET', path: '/api/mod' })
    expect(endpoints).toContainEqual({ method: 'GET', path: '/api/dst/config' })
    expect(endpoints).toContainEqual({ method: 'GET', path: '/api/statistics/active/user' })
  })

  it('does not duplicate method and path pairs', () => {
    const pairs = apiContracts.map(({ method, path }) => `${method} ${path}`)

    expect(new Set(pairs).size).toBe(pairs.length)
  })

  it('does not include stale generic paths from early planning', () => {
    const paths = apiContracts.map(({ path }) => path)

    expect(paths).not.toContain('/api/backup')
    expect(paths).not.toContain('/api/setting')
    expect(paths).not.toContain('/api/statistics')
  })
})
