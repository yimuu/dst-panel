export interface DashboardSummaryInput {
  todayOnline: number
  monthOnline: number
}

export interface DashboardSummaryCard {
  title: string
  value: number
  color: string
}

export function buildDashboardSummaryCards(input: DashboardSummaryInput): DashboardSummaryCard[] {
  return [
    { title: '今日在线人数', value: input.todayOnline, color: '#1677ff' },
    { title: '本月在线人数', value: input.monthOnline, color: '#f5c542' },
  ]
}
