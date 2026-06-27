import { ProCard } from '@ant-design/pro-components'
import {
  Alert,
  App as AntApp,
  Button,
  Drawer,
  Empty,
  Form,
  Image,
  Input,
  Select,
  Space,
  Spin,
  Switch,
  Tabs,
  Typography,
  Upload,
  type UploadProps,
} from 'antd'
import {
  DeleteOutlined,
  SaveOutlined,
  SearchOutlined,
  ShopOutlined,
  SyncOutlined,
  ToolOutlined,
  UploadOutlined,
} from '@ant-design/icons'
import { useEffect, useMemo, useState } from 'react'

import {
  deleteMod,
  getMods,
  getUgcMods,
  saveModInfo,
  searchMods,
  subscribeMod,
  updateAllModInfo,
  updateMod,
  uploadModInfoFile,
  type ModInfoRecord,
  type ModSearchItem,
  type UgcModInfo,
} from '@/features/mods/mod.api'
import {
  formatModUpdatedAt,
  formatWorkshopId,
  getConfigEntryLabel,
  getConfigOptionLabel,
  getModDisplayName,
  getModImageUrl,
  getModWorkshopId,
  isModEnabled,
  normalizeModConfig,
  stringifyConfigValue,
  type ModConfigEntry,
} from '@/features/mods/mod-model'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

const levelOptions = [
  { label: '森林', value: 'Master' },
  { label: '洞穴', value: 'Caves' },
]

