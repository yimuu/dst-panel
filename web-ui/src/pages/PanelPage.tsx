import { ProCard } from '@ant-design/pro-components'
import {
  App as AntApp,
  Button,
  Col,
  Empty,
  Input,
  InputNumber,
  Progress,
  Row,
  Select,
  Space,
  Spin,
  Statistic,
  Table,
  Tabs,
  Tag,
  Typography,
} from 'antd'
import {
  CloudDownloadOutlined,
  CloudUploadOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
  SaveOutlined,
  SendOutlined,
  SyncOutlined,
} from '@ant-design/icons'
import { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router'

import {
  createGameBackup,
  getAllOnlinePlayers,
  getLevelLogDownloadUrl,
  getLevelServerLog,
  getLevelStatus,
  getOnlinePlayers,
  getSystemInfo,
  regenerateWorld,
  rollbackGame,
  sendGameCommand,
  startLevel,
  stopLevel,
  updateGame,
  type LevelStatusInfo,
  type OnlinePlayer,
  type SystemInfo,
} from '@/features/game/game.api'
import { getPanelActionLabel } from '@/features/panel/panel-model'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'
import { routes } from '@/shared/config/routes'

type PanelTabKey = 'panel' | 'remote' | 'tooManyItemsPlus' | 'custom' | 'customEdit'

interface CommandPreset {
  name: string
  command: string
  danger?: boolean
}

interface CustomCommand extends CommandPreset {
  id: string
}

interface PendingConfirmation {
  title: string
  content: string
  action: () => Promise<unknown> | unknown
}

const COMMAND_HISTORY_KEY = 'dst-panel.commandHistory'
const CUSTOM_COMMAND_KEY = 'dst-panel.customCommands'

const builtinCommands: CommandPreset[] = [
  { name: '保存当前世界', command: 'c_save()' },
  { name: '公告即将维护', command: 'c_announce("服务器将在 5 分钟后维护")' },
  { name: '回档 1 天', command: 'c_rollback(1)' },
]

const itemPresets: CommandPreset[] = [
  { name: '木头', command: 'log' },
  { name: '草', command: 'cutgrass' },
  { name: '树枝', command: 'twigs' },
  { name: '石头', command: 'rocks' },
  { name: '金块', command: 'goldnugget' },
  { name: '火炬', command: 'torch' },
  { name: '背包', command: 'backpack' },
  { name: '齿轮', command: 'gears' },
]

export default function PanelPage() {
  const { message } = AntApp.useApp()
  const navigate = useNavigate()
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string>()
  const [activePanelTab, setActivePanelTab] = useState<PanelTabKey>('panel')
  const [levels, setLevels] = useState<LevelStatusInfo[]>([])
  const [systemInfo, setSystemInfo] = useState<SystemInfo>()
  const [activeLevel, setActiveLevel] = useState<string>()
  const [logLines, setLogLines] = useState<string[]>([])
  const [players, setPlayers] = useState<OnlinePlayer[]>([])
  const [command, setCommand] = useState('')
  const [remoteCommand, setRemoteCommand] = useState('')
  const [remoteMessage, setRemoteMessage] = useState('')
  const [commandHistory, setCommandHistory] = useState<string[]>(readCommandHistory)
  const [itemCode, setItemCode] = useState('')
  const [itemAmount, setItemAmount] = useState(1)
  const [itemTargetKuId, setItemTargetKuId] = useState<string>()
  const [customCommands, setCustomCommands] = useState<CustomCommand[]>(readCustomCommands)
  const [customName, setCustomName] = useState('')
  const [customCommand, setCustomCommand] = useState('')
  const [editingCommandId, setEditingCommandId] = useState<string>()
  const [pendingConfirmation, setPendingConfirmation] = useState<PendingConfirmation>()
  const [confirmLoading, setConfirmLoading] = useState(false)

  const currentLevel = levels.find((level) => level.uuid === activeLevel) ?? levels[0]
  const activeLevelName = currentLevel?.uuid
  const levelOptions = useMemo(
    () =>
      levels.map((level) => ({
        value: level.uuid,
        label: level.levelName || level.uuid,
      })),
    [levels],
  )

  async function loadStatus() {
    const nextLevels = normalizeLevelStatus(assertApiSuccess(await getLevelStatus()))
    setLevels(nextLevels)
    setActiveLevel((current) => {
      if (current && nextLevels.some((level) => level.uuid === current)) {
        return current
      }

      return nextLevels[0]?.uuid
    })
    return nextLevels
  }

  async function loadPanelData() {
    try {
      setLoading(true)
      const [levelEnvelope, systemEnvelope] = await Promise.all([getLevelStatus(), getSystemInfo()])
      const nextLevels = normalizeLevelStatus(assertApiSuccess(levelEnvelope))
      setLevels(nextLevels)
      setSystemInfo(assertApiSuccess(systemEnvelope))
      setActiveLevel((current) => {
        if (current && nextLevels.some((level) => level.uuid === current)) {
          return current
        }

        return nextLevels[0]?.uuid
      })
    } catch (error) {
      message.error(getErrorMessage(error, '加载面板数据失败'))
    } finally {
      setLoading(false)
    }
  }

  async function loadLogs(levelName: string) {
    try {
      setLogLines(assertApiSuccess(await getLevelServerLog(levelName, 80)))
    } catch (error) {
      message.error(getErrorMessage(error, '加载服务器日志失败'))
    }
  }

  useEffect(() => {
    void loadPanelData()
  }, [])

  useEffect(() => {
    if (activeLevelName) {
      void loadLogs(activeLevelName)
    }
  }, [activeLevelName])

  async function runAction(key: string, action: () => Promise<unknown>, success: string) {
    try {
      setActionLoading(key)
      await action()
      message.success(success)
    } catch (error) {
      message.error(getErrorMessage(error, '操作失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  function confirmDangerAction({ title, content, action }: PendingConfirmation) {
    setPendingConfirmation({
      title,
      content,
      action,
    })
  }

  async function handleConfirmDangerAction() {
    const current = pendingConfirmation
    if (!current) {
      return
    }

    try {
      setConfirmLoading(true)
      await current.action()
      setPendingConfirmation(undefined)
    } finally {
      setConfirmLoading(false)
    }
  }

  function requireLevel(): string | undefined {
    if (!activeLevelName) {
      message.warning('请选择世界')
      return undefined
    }

    return activeLevelName
  }

  async function handleUpdateGame() {
    await runAction(
      'update',
      async () => {
        assertApiSuccess(await updateGame())
      },
      '更新任务已提交',
    )
  }

  async function handleCreateBackup() {
    await runAction(
      'backup',
      async () => {
        assertApiSuccess(await createGameBackup())
      },
      '备份任务已提交',
    )
  }

  async function handleStartLevel() {
    const levelName = requireLevel()
    if (!levelName) {
      return
    }

    await runAction(
      'start',
      async () => {
        assertApiSuccess(await startLevel(levelName))
        await loadStatus()
      },
      '启动命令已发送',
    )
  }

  async function handleStopLevel() {
    const levelName = requireLevel()
    if (!levelName) {
      return
    }

    confirmDangerAction({
      title: '确认停止世界',
      content: `将停止 ${levelName} 世界。`,
      action: () =>
        runAction(
          'stop',
          async () => {
            assertApiSuccess(await stopLevel(levelName))
            await loadStatus()
          },
          '停止命令已发送',
        ),
    })
  }

  async function executeLevelCommand(
    levelName: string,
    nextCommand: string,
    key = 'command',
    success = '远程指令已发送',
  ) {
    let sent = false
    await runAction(
      key,
      async () => {
        assertApiSuccess(await sendGameCommand({ levelName, command: nextCommand }))
        setCommandHistory(pushCommandHistory(nextCommand))
        await loadLogs(levelName)
        sent = true
      },
      success,
    )
    return sent
  }

  function confirmLevelCommand(
    rawCommand: string,
    key = 'command',
    success = '远程指令已发送',
    onSent?: () => void,
  ) {
    const levelName = requireLevel()
    const nextCommand = rawCommand.trim()
    if (!levelName || !nextCommand) {
      return
    }

    confirmDangerAction({
      title: '确认执行指令',
      content: `将在 ${levelName} 执行：${nextCommand}`,
      action: async () => {
        if (await executeLevelCommand(levelName, nextCommand, key, success)) {
          onSent?.()
        }
      },
    })
  }

  function handleCommand(rawCommand: string) {
    confirmLevelCommand(rawCommand, 'command', '远程指令已发送', () => setCommand(''))
  }

  function handleRemoteCommand() {
    confirmLevelCommand(remoteCommand, 'remote-command', '指令已发送', () => setRemoteCommand(''))
  }

  function handleRemoteMessage() {
    const nextMessage = remoteMessage.trim()
    if (!nextMessage) {
      message.warning('请填写公告内容')
      return
    }

    confirmLevelCommand(buildAnnounceCommand(nextMessage), 'announcement', '公告已发送', () =>
      setRemoteMessage(''),
    )
  }

  function handleSendItem(nextItemCode = itemCode) {
    const normalizedItemCode = nextItemCode.trim()
    if (!normalizedItemCode) {
      message.warning('请填写物品代码')
      return
    }

    const commandToSend = buildItemCommand(normalizedItemCode, itemAmount, itemTargetKuId)
    confirmLevelCommand(commandToSend, 'item', '物品指令已发送', () => setItemCode(''))
  }

  function handleSaveGame() {
    confirmLevelCommand('c_save()', 'save', '保存指令已发送')
  }

  function handleRollback(day: number) {
    confirmDangerAction({
      title: '确认回档',
      content: `将回档 ${day} 天。`,
      action: () =>
        runAction(
          `rollback-${day}`,
          async () => {
            assertApiSuccess(await rollbackGame(day))
            if (activeLevelName) {
              await loadLogs(activeLevelName)
            }
          },
          `回档 ${day} 天命令已发送`,
        ),
    })
  }

  function handleRegenerateWorld() {
    confirmDangerAction({
      title: '确认重置世界',
      content: '将重新生成当前世界，请确认已经备份存档。',
      action: () =>
        runAction(
          'regenerate',
          async () => {
            assertApiSuccess(await regenerateWorld())
            if (activeLevelName) {
              await loadLogs(activeLevelName)
            }
          },
          '重置世界命令已发送',
        ),
    })
  }

  async function handleQueryPlayers(all: boolean) {
    const levelName = requireLevel()
    if (!levelName) {
      return
    }

    await runAction(
      all ? 'players-all' : 'players',
      async () => {
        const response = all ? await getAllOnlinePlayers() : await getOnlinePlayers(levelName)
        setPlayers(assertApiSuccess(response))
      },
      '玩家列表已刷新',
    )
  }

  function handleRunCustomCommand(nextCommand: string, label: string) {
    confirmLevelCommand(nextCommand, `custom:${label}`, `${label} 已发送`)
  }

  function deleteCustomCommand(id: string) {
    const nextCommands = customCommands.filter((item) => item.id !== id)
    setCustomCommands(nextCommands)
    writeCustomCommands(nextCommands)
    if (editingCommandId === id) {
      setCustomName('')
      setCustomCommand('')
      setEditingCommandId(undefined)
    }
  }

  function handleDeleteCustomCommand(id: string) {
    const target = customCommands.find((item) => item.id === id)
    confirmDangerAction({
      title: '确认删除指令',
      content: `将删除自定义指令 ${target?.name ?? id}。`,
      action: () => deleteCustomCommand(id),
    })
  }

  function handleRemoveHistory(historyCommand: string) {
    const nextHistory = commandHistory.filter((item) => item !== historyCommand)
    setCommandHistory(nextHistory)
    writeCommandHistory(nextHistory)
  }

  function handleSaveCustomCommand() {
    const nextName = customName.trim()
    const nextCommand = customCommand.trim()
    if (!nextName || !nextCommand) {
      message.warning('请填写指令名称和 Lua 指令')
      return
    }

    const nextCommands = editingCommandId
      ? customCommands.map((item) =>
          item.id === editingCommandId ? { ...item, name: nextName, command: nextCommand } : item,
        )
      : [
          ...customCommands,
          {
            id: `${Date.now()}-${nextName}`,
            name: nextName,
            command: nextCommand,
          },
        ]
    setCustomCommands(nextCommands)
    writeCustomCommands(nextCommands)
    setCustomName('')
    setCustomCommand('')
    setEditingCommandId(undefined)
    message.success('指令已保存')
  }

  function handleEditCustomCommand(item: CustomCommand) {
    setCustomName(item.name)
    setCustomCommand(item.command)
    setEditingCommandId(item.id)
    setActivePanelTab('customEdit')
  }

  function renderToolTab() {
    switch (activePanelTab) {
      case 'remote':
        return (
          <RemoteCommandTab
            activeLevelName={activeLevelName}
            levelOptions={levelOptions}
            command={remoteCommand}
            message={remoteMessage}
            history={commandHistory}
            actionLoading={actionLoading}
            onLevelChange={setActiveLevel}
            onCommandChange={setRemoteCommand}
            onMessageChange={setRemoteMessage}
            onSendCommand={handleRemoteCommand}
            onSendMessage={handleRemoteMessage}
            onRunHistory={(historyCommand) => confirmLevelCommand(historyCommand)}
            onRemoveHistory={handleRemoveHistory}
          />
        )
      case 'tooManyItemsPlus':
        return (
          <TooManyItemsTab
            activeLevelName={activeLevelName}
            levelOptions={levelOptions}
            players={players}
            itemCode={itemCode}
            itemAmount={itemAmount}
            targetKuId={itemTargetKuId}
            actionLoading={actionLoading}
            onLevelChange={setActiveLevel}
            onItemCodeChange={setItemCode}
            onItemAmountChange={setItemAmount}
            onTargetKuIdChange={setItemTargetKuId}
            onSendItem={handleSendItem}
            onQueryPlayers={() => void handleQueryPlayers(false)}
            onQueryAllPlayers={() => void handleQueryPlayers(true)}
          />
        )
      case 'custom':
        return (
          <CustomCommandTab
            commands={[...builtinCommands, ...customCommands]}
            actionLoading={actionLoading}
            onRunCommand={(item) => handleRunCustomCommand(item.command, item.name)}
            onEditCommand={(item) => {
              if (isCustomCommand(item)) {
                handleEditCustomCommand(item)
              }
            }}
          />
        )
      case 'customEdit':
        return (
          <CustomCommandEditorTab
            name={customName}
            command={customCommand}
            commands={customCommands}
            editingCommandId={editingCommandId}
            onNameChange={setCustomName}
            onCommandChange={setCustomCommand}
            onSave={handleSaveCustomCommand}
            onEdit={handleEditCustomCommand}
            onDelete={handleDeleteCustomCommand}
            onReset={() => {
              setCustomName('')
              setCustomCommand('')
              setEditingCommandId(undefined)
            }}
          />
        )
      default:
        return null
    }
  }

  return (
    <div className="panel-page">
      <Tabs
        className="panel-tabs"
        activeKey={activePanelTab}
        onChange={(key) => setActivePanelTab(key as PanelTabKey)}
        items={[
          { key: 'panel', label: '面板' },
          { key: 'remote', label: '远程' },
          { key: 'tooManyItemsPlus', label: 'TooManyItemsPlus' },
          { key: 'custom', label: '自定义指令' },
          { key: 'customEdit', label: '自定义指令-编辑' },
        ]}
      />

      {activePanelTab === 'panel' ? (
        <>
          <ProCard className="panel-resource-card" bordered={false}>
            {loading ? (
              <div className="page-loading">
                <Spin />
              </div>
            ) : (
              <Row gutter={[24, 16]} align="middle">
                <Col xs={24} lg={6}>
                  <Statistic title="面板内存" value={formatBytes(systemInfo?.panelMemUsage ?? 0)} />
                  <Space direction="vertical" size={2}>
                    <Typography.Text strong>{systemInfo?.host?.hostname ?? '-'}</Typography.Text>
                    <Typography.Text type="secondary">{formatHost(systemInfo)}</Typography.Text>
                  </Space>
                </Col>
                <Col xs={24} lg={6}>
                  <Space>
                    <Progress
                      type="circle"
                      percent={roundPercent(systemInfo?.mem?.usedPercent ?? 0)}
                      size={72}
                      strokeColor="#73d13d"
                    />
                    <Statistic
                      title="内存使用"
                      value={formatBytes(systemInfo?.mem?.used ?? 0)}
                      suffix={
                        <span className="stat-sub">
                          总内存 {formatBytes(systemInfo?.mem?.total ?? 0)}
                        </span>
                      }
                    />
                  </Space>
                </Col>
                <Col xs={24} lg={6}>
                  <Space>
                    <Progress
                      type="circle"
                      percent={roundPercent(systemInfo?.cpu?.cpuUsedPercent ?? 0)}
                      size={72}
                    />
                    <Statistic
                      title="CPU使用"
                      value={roundPercent(systemInfo?.cpu?.cpuUsedPercent ?? 0)}
                      suffix="%"
                    />
                  </Space>
                </Col>
                <Col xs={24} lg={6}>
                  <Space>
                    <Progress
                      type="circle"
                      percent={roundPercent(primaryDisk(systemInfo)?.usage ?? 0)}
                      size={72}
                    />
                    <Statistic
                      title="磁盘使用"
                      value={formatBytes(primaryDisk(systemInfo)?.total ?? 0)}
                    />
                  </Space>
                </Col>
              </Row>
            )}
          </ProCard>

          <div className="panel-content-grid">
            <div className="panel-left-column">
              <ProCard title="服务器信息" bordered={false}>
                <Space className="panel-action-row" wrap>
                  <Button
                    type="primary"
                    icon={<SyncOutlined />}
                    loading={actionLoading === 'update'}
                    onClick={() => void handleUpdateGame()}
                  >
                    更新游戏
                  </Button>
                  <Button
                    type="primary"
                    icon={<SaveOutlined />}
                    loading={actionLoading === 'backup'}
                    onClick={() => void handleCreateBackup()}
                  >
                    创建备份
                  </Button>
                  <Button
                    type="primary"
                    icon={<CloudUploadOutlined />}
                    onClick={() => navigate(routes.backup)}
                  >
                    上传存档
                  </Button>
                  <Button
                    type="primary"
                    icon={<CloudDownloadOutlined />}
                    onClick={() => navigate(routes.genMap)}
                  >
                    地图预览
                  </Button>
                </Space>
                <div className="server-info-grid">
                  <InfoItem label="当前世界" value={currentLevel?.levelName ?? '-'} />
                  <InfoItem label="世界目录" value={activeLevelName ?? '-'} />
                  <InfoItem label="运行状态" value={currentLevel?.status ? '运行' : '停止'} />
                  <InfoItem
                    label="模组数量"
                    value={String(countEnabledMods(currentLevel?.modoverrides))}
                  />
                  <InfoItem label="玩家数量" value={String(players.length)} />
                  <InfoItem
                    label="世界类型"
                    value={currentLevel?.is_master ? '主世界' : '从世界'}
                  />
                  <InfoItem label="CPU占用" value={currentLevel?.Ps?.cpuUage ?? '-'} />
                  <InfoItem label="内存占用" value={currentLevel?.Ps?.memUage ?? '-'} />
                </div>
              </ProCard>

              <ProCard
                title="世界列表"
                bordered={false}
                extra={
                  <Space>
                    <Button
                      type="primary"
                      icon={<PlayCircleOutlined />}
                      loading={actionLoading === 'start'}
                      onClick={() => void handleStartLevel()}
                    >
                      {getPanelActionLabel('start')}
                    </Button>
                    <Button
                      type="primary"
                      icon={<PoweroffOutlined />}
                      loading={actionLoading === 'stop'}
                      onClick={() => void handleStopLevel()}
                    >
                      {getPanelActionLabel('stop')}
                    </Button>
                  </Space>
                }
              >
                <Table
                  rowKey="uuid"
                  loading={loading}
                  pagination={false}
                  dataSource={levels}
                  rowSelection={{
                    type: 'radio',
                    selectedRowKeys: activeLevelName ? [activeLevelName] : [],
                    onChange: (keys) => setActiveLevel(String(keys[0])),
                  }}
                  onRow={(record) => ({
                    onClick: () => setActiveLevel(record.uuid),
                  })}
                  columns={[
                    {
                      title: '世界',
                      dataIndex: 'levelName',
                      render: (_, row) => (
                        <Space>
                          <span className="level-badge">
                            {(row.levelName || row.uuid).slice(0, 1)}
                          </span>
                          <Space direction="vertical" size={0}>
                            <span>{row.levelName}</span>
                            <Typography.Text type="secondary">{row.uuid}</Typography.Text>
                          </Space>
                        </Space>
                      ),
                    },
                    { title: '内存', render: (_, row) => row.Ps?.memUage ?? '-' },
                    { title: 'CPU', render: (_, row) => row.Ps?.cpuUage ?? '-' },
                    {
                      title: '状态',
                      dataIndex: 'status',
                      render: (status) => (
                        <Tag color={status ? 'processing' : 'default'}>
                          {status ? '运行' : '停止'}
                        </Tag>
                      ),
                    },
                  ]}
                />
              </ProCard>
            </div>

            <div className="panel-right-column">
              <ProCard
                title="服务器日志"
                bordered={false}
                extra={
                  <Space>
                    <Select
                      value={activeLevelName}
                      options={levelOptions}
                      onChange={setActiveLevel}
                      style={{ minWidth: 120 }}
                    />
                    <Button
                      type="primary"
                      icon={<CloudDownloadOutlined />}
                      href={activeLevelName ? getLevelLogDownloadUrl(activeLevelName) : undefined}
                      disabled={!activeLevelName}
                    >
                      下载日志
                    </Button>
                  </Space>
                }
              >
                <div className="server-log">
                  {logLines.length === 0 ? (
                    <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无日志" />
                  ) : (
                    logLines.map((line, index) => (
                      <div key={`${line}-${index}`} className="server-log-line">
                        <span>{index + 1}</span>
                        <code>{line}</code>
                      </div>
                    ))
                  )}
                </div>
                <Input.Search
                  className="command-input"
                  enterButton={<SendOutlined />}
                  placeholder="输入远程指令"
                  value={command}
                  loading={actionLoading === 'command'}
                  onChange={(event) => setCommand(event.target.value)}
                  onPressEnter={(event) => void handleCommand(event.currentTarget.value)}
                  onSearch={(value) => void handleCommand(value)}
                />
                <Space className="rollback-row" wrap>
                  <Button
                    type="primary"
                    icon={<SaveOutlined />}
                    loading={actionLoading === 'save'}
                    onClick={() => void handleSaveGame()}
                  >
                    保存存档
                  </Button>
                  <Button
                    danger
                    type="primary"
                    loading={actionLoading === 'regenerate'}
                    onClick={() => void handleRegenerateWorld()}
                  >
                    重置世界
                  </Button>
                  {[1, 2, 3, 4, 5, 6].map((day) => (
                    <Button
                      key={day}
                      loading={actionLoading === `rollback-${day}`}
                      onClick={() => void handleRollback(day)}
                    >
                      回档({day})天
                    </Button>
                  ))}
                </Space>
              </ProCard>

              <ProCard title="玩家列表" bordered={false}>
                <Space className="player-toolbar" wrap>
                  <Select
                    value={activeLevelName}
                    options={levelOptions}
                    onChange={setActiveLevel}
                    style={{ minWidth: 120 }}
                  />
                  <Button
                    type="primary"
                    loading={actionLoading === 'players'}
                    onClick={() => void handleQueryPlayers(false)}
                  >
                    查询
                  </Button>
                  <Button
                    type="primary"
                    loading={actionLoading === 'players-all'}
                    onClick={() => void handleQueryPlayers(true)}
                  >
                    查询所有
                  </Button>
                  <Tag color="success">{players.length}</Tag>
                </Space>
                <Table
                  rowKey="key"
                  pagination={false}
                  dataSource={players}
                  columns={[
                    { title: '玩家', dataIndex: 'name' },
                    { title: 'KU ID', dataIndex: 'kuId' },
                    { title: '角色', dataIndex: 'role' },
                    { title: '天数', dataIndex: 'day' },
                  ]}
                />
              </ProCard>
            </div>
          </div>
        </>
      ) : (
        renderToolTab()
      )}
      {pendingConfirmation ? (
        <div className="confirm-dialog-backdrop">
          <div
            aria-labelledby="confirm-dialog-title"
            aria-modal="true"
            className="confirm-dialog"
            role="dialog"
          >
            <Typography.Title id="confirm-dialog-title" level={4}>
              {pendingConfirmation.title}
            </Typography.Title>
            <Typography.Paragraph>{pendingConfirmation.content}</Typography.Paragraph>
            <Space className="confirm-dialog-actions">
              <Button disabled={confirmLoading} onClick={() => setPendingConfirmation(undefined)}>
                取消
              </Button>
              <Button
                danger
                loading={confirmLoading}
                type="primary"
                onClick={() => void handleConfirmDangerAction()}
              >
                确认
              </Button>
            </Space>
          </div>
        </div>
      ) : null}
    </div>
  )
}

function RemoteCommandTab({
  activeLevelName,
  levelOptions,
  command,
  message,
  history,
  actionLoading,
  onLevelChange,
  onCommandChange,
  onMessageChange,
  onSendCommand,
  onSendMessage,
  onRunHistory,
  onRemoveHistory,
}: {
  activeLevelName?: string
  levelOptions: { value: string; label: string }[]
  command: string
  message: string
  history: string[]
  actionLoading?: string
  onLevelChange: (levelName: string) => void
  onCommandChange: (value: string) => void
  onMessageChange: (value: string) => void
  onSendCommand: () => void
  onSendMessage: () => void
  onRunHistory: (command: string) => void
  onRemoveHistory: (command: string) => void
}) {
  return (
    <div className="panel-command-grid">
      <ProCard title="发送指令" bordered={false}>
        <Space className="panel-command-toolbar" wrap>
          <Select
            value={activeLevelName}
            options={levelOptions}
            onChange={onLevelChange}
            style={{ minWidth: 140 }}
          />
        </Space>
        <Input.TextArea
          rows={4}
          placeholder="输入控制台指令"
          value={command}
          onChange={(event) => onCommandChange(event.target.value)}
        />
        <Button
          className="panel-command-submit"
          type="primary"
          aria-label="发送指令"
          icon={<SendOutlined />}
          loading={actionLoading === 'remote-command'}
          onClick={onSendCommand}
        >
          发送指令
        </Button>
      </ProCard>

      <ProCard title="发送公告" bordered={false}>
        <Input.TextArea
          rows={4}
          placeholder="输入公告内容"
          value={message}
          onChange={(event) => onMessageChange(event.target.value)}
        />
        <Button
          className="panel-command-submit"
          type="primary"
          aria-label="发送公告"
          icon={<SendOutlined />}
          loading={actionLoading === 'announcement'}
          onClick={onSendMessage}
        >
          发送公告
        </Button>
      </ProCard>

      <ProCard className="panel-command-history" title="指令历史" bordered={false}>
        {history.length === 0 ? (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无历史指令" />
        ) : (
          <Space wrap>
            {history.map((item) => (
              <Tag
                key={item}
                closable
                onClose={(event) => {
                  event.preventDefault()
                  onRemoveHistory(item)
                }}
                onClick={() => onRunHistory(item)}
              >
                {item}
              </Tag>
            ))}
          </Space>
        )}
      </ProCard>
    </div>
  )
}

function TooManyItemsTab({
  activeLevelName,
  levelOptions,
  players,
  itemCode,
  itemAmount,
  targetKuId,
  actionLoading,
  onLevelChange,
  onItemCodeChange,
  onItemAmountChange,
  onTargetKuIdChange,
  onSendItem,
  onQueryPlayers,
  onQueryAllPlayers,
}: {
  activeLevelName?: string
  levelOptions: { value: string; label: string }[]
  players: OnlinePlayer[]
  itemCode: string
  itemAmount: number
  targetKuId?: string
  actionLoading?: string
  onLevelChange: (levelName: string) => void
  onItemCodeChange: (value: string) => void
  onItemAmountChange: (value: number) => void
  onTargetKuIdChange: (value?: string) => void
  onSendItem: (itemCode?: string) => void
  onQueryPlayers: () => void
  onQueryAllPlayers: () => void
}) {
  const playerOptions = players.map((player) => ({
    value: player.kuId,
    label: `${player.name} / ${player.kuId}`,
  }))

  return (
    <ProCard title="TooManyItemsPlus" bordered={false}>
      <Space className="panel-command-toolbar" wrap>
        <Select
          value={activeLevelName}
          options={levelOptions}
          onChange={onLevelChange}
          style={{ minWidth: 140 }}
        />
        <Select
          allowClear
          placeholder="选择玩家"
          value={targetKuId}
          options={playerOptions}
          onChange={onTargetKuIdChange}
          style={{ minWidth: 220 }}
        />
        <InputNumber
          min={1}
          max={99}
          value={itemAmount}
          addonAfter="数量"
          onChange={(value) => onItemAmountChange(Number(value ?? 1))}
        />
        <Button loading={actionLoading === 'players'} onClick={onQueryPlayers}>
          刷新玩家
        </Button>
        <Button loading={actionLoading === 'players-all'} onClick={onQueryAllPlayers}>
          查询所有
        </Button>
      </Space>
      <Input.Search
        className="panel-item-input"
        placeholder="请输入物品代码"
        value={itemCode}
        enterButton="发送物品"
        loading={actionLoading === 'item'}
        onChange={(event) => onItemCodeChange(event.target.value)}
        onSearch={(value) => onSendItem(value)}
      />
      <div className="panel-item-presets">
        {itemPresets.map((item) => (
          <Button key={item.command} type="primary" onClick={() => onSendItem(item.command)}>
            {item.name}
          </Button>
        ))}
      </div>
    </ProCard>
  )
}

function CustomCommandTab({
  commands,
  actionLoading,
  onRunCommand,
  onEditCommand,
}: {
  commands: CommandPreset[]
  actionLoading?: string
  onRunCommand: (item: CommandPreset) => void
  onEditCommand: (item: CommandPreset) => void
}) {
  return (
    <ProCard title="自定义指令" bordered={false}>
      <div className="panel-custom-command-grid">
        {commands.map((item) => (
          <div key={`${item.name}-${item.command}`} className="panel-custom-command">
            <Button
              type="primary"
              danger={item.danger}
              loading={actionLoading === `custom:${item.name}`}
              onClick={() => onRunCommand(item)}
            >
              {item.name}
            </Button>
            {'id' in item ? (
              <Button onClick={() => onEditCommand(item)} type="link">
                编辑
              </Button>
            ) : null}
          </div>
        ))}
      </div>
    </ProCard>
  )
}

function CustomCommandEditorTab({
  name,
  command,
  commands,
  editingCommandId,
  onNameChange,
  onCommandChange,
  onSave,
  onEdit,
  onDelete,
  onReset,
}: {
  name: string
  command: string
  commands: CustomCommand[]
  editingCommandId?: string
  onNameChange: (value: string) => void
  onCommandChange: (value: string) => void
  onSave: () => void
  onEdit: (item: CustomCommand) => void
  onDelete: (id: string) => void
  onReset: () => void
}) {
  return (
    <div className="panel-command-grid">
      <ProCard title={editingCommandId ? '编辑指令' : '新增指令'} bordered={false}>
        <Space className="panel-command-toolbar" direction="vertical">
          <Input
            placeholder="指令名称"
            value={name}
            onChange={(event) => onNameChange(event.target.value)}
          />
          <Input.TextArea
            rows={5}
            placeholder="Lua 指令"
            value={command}
            onChange={(event) => onCommandChange(event.target.value)}
          />
          <Space>
            <Button type="primary" onClick={onSave}>
              保存指令
            </Button>
            <Button onClick={onReset}>清空</Button>
          </Space>
        </Space>
      </ProCard>

      <ProCard title="已保存指令" bordered={false}>
        {commands.length === 0 ? (
          <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} description="暂无自定义指令" />
        ) : (
          <Table
            rowKey="id"
            pagination={false}
            dataSource={commands}
            columns={[
              { title: '名称', dataIndex: 'name' },
              {
                title: 'Lua 指令',
                dataIndex: 'command',
                render: (value) => <Typography.Text code>{value}</Typography.Text>,
              },
              {
                title: '操作',
                render: (_, row) => (
                  <Space>
                    <Button type="link" onClick={() => onEdit(row)}>
                      编辑
                    </Button>
                    <Button danger type="link" onClick={() => onDelete(row.id)}>
                      删除
                    </Button>
                  </Space>
                ),
              },
            ]}
          />
        )}
      </ProCard>
    </div>
  )
}

function InfoItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="server-info-item">
      <Typography.Text strong>{label}</Typography.Text>
      <span>{value || '-'}</span>
    </div>
  )
}

function formatHost(systemInfo: SystemInfo | undefined): string {
  if (!systemInfo?.host) {
    return '-'
  }

  return `${systemInfo.host.os} / ${systemInfo.host.kernelArch}-${systemInfo.host.platform}`
}

function primaryDisk(systemInfo: SystemInfo | undefined) {
  return systemInfo?.disk?.devices?.[0]
}

function normalizeLevelStatus(value: unknown): LevelStatusInfo[] {
  return Array.isArray(value) ? value : []
}

function roundPercent(value: number): number {
  if (!Number.isFinite(value)) {
    return 0
  }

  return Math.max(0, Math.min(100, Number(value.toFixed(2))))
}

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) {
    return '0 B'
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let nextValue = value
  let unitIndex = 0
  while (nextValue >= 1024 && unitIndex < units.length - 1) {
    nextValue /= 1024
    unitIndex += 1
  }

  return `${nextValue.toFixed(unitIndex === 0 ? 0 : 2)} ${units[unitIndex]}`
}

