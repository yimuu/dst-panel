import { ProCard } from '@ant-design/pro-components'

interface PlayerListPageProps {
  title?: string
}

export default function PlayerListPage({ title = '玩家列表' }: PlayerListPageProps) {
  return (
    <ProCard title={title} bordered={false}>
      名单数据加载中
    </ProCard>
  )
}