export default function ModPage() {
  const { message } = AntApp.useApp()
  const [loading, setLoading] = useState(true)
  const [mods, setMods] = useState<ModInfoRecord[]>([])
  const [selectedModId, setSelectedModId] = useState<string>()
  const [activeTab, setActiveTab] = useState('settings')
  const [levelName, setLevelName] = useState('Master')
  const [optionDrawerOpen, setOptionDrawerOpen] = useState(false)
  const [actionLoading, setActionLoading] = useState<string>()
  const [searchText, setSearchText] = useState('')
  const [searching, setSearching] = useState(false)
  const [searchResults, setSearchResults] = useState<ModSearchItem[]>([])
  const [ugcLoading, setUgcLoading] = useState(false)
  const [ugcMods, setUgcMods] = useState<UgcModInfo[]>([])

  async function loadMods() {
    try {
      setLoading(true)
      const envelope = await getMods()
      const nextMods = assertApiSuccess(envelope)
      setMods(nextMods)
      setSelectedModId((current) => {
        if (current && nextMods.some((mod) => getModWorkshopId(mod) === current)) {
          return current
        }

        return getModWorkshopId(nextMods[0] ?? {})
      })
    } catch (error) {
      message.error(getErrorMessage(error, '加载模组失败'))
    } finally {
      setLoading(false)
    }
  }

  async function loadUgcMods(nextLevelName = levelName) {
    try {
      setUgcLoading(true)
      const envelope = await getUgcMods(nextLevelName)
      setUgcMods(assertApiSuccess(envelope))
    } catch (error) {
      message.error(getErrorMessage(error, '读取 Ugc 模组失败'))
    } finally {
      setUgcLoading(false)
    }
  }

  useEffect(() => {
    void loadMods()
  }, [])

  const selectedMod = useMemo(() => {
    return mods.find((mod) => getModWorkshopId(mod) === selectedModId) ?? mods[0]
  }, [mods, selectedModId])
  const selectedLevelLabel =
    levelOptions.find((option) => option.value === levelName)?.label ?? levelName
  const selectedModOptions = normalizeModConfig(selectedMod?.mod_config)

  async function handleSave() {
    if (!selectedMod) {
      message.warning('请选择模组')
      return
    }

    try {
      setActionLoading('save')
      const envelope = await saveModInfo(selectedMod)
      const savedMod = assertApiSuccess(envelope)
      setMods((current) =>
        current.map((mod) =>
          getModWorkshopId(mod) === getModWorkshopId(savedMod) ? savedMod : mod,
        ),
      )
      message.success('模组配置已保存')
    } catch (error) {
      message.error(getErrorMessage(error, '保存模组失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleUpdateAll() {
    try {
      setActionLoading('update-all')
      assertApiSuccess(await updateAllModInfo())
      message.success('已提交全部更新')
      await loadMods()
    } catch (error) {
      message.error(getErrorMessage(error, '全部更新失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleUpdateSelected() {
    if (!selectedMod) {
      return
    }

    try {
      setActionLoading('update-selected')
      const envelope = await updateMod(getModWorkshopId(selectedMod))
      const updatedMod = assertApiSuccess(envelope)
      setMods((current) =>
        current.map((mod) =>
          getModWorkshopId(mod) === getModWorkshopId(updatedMod) ? updatedMod : mod,
        ),
      )
      message.success('模组已更新')
    } catch (error) {
      message.error(getErrorMessage(error, '更新模组失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleDelete(mod: ModInfoRecord) {
    const modId = getModWorkshopId(mod)
    try {
      setActionLoading(`delete-${modId}`)
      assertApiSuccess(await deleteMod(modId))
      setMods((current) => current.filter((item) => getModWorkshopId(item) !== modId))
      setSelectedModId((current) => (current === modId ? undefined : current))
      message.success('模组已删除')
    } catch (error) {
      message.error(getErrorMessage(error, '删除模组失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleSearch(value = searchText) {
    try {
      setSearching(true)
      const envelope = await searchMods(value.trim())
      const result = assertApiSuccess(envelope)
      setSearchResults(result.data ?? [])
    } catch (error) {
      message.error(getErrorMessage(error, '搜索模组失败'))
    } finally {
      setSearching(false)
    }
  }

  async function handleSubscribe(item: ModSearchItem) {
    try {
      setActionLoading(`subscribe-${item.modid}`)
      const envelope = await subscribeMod(item.modid)
      const subscribedMod = assertApiSuccess(envelope)
      setMods((current) => upsertMod(current, subscribedMod))
      setSelectedModId(getModWorkshopId(subscribedMod))
      setActiveTab('settings')
      message.success('已订阅模组')
    } catch (error) {
      message.error(getErrorMessage(error, '订阅模组失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  function handleToggle(mod: ModInfoRecord, enabled: boolean) {
    const modId = getModWorkshopId(mod)
    setMods((current) =>
      current.map((item) => (getModWorkshopId(item) === modId ? { ...item, enabled } : item)),
    )
  }

  const uploadProps: UploadProps = {
    accept: '.lua,.txt',
    beforeUpload: (file) => {
      void handleUploadModInfo(file)
      return Upload.LIST_IGNORE
    },
    showUploadList: false,
  }

  async function handleUploadModInfo(file: File) {
    if (!selectedMod) {
      message.warning('请先选择模组')
      return
    }

    try {
      setActionLoading('upload')
      const modinfo = await file.text()
      assertApiSuccess(
        await uploadModInfoFile({ workshopId: getModWorkshopId(selectedMod), modinfo }),
      )
      message.success('自定义模组配置已上传')
      await loadMods()
    } catch (error) {
      message.error(getErrorMessage(error, '上传自定义模组配置失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  function handleOpenWorkshop() {
    if (!selectedMod) {
      return
    }

    window.open(
      `https://steamcommunity.com/sharedfiles/filedetails/?id=${getModWorkshopId(selectedMod)}`,
    )
  }

  function handleTabChange(key: string) {
    setActiveTab(key)
    if (key === 'ugc') {
      void loadUgcMods()
    }
  }

  return (
    <ProCard className="mod-page-card" bordered={false}>
      <Tabs
        activeKey={activeTab}
        onChange={handleTabChange}
        items={[
          {
            key: 'settings',
            label: '模组设置',
            children: (
              <>
                <Alert
                  className="mod-page-alert"
                  type="info"
                  showIcon
                  closable
                  message="请先启动世界，mod将自动下载，ugc模块将优先读取。点击保存按钮会替换所有世界的模组配置"
                />
                <Space className="mod-toolbar" wrap>
                  <Button
                    type="primary"
                    icon={<SaveOutlined />}
                    loading={actionLoading === 'save'}
                    onClick={() => void handleSave()}
                  >
                    保存
                  </Button>
                  <Button
                    type="primary"
                    icon={<SyncOutlined />}
                    loading={actionLoading === 'update-all'}
                    onClick={() => void handleUpdateAll()}
                  >
                    全部更新
                  </Button>
                  <Upload {...uploadProps}>
                    <Button
                      type="primary"
                      icon={<UploadOutlined />}
                      loading={actionLoading === 'upload'}
                    >
                      上传自定义模组配置
                    </Button>
                  </Upload>
                  <Select
                    className="mod-level-select"
                    value={levelName}
                    options={levelOptions}
                    onChange={setLevelName}
                  />
                  <Button type="primary" className="mod-level-save">
                    保存到{selectedLevelLabel}
                  </Button>
                </Space>

                {loading ? (
                  <div className="page-loading">
                    <Spin />
                  </div>
                ) : mods.length === 0 ? (
                  <Empty description="暂无模组" />
                ) : (
                  <div className="mod-settings-layout">
                    <div className="mod-list">
                      {mods.map((mod) => {
                        const modId = getModWorkshopId(mod)
                        return (
                          <div
                            key={modId}
                            role="button"
                            tabIndex={0}
                            className={
                              modId === getModWorkshopId(selectedMod ?? {})
                                ? 'mod-row is-active'
                                : 'mod-row'
                            }
                            data-testid={`mod-row-${modId}`}
                            onClick={() => setSelectedModId(modId)}
                            onKeyDown={(event) => {
                              if (event.key === 'Enter' || event.key === ' ') {
                                setSelectedModId(modId)
                              }
                            }}
                          >
                            <img src={getModImageUrl(mod)} alt={getModDisplayName(mod)} />
                            <span className="mod-row-main">
                              <strong>{getModDisplayName(mod)}</strong>
                              <span
                                className="mod-row-actions"
                                onClick={(event) => event.stopPropagation()}
                              >
                                <Switch
                                  checked={isModEnabled(mod)}
                                  checkedChildren="开启"
                                  unCheckedChildren="关闭"
                                  onChange={(checked) => handleToggle(mod, checked)}
                                />
                                <Button
                                  danger
                                  type="link"
                                  size="small"
                                  icon={<DeleteOutlined />}
                                  loading={actionLoading === `delete-${modId}`}
                                  onClick={() => void handleDelete(mod)}
                                >
                                  删除
                                </Button>
                              </span>
                            </span>
                          </div>
                        )
                      })}
                    </div>
                    <ModDetails mod={selectedMod} />
                  </div>
                )}

                <Space className="mod-bottom-actions" wrap>
                  <Button
                    type="primary"
                    icon={<ToolOutlined />}
                    disabled={!selectedMod}
                    onClick={() => setOptionDrawerOpen(true)}
                  >
                    选项
                  </Button>
                  <Button
                    type="primary"
                    icon={<SyncOutlined />}
                    disabled={!selectedMod}
                    loading={actionLoading === 'update-selected'}
                    onClick={() => void handleUpdateSelected()}
                  >
                    更新
                  </Button>
                  <Button
                    icon={<ShopOutlined />}
                    disabled={!selectedMod}
                    onClick={handleOpenWorkshop}
                  >
                    创意工坊
                  </Button>
                </Space>
              </>
            ),
          },
          {
            key: 'subscription',
            label: '模组订阅',
            children: (
              <div className="mod-subscription-panel">
                <Input.Search
                  className="mod-search-input"
                  enterButton={
                    <Button type="primary" icon={<SearchOutlined />}>
                      搜索
                    </Button>
                  }
                  loading={searching}
                  placeholder="输入创意工坊 ID 或关键词"
                  value={searchText}
                  onChange={(event) => setSearchText(event.target.value)}
                  onSearch={(value) => void handleSearch(value)}
                />
                <div className="mod-search-grid">
                  {searchResults.map((item) => (
                    <article className="mod-search-card" key={item.modid}>
                      <img src={item.img || '/assets/dst/mods.png'} alt={item.name || item.modid} />
                      <div className="mod-search-card-body">
                        <h3>{item.name || item.modid}</h3>
                        <Typography.Text type="secondary">
                          作者: {item.author || '-'}
                        </Typography.Text>
                        <Typography.Text type="secondary">
                          评分: {item.score ?? '-'}
                        </Typography.Text>
                        <Typography.Text type="secondary">
                          订阅: {item.subscription || '-'}
                        </Typography.Text>
                        <Typography.Text type="secondary">{item.time || '-'}</Typography.Text>
                        <Button
                          block
                          type="primary"
                          loading={actionLoading === `subscribe-${item.modid}`}
                          onClick={() => void handleSubscribe(item)}
                        >
                          订阅
                        </Button>
                      </div>
                    </article>
                  ))}
                </div>
                {!searching && searchResults.length === 0 ? (
                  <Empty description="暂无搜索结果" />
                ) : null}
              </div>
            ),
          },
          {
            key: 'ugc',
            label: 'Ugc模组',
            children: (
              <div className="mod-ugc-panel">
                <Space className="mod-toolbar" wrap>
                  <Select
                    className="mod-level-select"
                    value={levelName}
                    options={levelOptions}
                    onChange={(value) => {
                      setLevelName(value)
                      void loadUgcMods(value)
                    }}
                  />
                  <Button
                    type="primary"
                    icon={<SyncOutlined />}
                    loading={ugcLoading}
                    onClick={() => void loadUgcMods()}
                  >
                    刷新
                  </Button>
                </Space>
                {ugcLoading ? (
                  <div className="page-loading">
                    <Spin />
                  </div>
                ) : ugcMods.length === 0 ? (
                  <Empty description="暂无 Ugc 模组" />
                ) : (
                  <div className="mod-ugc-list">
                    {ugcMods.map((ugcMod) => (
                      <article className="mod-ugc-row" key={ugcMod.workshopId}>
                        <img
                          src={ugcMod.img || '/assets/dst/mods.png'}
                          alt={ugcMod.name || ugcMod.workshopId}
                        />
                        <div>
                          <strong>{ugcMod.name || ugcMod.workshopId}</strong>
                          <Typography.Text type="secondary">
                            创意工坊:{formatWorkshopId(ugcMod.workshopId)}
                          </Typography.Text>
                        </div>
                      </article>
                    ))}
                  </div>
                )}
              </div>
            ),
          },
        ]}
      />
      <Drawer
        width="46vw"
        title={selectedMod ? getModDisplayName(selectedMod) : '模组选项'}
        open={optionDrawerOpen}
        onClose={() => setOptionDrawerOpen(false)}
        extra={
          <Button type="primary" onClick={() => setOptionDrawerOpen(false)}>
            保存偏好
          </Button>
        }
      >
        {selectedModOptions.length === 0 ? (
          <Empty description="暂无可配置选项" />
        ) : (
          <Form className="mod-option-form" labelCol={{ flex: '180px' }} wrapperCol={{ flex: 1 }}>
            {selectedModOptions.map((entry) => (
              <ModOptionControl key={getConfigEntryLabel(entry)} entry={entry} />
            ))}
          </Form>
        )}
      </Drawer>
    </ProCard>
  )
}

function ModDetails({ mod }: { mod: ModInfoRecord | undefined }) {
  if (!mod) {
    return <Empty description="请选择模组" />
  }

  return (
    <section className="mod-detail-panel">
      <Image
        width={84}
        height={84}
        src={getModImageUrl(mod)}
        alt={getModDisplayName(mod)}
        preview={false}
      />
      <div className="mod-detail-main">
        <Typography.Title level={4}>{getModDisplayName(mod)}</Typography.Title>
        <div className="mod-detail-meta">
          <span>版本: {mod.v || '-'}</span>
          <span>创意工坊:{getModWorkshopId(mod)}</span>
          <span>最后更新: {formatModUpdatedAt(mod.last_time)}</span>
          <span>作者: {mod.auth || '-'}</span>
          <span>饥荒联机版兼容</span>
        </div>
      </div>
      <p>{mod.description || '暂无描述'}</p>
    </section>
  )
}

function ModOptionControl({ entry }: { entry: ModConfigEntry }) {
  const options = (entry.options ?? []).map((option) => ({
    label: getConfigOptionLabel(option),
    value: stringifyConfigValue(option.data),
  }))

  return (
    <Form.Item label={getConfigEntryLabel(entry)}>
      <Select
        value={stringifyConfigValue(entry.default)}
        options={
          options.length > 0
            ? options
            : [
                {
                  label: stringifyConfigValue(entry.default),
                  value: stringifyConfigValue(entry.default),
                },
              ]
        }
      />
    </Form.Item>
  )
}

function upsertMod(mods: ModInfoRecord[], nextMod: ModInfoRecord): ModInfoRecord[] {
  const nextModId = getModWorkshopId(nextMod)
  if (mods.some((mod) => getModWorkshopId(mod) === nextModId)) {
    return mods.map((mod) => (getModWorkshopId(mod) === nextModId ? nextMod : mod))
  }

  return [nextMod, ...mods]
}
