import { ProCard } from '@ant-design/pro-components'
import { Button, Input, Space, Table, Tag } from 'antd'
import { SearchOutlined } from '@ant-design/icons'
import { useState } from 'react'

const lobbyRows = [
  {
    key: '1',
    name: '官方大厅数据',
    mode: '生存',
    players: '0/8',
    region: 'ap-east-1',
  },
]

export default function LobbyPage() {
  const [keyword, setKeyword] = useState('')

  return (
    <ProCard title="大厅列表" className="data-page-card" bordered={false}>
      <Space className="data-toolbar" wrap>
        <Input
          allowClear
          placeholder="房间名称"
          value={keyword}
          onChange={(event) => setKeyword(event.target.value)}
        />
        <Button type="primary" icon={<SearchOutlined />}>
          搜索大厅
        </Button>
        <Tag color="processing">Klei / dstserverlist 代理</Tag>
      </Space>
      <Table
        rowKey="key"
        pagination={false}
        dataSource={lobbyRows.filter((row) => !keyword || row.name.includes(keyword))}
        columns={[
          { title: '房间', dataIndex: 'name' },
          { title: '模式', dataIndex: 'mode' },
          { title: '玩家', dataIndex: 'players' },
          { title: '区域', dataIndex: 'region' },
        ]}
      />
    </ProCard>
  )
}
