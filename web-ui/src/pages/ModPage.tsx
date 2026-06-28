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
  Pagination,
  Popconfirm,
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
import { saveGameConfig } from '@/features/game/game.api'
import {
  createModOverridesLua,
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

const modSearchPageSize = 12

const modSearchModeOptions = [
  { label: '关键词', value: 'keyword' },
  { label: '创意工坊ID', value: 'workshopId' },
  { label: '全部相关', value: 'all' },
] satisfies Array<{ label: string; value: ModSearchMode }>

type ModSearchMode = 'keyword' | 'workshopId' | 'all'

interface ModSearchMeta {
  page: number
  size: number
  total: number
  totalPage: number
}

const defaultSearchMeta: ModSearchMeta = {
  page: 1,
  size: modSearchPageSize,
  total: 0,
  totalPage: 0,
}

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
  const [searchMode, setSearchMode] = useState<ModSearchMode>('keyword')
  const [searching, setSearching] = useState(false)
  const [searchResults, setSearchResults] = useState<ModSearchItem[]>([])
  const [searchMeta, setSearchMeta] = useState<ModSearchMeta>(defaultSearchMeta)
  const [subscriptionSearchLoaded, setSubscriptionSearchLoaded] = useState(false)
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

  async function handleSaveToLevel() {
    try {
      setActionLoading('save-level')
      const modData = createModOverridesLua(mods)
      assertApiSuccess(await saveGameConfig({ modData }))
      message.success(`${selectedLevelLabel}模组配置已保存，重启世界后生效`)
    } catch (error) {
      message.error(getErrorMessage(error, '保存世界模组失败'))
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

  function handleOpenOptions(mod: ModInfoRecord) {
    setSelectedModId(getModWorkshopId(mod))
    setOptionDrawerOpen(true)
  }

  async function handleUpdateMod(mod: ModInfoRecord) {
    const modId = getModWorkshopId(mod)
    if (!modId) {
      return
    }

    try {
      setSelectedModId(modId)
      setActionLoading(`update-${modId}`)
      const envelope = await updateMod(modId)
      const updatedMod = assertApiSuccess(envelope)
      setMods((current) => upsertMod(current, updatedMod))
      setSelectedModId(getModWorkshopId(updatedMod))
      message.success('模组已更新')
    } catch (error) {
      message.error(getErrorMessage(error, '更新模组失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  function handleConfigDefaultChange(entry: ModConfigEntry, nextDefault: unknown) {
    if (!selectedMod) {
      return
    }

    const modId = getModWorkshopId(selectedMod)
    const entryName = typeof entry.name === 'string' ? entry.name : ''
    if (!modId || !entryName) {
      return
    }

    setMods((current) =>
      current.map((mod) =>
        getModWorkshopId(mod) === modId
          ? {
              ...mod,
              mod_config: updateModConfigDefault(mod.mod_config, entryName, nextDefault),
            }
          : mod,
      ),
    )
  }

  async function handleSavePreferences() {
    if (!selectedMod) {
      return
    }

    try {
      setActionLoading('save-options')
      const envelope = await saveModInfo(selectedMod)
      const savedMod = assertApiSuccess(envelope)
      setMods((current) => upsertMod(current, savedMod))
      setSelectedModId(getModWorkshopId(savedMod))
      setOptionDrawerOpen(false)
      message.success('模组偏好已保存')
    } catch (error) {
      message.error(getErrorMessage(error, '保存模组偏好失败'))
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

  async function handleSearch(value = searchText, page = 1, mode = searchMode) {
    try {
      setSubscriptionSearchLoaded(true)
      setSearching(true)
      const searchValue = buildSearchQueryText(mode, value)
      const envelope = await searchMods(searchValue, page, modSearchPageSize)
      const result = assertApiSuccess(envelope)
      setSearchResults(result.data ?? [])
      setSearchMeta(normalizeSearchMeta(result, page, modSearchPageSize))
    } catch (error) {
      message.error(getErrorMessage(error, '搜索模组失败'))
    } finally {
      setSearching(false)
    }
  }

  async function handleSubscribe(item: ModSearchItem) {
    const modId = getSearchItemWorkshopId(item)
    if (!modId) {
      message.warning('无法识别模组 ID')
      return
    }

    try {
      setActionLoading(`subscribe-${modId}`)
      const envelope = await subscribeMod(modId)
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
    if (key === 'subscription' && !subscriptionSearchLoaded) {
      void handleSearch('', 1, 'all')
    }
    if (key === 'ugc') {
      void loadUgcMods()
    }
  }

  function handleSearchModeChange(mode: ModSearchMode) {
    setSearchMode(mode)
    if (mode === 'all') {
      setSearchText('')
      void handleSearch('', 1, mode)
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
                  <Button
                    type="primary"
                    className="mod-level-save"
                    loading={actionLoading === 'save-level'}
                    disabled={mods.length === 0}
                    onClick={() => void handleSaveToLevel()}
                  >
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
                                  type="primary"
                                  size="small"
                                  icon={<ToolOutlined />}
                                  onClick={() => handleOpenOptions(mod)}
                                >
                                  选项
                                </Button>
                                <Button
                                  type="primary"
                                  size="small"
                                  icon={<SyncOutlined />}
                                  loading={actionLoading === `update-${modId}`}
                                  onClick={() => void handleUpdateMod(mod)}
                                >
                                  更新
                                </Button>
                                <Popconfirm
                                  title={`确认删除 ${getModDisplayName(mod)}`}
                                  description="删除后会从当前模组列表中移除该配置。"
                                  okText="确认"
                                  cancelText="取消"
                                  onConfirm={() => void handleDelete(mod)}
                                >
                                  <Button
                                    danger
                                    type="link"
                                    size="small"
                                    icon={<DeleteOutlined />}
                                    loading={actionLoading === `delete-${modId}`}
                                  >
                                    删除
                                  </Button>
                                </Popconfirm>
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
                <Space className="mod-search-toolbar" wrap>
                  <Select
                    aria-label="查询方式"
                    className="mod-search-mode"
                    value={searchMode}
                    options={modSearchModeOptions}
                    onChange={handleSearchModeChange}
                  />
                  <Input.Search
                    className="mod-search-input"
                    enterButton={
                      <Button type="primary" icon={<SearchOutlined />}>
                        搜索
                      </Button>
                    }
                    disabled={searchMode === 'all'}
                    loading={searching}
                    placeholder="输入创意工坊 ID 或关键词"
                    value={searchText}
                    onChange={(event) => setSearchText(event.target.value)}
                    onSearch={(value) => void handleSearch(value, 1)}
                  />
                </Space>
                <div className="mod-search-grid">
                  {searchResults.map((item, index) => (
                    <article
                      className="mod-search-card"
                      key={getSearchItemWorkshopId(item) || item.name || `search-${index}`}
                    >
                      <img
                        src={getSearchItemImageUrl(item)}
                        alt={item.name || getSearchItemWorkshopId(item)}
                      />
                      <div className="mod-search-card-body">
                        <h3>{item.name || getSearchItemWorkshopId(item)}</h3>
                        <Typography.Text type="secondary">
                          作者: {item.author || '-'}
                        </Typography.Text>
                        <Typography.Text type="secondary">
                          评分: {formatSearchScore(item)}
                        </Typography.Text>
                        <Typography.Text type="secondary">
                          订阅: {formatSearchSubscription(item)}
                        </Typography.Text>
                        <Typography.Text type="secondary">{formatSearchTime(item)}</Typography.Text>
                        <Button
                          block
                          type="primary"
                          disabled={!getSearchItemWorkshopId(item)}
                          loading={actionLoading === `subscribe-${getSearchItemWorkshopId(item)}`}
                          onClick={() => void handleSubscribe(item)}
                        >
                          订阅
                        </Button>
                      </div>
                    </article>
                  ))}
                </div>
                {searchMeta.total > 0 ? (
                  <Pagination
                    className="mod-search-pagination"
                    current={searchMeta.page}
                    disabled={searching}
                    pageSize={searchMeta.size}
                    showSizeChanger={false}
                    showTotal={(total, range) => `${range[0]}-${range[1]} / ${total}`}
                    total={searchMeta.total}
                    onChange={(page) => void handleSearch(searchText, page)}
                  />
                ) : null}
                {!searching && subscriptionSearchLoaded && searchResults.length === 0 ? (
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
          <Button
            type="primary"
            loading={actionLoading === 'save-options'}
            onClick={() => void handleSavePreferences()}
          >
            保存偏好
          </Button>
        }
      >
        {selectedModOptions.length === 0 ? (
          <Empty description="暂无可配置选项" />
        ) : (
          <Form className="mod-option-form" labelCol={{ flex: '180px' }} wrapperCol={{ flex: 1 }}>
            {selectedModOptions.map((entry) => (
              <ModOptionControl
                key={getConfigEntryLabel(entry)}
                entry={entry}
                onChange={(nextValue) => handleConfigDefaultChange(entry, nextValue)}
              />
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

function ModOptionControl({
  entry,
  onChange,
}: {
  entry: ModConfigEntry
  onChange: (nextValue: unknown) => void
}) {
  const sourceOptions = Array.isArray(entry.options) ? entry.options : []
  const options = sourceOptions.map((option) => ({
    label: getConfigOptionLabel(option),
    value: configValueKey(option.data),
  }))

  return (
    <Form.Item label={getConfigEntryLabel(entry)}>
      <Select
        value={configValueKey(entry.default)}
        options={
          options.length > 0
            ? options
            : [
                {
                  label: stringifyConfigValue(entry.default),
                  value: configValueKey(entry.default),
                },
              ]
        }
        onChange={(value) => {
          const matchedOption = sourceOptions.find(
            (option) => configValueKey(option.data) === value,
          )
          onChange(matchedOption ? matchedOption.data : entry.default)
        }}
      />
    </Form.Item>
  )
}

function updateModConfigDefault(
  value: ModInfoRecord['mod_config'],
  entryName: string,
  nextDefault: unknown,
): ModInfoRecord['mod_config'] {
  const nextOptions = normalizeModConfig(value).map((entry) =>
    entry.name === entryName ? { ...entry, default: nextDefault } : entry,
  )

  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value)
      if (isRecord(parsed) && 'configuration_options' in parsed) {
        return JSON.stringify({ ...parsed, configuration_options: nextOptions })
      }
    } catch {
      return nextOptions
    }
  }

  if (isRecord(value) && 'configuration_options' in value) {
    return { ...value, configuration_options: nextOptions }
  }

  return nextOptions
}

function configValueKey(value: unknown): string {
  return JSON.stringify(value ?? null)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null
}

function getSearchItemWorkshopId(item: ModSearchItem): string {
  const value = item.modid ?? item.id ?? ''
  return formatWorkshopId(String(value))
}

function getSearchItemImageUrl(item: ModSearchItem): string {
  const img = typeof item.img === 'string' ? item.img.trim() : ''
  if (img && img !== 'xxx') {
    return img
  }

  return '/assets/dst/mods.png'
}

function formatSearchSubscription(item: ModSearchItem): string {
  const value = item.subscription ?? item.sub
  const text = value === undefined || value === null ? '' : String(value).trim()
  return !text || text.toLowerCase() === 'nan' ? '-' : text
}

function formatSearchScore(item: ModSearchItem): string {
  if (typeof item.score === 'number' && Number.isFinite(item.score)) {
    return String(item.score)
  }

  const star = item.vote?.star
  const num = item.vote?.num
  if (star === undefined || star === null || !Number.isFinite(Number(star))) {
    return '-'
  }
  if (num === undefined || num === null || num === 0) {
    return String(star)
  }
  return `${star}/${num}`
}

function formatSearchTime(item: ModSearchItem): string {
  const value = item.time ?? item.created ?? item.last_time
  if (value === undefined || value === null) {
    return '-'
  }

  if (typeof value === 'number') {
    return formatModUpdatedAt(value)
  }

  const text = String(value).trim()
  if (!text || text.toLowerCase() === 'nan') {
    return '-'
  }

  if (/^\d+(\.\d+)?$/.test(text)) {
    return formatModUpdatedAt(text)
  }

  return text
}

function buildSearchQueryText(mode: ModSearchMode, value: string): string {
  const text = value.trim()
  if (mode === 'all') {
    return ''
  }
  if (mode === 'workshopId') {
    return formatWorkshopId(text)
  }
  return text
}

function normalizeSearchMeta(
  response: {
    data?: ModSearchItem[]
    page?: number
    size?: number
    total?: number
    totalPage?: number
  },
  requestedPage: number,
  requestedSize: number,
): ModSearchMeta {
  const resultCount = response.data?.length ?? 0
  const page = positiveInteger(response.page, requestedPage)
  const size = positiveInteger(response.size, requestedSize)
  const total = Math.max(nonNegativeInteger(response.total, resultCount), resultCount)
  const totalPage = positiveInteger(response.totalPage, total > 0 ? Math.ceil(total / size) : 0)

  return { page, size, total, totalPage }
}

function positiveInteger(value: unknown, fallback: number): number {
  const numberValue = Number(value)
  if (!Number.isFinite(numberValue) || numberValue <= 0) {
    return fallback
  }
  return Math.trunc(numberValue)
}

function nonNegativeInteger(value: unknown, fallback: number): number {
  const numberValue = Number(value)
  if (!Number.isFinite(numberValue) || numberValue < 0) {
    return fallback
  }
  return Math.trunc(numberValue)
}

function upsertMod(mods: ModInfoRecord[], nextMod: ModInfoRecord): ModInfoRecord[] {
  const nextModId = getModWorkshopId(nextMod)
  if (mods.some((mod) => getModWorkshopId(mod) === nextModId)) {
    return mods.map((mod) => (getModWorkshopId(mod) === nextModId ? nextMod : mod))
  }

  return [nextMod, ...mods]
}
