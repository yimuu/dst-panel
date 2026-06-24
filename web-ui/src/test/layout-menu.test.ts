import { describe, expect, it } from 'vitest'

import { adminMenuItems } from '@/layouts/menu'

describe('admin menu', () => {
  it('contains the core admin route paths', () => {
    const paths = adminMenuItems.map((item) => item.path)

    expect(paths).toContain('/panel')
    expect(paths).toContain('/levels/levels')
    expect(paths).toContain('/mod')
    expect(paths).toContain('/backup')
    expect(paths).toContain('/setting')
  })
})
