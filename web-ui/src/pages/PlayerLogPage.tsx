import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Button, Input, Popconfirm, Space, Table, Tag } from 'antd'
import { DeleteOutlined, ReloadOutlined, SearchOutlined } from '@ant-design/icons'
import { useEffect, useState } from 'react'

import {
  deletePlayerLogs,
  getPlayerLogs,
  type PlayerLogRecord,
} from '@/features/player-logs/player-log.api'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

export default function PlayerLogPage() {
  const { message } = AntApp.useApp()
  const [loading, setLoading] = useState(true)
  const [logs, setLogs] = useState<PlayerLogRecord[]>([])
  const [selectedIds, setSelectedIds] = useState<number[]>([])
  const [name, setName] = useState('')

  async function loadLogs() {
    try {
      setLoading(true)
      const page = assertApiSuccess(await getPlayerLogs({ name }))
      setLogs(page.data ?? [])
    } catch (error) {
      message.error(getErrorMessage(error, '加载玩家日志失败'))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadLogs()
  }, [])

  async function handleDelete() {
    if (selectedIds.length === 0) {
      message.warning('请选择日志')
      return
    }

    try {
      assertApiSuccess(await deletePlayerLogs(selectedIds))
      setSelectedIds([])
      await loadLogs()
    } catch (error) {
      message.error(getErrorMessage(error, '删除玩家日志失败'))
    }
  }

  return (
    <ProCard title="玩家记录" className="data-page-card" bordered={false}>
      <Space className="data-toolbar" wrap>
        <Input
          allowClear
          placeholder="玩家名称"
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
        <Button type="primary" icon={<SearchOutlined />} onClick={() => void loadLogs()}>
          查询
        </Button>
        <Button icon={<ReloadOutlined />} onClick={() => void loadLogs()}>
          刷新
        </Button>
        <Popconfirm
          title="确认删除选中的玩家日志"
          description="删除后无法从面板恢复。"
          okText="确认"
          cancelText="取消"
          disabled={selectedIds.length === 0}
          onConfirm={() => void handleDelete()}
        >
          <Button
            danger
            icon={<DeleteOutlined />}
            onClick={() => {
              if (selectedIds.length === 0) {
                message.warning('请选择日志')
              }
            }}
          >
            删除
          </Button>
        </Popconfirm>
      </Space>
      <Table
        rowKey={(row) => row.ID ?? row.id ?? `${row.name}-${row.createdAt}`}
        loading={loading}
        dataSource={logs}
        rowSelection={{
          selectedRowKeys: selectedIds,
          onChange: (keys) => setSelectedIds(keys.map(Number).filter(Number.isFinite)),
        }}
        pagination={{ pageSize: 10 }}
        columns={[
          { title: '玩家', dataIndex: 'name' },
          { title: 'KU ID', dataIndex: 'kuId', render: (value, row) => value ?? row.ku_id ?? '-' },
          { title: '角色', dataIndex: 'role' },
          {
            title: '动作',
            dataIndex: 'action',
            render: (value) => (
              <Tag color={value === '进入游戏' ? 'success' : 'default'}>{value}</Tag>
            ),
          },
          { title: 'IP', dataIndex: 'ip' },
          {
            title: '时间',
            dataIndex: 'createdAt',
            render: (value, row) => value ?? row.created_at ?? '-',
          },
        ]}
      />
    </ProCard>
  )
}
