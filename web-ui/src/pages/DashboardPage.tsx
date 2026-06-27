import { ProCard } from '@ant-design/pro-components'
import { DatePicker, Empty, Segmented, Statistic, Timeline } from 'antd'

import { buildDashboardSummaryCards } from '@/features/dashboard/dashboard-model'

const summaryCards = buildDashboardSummaryCards({ todayOnline: 1, monthOnline: 2 })
const activeDays = ['03-02', '03-03', '03-04', '03-05', '03-06', '03-07', '03-08']

export default function DashboardPage() {
  return (
    <div className="dashboard-page">
      <ProCard className="dashboard-toolbar" bordered={false}>
        <DatePicker.RangePicker />
        <Segmented options={['本周', '上周', '本月', '上月']} defaultValue="本周" />
      </ProCard>

      <div className="dashboard-summary-grid">
        {summaryCards.map((card) => (
          <ProCard key={card.title} bordered={false}>
            <Statistic title={card.title} value={card.value} valueStyle={{ color: card.color }} />
          </ProCard>
        ))}
      </div>

      <div className="dashboard-chart-grid">
        <ProCard title="本周玩家活跃情况" bordered={false}>
          <div className="activity-chart" aria-label="本周玩家活跃情况">
            <div className="chart-legend">
              <span className="legend-line">活跃玩家</span>
              <span className="legend-bar">加入玩家</span>
            </div>
            <div className="chart-plot">
              <div className="chart-y-axis">
                {[1, 0.8, 0.6, 0.4, 0.2, 0].map((tick) => (
                  <span key={tick}>{tick}</span>
                ))}
              </div>
              <div className="chart-bars">
                {activeDays.map((day, index) => (
                  <div key={day} className="chart-day">
                    <span className={index === 2 ? 'activity-bar active' : 'activity-bar'} />
                    <span
                      className={
                        index === 0 || index === 2 ? 'activity-dot active' : 'activity-dot'
                      }
                    />
                    <span className="chart-day-label">2026-{day}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </ProCard>

        <ProCard title="本周角色比例" bordered={false}>
          <div className="role-ratio-card">
            <div className="role-legend">
              <span />
              威尔逊
            </div>
            <div className="role-donut">
              <span>威尔逊</span>
            </div>
          </div>
        </ProCard>
      </div>

      <div className="dashboard-bottom-grid">
        <ProCard title="本周前十玩家排名" bordered={false}>
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无排行数据" />
        </ProCard>
        <ProCard title="重置时间线" bordered={false}>
          <Timeline items={[{ color: '#4f46e5', children: '2026/3/2 17:45:12' }]} />
        </ProCard>
      </div>
    </div>
  )
}
