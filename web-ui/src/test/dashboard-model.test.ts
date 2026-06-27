import { describe, expect, it } from 'vitest'

import { buildDashboardSummaryCards } from '@/features/dashboard/dashboard-model'

describe('dashboard model', () => {
  it('builds official summary cards', () => {
    expect(buildDashboardSummaryCards({ todayOnline: 1, monthOnline: 2 })).toEqual([
      { title: '今日在线人数', value: 1, color: '#1677ff' },
      { title: '本月在线人数', value: 2, color: '#f5c542' },
    ])
  })
})