function countEnabledMods(modoverrides: string | undefined): number {
  if (!modoverrides) {
    return 0
  }

  return (modoverrides.match(/workshop-\d+/g) ?? []).length
}

function buildAnnounceCommand(message: string): string {
  return `c_announce("${escapeLuaString(message)}")`
}

function buildItemCommand(itemCode: string, amount: number, kuId?: string): string {
  const safeItemCode = escapeLuaString(itemCode)
  const safeAmount = Math.max(1, Math.min(99, Math.floor(amount || 1)))
  if (!kuId) {
    return `c_spawn("${safeItemCode}", ${safeAmount})`
  }

  return `ThePlayer = UserToPlayer("${escapeLuaString(
    kuId,
  )}") c_give("${safeItemCode}", ${safeAmount}) ThePlayer = nil`
}

function escapeLuaString(value: string): string {
  return value
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t')
}

function readCommandHistory(): string[] {
  return readStorageArray<string>(COMMAND_HISTORY_KEY).filter((item) => typeof item === 'string')
}

function pushCommandHistory(command: string): string[] {
  const nextHistory = [command, ...readCommandHistory().filter((item) => item !== command)].slice(
    0,
    20,
  )
  writeCommandHistory(nextHistory)
  return nextHistory
}

function writeCommandHistory(history: string[]) {
  writeStorageArray(COMMAND_HISTORY_KEY, history)
}

function readCustomCommands(): CustomCommand[] {
  return readStorageArray<CustomCommand>(CUSTOM_COMMAND_KEY).filter(isCustomCommand)
}

function isCustomCommand(item: CommandPreset): item is CustomCommand {
  return 'id' in item && typeof item.id === 'string'
}

function writeCustomCommands(commands: CustomCommand[]) {
  writeStorageArray(CUSTOM_COMMAND_KEY, commands)
}

function readStorageArray<T>(key: string): T[] {
  if (typeof window === 'undefined') {
    return []
  }

  try {
    const rawValue = window.localStorage.getItem(key)
    const parsed = rawValue ? JSON.parse(rawValue) : []
    return Array.isArray(parsed) ? parsed : []
  } catch {
    return []
  }
}

function writeStorageArray<T>(key: string, value: T[]) {
  if (typeof window === 'undefined') {
    return
  }

  window.localStorage.setItem(key, JSON.stringify(value))
}
