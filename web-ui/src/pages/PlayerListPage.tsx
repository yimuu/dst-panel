import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Button, Input, Popconfirm, Space, Table, Tag } from 'antd'
import { DeleteOutlined, ReloadOutlined, UserAddOutlined } from '@ant-design/icons'
import { useCallback, useEffect, useMemo, useState } from 'react'

import { getPlayerListTitle, type PlayerListKind } from '@/features/room/player-lists'
import {
  addPlayerListEntries,
  getPlayerList,
  removePlayerListEntries,
  savePlayerList,
} from '@/features/room/room.api'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

interface PlayerEntry {
  key: string
  kuId: string
}

interface PlayerListPageProps {
  kind: PlayerListKind
}

export default function PlayerListPage({ kind }: PlayerListPageProps) {
  const title = getPlayerListTitle(kind)
  const { message } = AntApp.useApp()
  const [entries, setEntries] = useState<string[]>([])
  const [draft, setDraft] = useState('')
  const [loading, setLoading] = useState(false)
  const rows = useMemo<PlayerEntry[]>(() => entries.map((kuId) => ({ key: kuId, kuId })), [entries])

  const loadEntries = useCallback(async () => {
    try {
      setLoading(true)
      const values = assertApiSuccess(await getPlayerList(kind))
      setEntries(values)
    } catch (error) {
      message.error(getErrorMessage(error, '加载名单失败'))
    } finally {
      setLoading(false)
    }
  }, [kind, message])

  useEffect(() => {
    void loadEntries()
  }, [loadEntries])

  async function addEntry(value: string) {
    const kuId = value.trim()
    if (!kuId || entries.includes(kuId)) {
      return
    }
    const nextEntries = kind === 'whitelist' ? [kuId, ...entries] : [...entries, kuId]
    try {
      if (kind === 'whitelist') {
        assertApiSuccess(await savePlayerList(kind, nextEntries))
      } else {
        assertApiSuccess(await addPlayerListEntries(kind, [kuId]))
      }
      setEntries(nextEntries)
      setDraft('')
      message.success('添加成功')
    } catch (error) {
      message.error(getErrorMessage(error, '添加失败'))
    }
  }

  async function removeEntry(kuId: string) {
    const nextEntries = entries.filter((entry) => entry !== kuId)
    try {
      if (kind === 'whitelist') {
        assertApiSuccess(await savePlayerList(kind, nextEntries))
      } else {
        assertApiSuccess(await removePlayerListEntries(kind, [kuId]))
      }
      setEntries(nextEntries)
      message.success('删除成功')
    } catch (error) {
      message.error(getErrorMessage(error, '删除失败'))
    }
  }

  return (
    <div className="player-list-page">
      <ProCard
        title={title}
        bordered={false}
        extra={
          <Button icon={<ReloadOutlined />} loading={loading} onClick={() => void loadEntries()}>
            刷新
          </Button>
        }
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
          loading={loading}
          pagination={false}
          dataSource={rows}
          columns={[
            { title: 'KU ID', dataIndex: 'kuId' },
            {
              title: '操作',
              width: 140,
              render: (_, row) => (
                <Popconfirm
                  title={`确认删除 ${row.kuId}`}
                  description={`将从${title}中移除该玩家。`}
                  okText="确认"
                  cancelText="取消"
                  onConfirm={() => void removeEntry(row.kuId)}
                >
                  <Button danger icon={<DeleteOutlined />}>
                    删除
                  </Button>
                </Popconfirm>
              ),
            },
          ]}
        />
      </ProCard>
    </div>
  )
}
