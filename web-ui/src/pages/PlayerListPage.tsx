import { ProCard } from '@ant-design/pro-components'
import { Button, Input, Space, Table, Tag } from 'antd'
import { DeleteOutlined, ReloadOutlined, UserAddOutlined } from '@ant-design/icons'
import { useMemo, useState } from 'react'

import { getPlayerListTitle, type PlayerListKind } from '@/features/room/player-lists'

const initialEntries: Record<PlayerListKind, string[]> = {
  adminlist: ['KU_admin_001', 'KU_admin_002'],
  whitelist: ['KU_white_001', 'KU_white_002'],
  blacklist: ['KU_black_001'],
}

interface PlayerEntry {
  key: string
  kuId: string
}

interface PlayerListPageProps {
  kind: PlayerListKind
}

export default function PlayerListPage({ kind }: PlayerListPageProps) {
  const title = getPlayerListTitle(kind)
  const [entries, setEntries] = useState(() => initialEntries[kind])
  const [draft, setDraft] = useState('')
  const rows = useMemo<PlayerEntry[]>(() => entries.map((kuId) => ({ key: kuId, kuId })), [entries])

  function addEntry(value: string) {
    const kuId = value.trim()
    if (!kuId || entries.includes(kuId)) {
      return
    }
    setEntries((current) => [kuId, ...current])
    setDraft('')
  }

  return (
    <div className="player-list-page">
      <ProCard
        title={title}
        bordered={false}
        extra={<Button icon={<ReloadOutlined />}>刷新</Button>}
      >
        <Space className="player-list-toolbar" wrap>
          <Input.Search
            allowClear
            enterButton={
              <Button type="primary" icon={<UserAddOutlined />}>
                添加
              </Button>
            }
            placeholder="输入 KU ID"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            onSearch={addEntry}
          />
          <Tag color="processing">{entries.length}</Tag>
        </Space>
        <Table
          rowKey="key"
          pagination={false}
          dataSource={rows}
          columns={[
            { title: 'KU ID', dataIndex: 'kuId' },
            {
              title: '操作',
              width: 140,
              render: (_, row) => (
                <Button
                  danger
                  icon={<DeleteOutlined />}
                  onClick={() =>
                    setEntries((current) => current.filter((kuId) => kuId !== row.kuId))
                  }
                >
                  删除
                </Button>
              ),
            },
          ]}
        />
      </ProCard>
    </div>
  )
}
