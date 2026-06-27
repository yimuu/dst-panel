import { describe, expect, it } from 'vitest'

import { api, setClusterHeader } from '@/shared/api/http'

describe('http client', () => {
  it('sets and clears the Cluster header', () => {
    setClusterHeader('Cluster_1')
    expect(api.defaults.headers.common.Cluster).toBe('Cluster_1')

    setClusterHeader(undefined)
    expect(api.defaults.headers.common.Cluster).toBeUndefined()
  })
})
