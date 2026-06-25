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
    const paths = apiContracts.map(({ path }) => path)

    expect(paths).toContain('/api/login')
    expect(paths).toContain('/api/game/8level/status')
    expect(paths).toContain('/api/cluster/level')
    expect(paths).toContain('/api/game/backup')
    expect(paths).toContain('/api/dst/config')
    expect(paths).toContain('/api/statistics/active/user')
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
