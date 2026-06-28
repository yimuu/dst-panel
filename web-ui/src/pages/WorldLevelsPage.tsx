import { ProCard } from '@ant-design/pro-components'
import {
  Alert,
  App as AntApp,
  Button,
  Empty,
  Form,
  Input,
  InputNumber,
  Modal,
  Popconfirm,
  Radio,
  Select,
  Space,
  Spin,
  Switch,
  Tabs,
  Upload,
  type TabsProps,
  type UploadProps,
} from 'antd'
import { useEffect, useMemo, useState } from 'react'
import { Link } from 'react-router'

import {
  createLevel,
  deleteLevel,
  getLevels,
  saveLevels,
  type WorldLevel,
} from '@/features/levels/level.api'
import { getWorldSettingsDefinition } from '@/features/maps/map.api'
import {
  buildWorldSettingGroups,
  getAtlasImageUrl,
  parseWorldLocation,
  parseWorldOverrideValues,
  updateWorldOverrideValue,
  type WorldSettingGroup,
  type WorldSettingItem,
  type WorldSettingsDefinition,
} from '@/features/worlds/world-settings-model'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'
import { routes } from '@/shared/config/routes'
import type { ServerIniPayload } from '@/shared/types/domain'

type LevelType = 'forest' | 'cave' | 'porkland'

interface AddLevelFormValues {
  levelName: string
  uuid?: string
  type: LevelType
}

export default function WorldLevelsPage() {
  const { message } = AntApp.useApp()
  const [addForm] = Form.useForm<AddLevelFormValues>()
  const [loading, setLoading] = useState(true)
  const [levels, setLevels] = useState<WorldLevel[]>([])
  const [definition, setDefinition] = useState<WorldSettingsDefinition | null>(null)
  const [activeLevel, setActiveLevel] = useState<string>()
  const [actionLoading, setActionLoading] = useState<string>()
  const [addOpen, setAddOpen] = useState(false)

  useEffect(() => {
    let ignore = false

    async function loadWorldPage() {
      try {
        setLoading(true)
        const [levelEnvelope, worldDefinition] = await Promise.all([
          getLevels(),
          getWorldSettingsDefinition(),
        ])
        const nextLevels = assertApiSuccess(levelEnvelope)
        if (!ignore) {
          setLevels(nextLevels)
          setDefinition(worldDefinition)
          setActiveLevel((current) => current ?? nextLevels[0]?.uuid)
        }
      } catch (error) {
        if (!ignore) {
          message.error(getErrorMessage(error, '加载世界设置失败'))
        }
      } finally {
        if (!ignore) {
          setLoading(false)
        }
      }
    }

    void loadWorldPage()

    return () => {
      ignore = true
    }
  }, [message])

  const currentLevel = levels.find((level) => level.uuid === activeLevel) ?? levels[0]
  const settingGroups = useMemo(() => {
    if (!definition || !currentLevel) {
      return { worldSettings: [], worldGen: [] }
    }

    const worldKind = parseWorldLocation(currentLevel.leveldataoverride)
    const source = definition.zh[worldKind] ?? definition.zh.forest
    const values = parseWorldOverrideValues(currentLevel.leveldataoverride)
    return {
      worldSettings: buildWorldSettingGroups(source.WORLDSETTINGS_GROUP ?? {}, values),
      worldGen: buildWorldSettingGroups(source.WORLDGEN_GROUP ?? {}, values),
    }
  }, [currentLevel, definition])

  function updateLevel(uuid: string, patch: Partial<WorldLevel>) {
    setLevels((current) =>
      current.map((level) => (level.uuid === uuid ? { ...level, ...patch } : level)),
    )
  }

  function updateServerIni(uuid: string, patch: Partial<ServerIniPayload>) {
    setLevels((current) =>
      current.map((level) =>
        level.uuid === uuid
          ? {
              ...level,
              is_master: patch.is_master ?? level.is_master,
              server_ini: { ...level.server_ini, ...patch },
            }
          : level,
      ),
    )
  }

  function handleWorldSettingChange(item: WorldSettingItem, value: string) {
    if (!currentLevel) {
      return
    }
    updateLevel(currentLevel.uuid, {
      leveldataoverride: updateWorldOverrideValue(currentLevel.leveldataoverride, item.key, value),
    })
  }

  async function handleSaveLevels() {
    try {
      setActionLoading('save')
      assertApiSuccess(await saveLevels(levels))
      message.success('世界设置已保存')
    } catch (error) {
      message.error(getErrorMessage(error, '保存世界设置失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleAddLevel() {
    try {
      const values = await addForm.validateFields()
      setActionLoading('create')
      const created = assertApiSuccess(await createLevel(createDefaultLevel(levels, values)))
      setLevels((current) => [...current, created])
      setActiveLevel(created.uuid)
      setAddOpen(false)
      addForm.resetFields()
      message.success('世界已添加')
    } catch (error) {
      if (error instanceof Error) {
        message.error(getErrorMessage(error, '添加世界失败'))
      }
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleDeleteLevel(level: WorldLevel) {
    if (level.uuid === 'Master') {
      message.warning('Master 世界不能删除')
      return
    }

    try {
      setActionLoading(`delete:${level.uuid}`)
      assertApiSuccess(await deleteLevel(level.uuid))
      setLevels((current) => current.filter((item) => item.uuid !== level.uuid))
      setActiveLevel((current) => (current === level.uuid ? levels[0]?.uuid : current))
      message.success('世界已删除')
    } catch (error) {
      message.error(getErrorMessage(error, '删除世界失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleImportLevel(file: File) {
    try {
      setActionLoading('import')
      const imported = JSON.parse(await file.text()) as WorldLevel
      if (!imported.uuid || !imported.levelName) {
        throw new Error('世界配置文件缺少 uuid 或 levelName')
      }

      const created = assertApiSuccess(await createLevel(imported))
      setLevels((current) => [...current, created])
      setActiveLevel(created.uuid)
      message.success('世界已导入')
    } catch (error) {
      message.error(getErrorMessage(error, '导入世界失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  function handleDownloadLevel() {
    if (!currentLevel) {
      message.warning('请选择世界')
      return
    }

    const blob = new Blob([JSON.stringify(currentLevel, null, 2)], {
      type: 'application/json;charset=utf-8',
    })
    const url = URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = `${currentLevel.uuid}.json`
    link.click()
    URL.revokeObjectURL(url)
  }

  function openAddModal() {
    addForm.setFieldsValue({
      levelName: '新世界',
      uuid: `World${nextLevelIndex(levels)}`,
      type: 'forest',
    })
    setAddOpen(true)
  }

  const uploadProps: UploadProps = {
    accept: '.json',
    beforeUpload: (file) => {
      void handleImportLevel(file)
      return Upload.LIST_IGNORE
    },
    showUploadList: false,
  }

  const levelTabItems: TabsProps['items'] = levels.map((level) => ({
    key: level.uuid,
    label: level.levelName,
    closable: level.uuid !== 'Master',
    children: (
      <LevelEditor
        level={level}
        settingGroups={settingGroups}
        onLuaChange={(field, value) => updateLevel(level.uuid, { [field]: value })}
        onServerIniChange={(patch) => updateServerIni(level.uuid, patch)}
        onWorldSettingChange={handleWorldSettingChange}
      />
    ),
  }))

  return (
    <ProCard className="world-levels-card" bordered={false}>
      {loading ? (
        <div className="page-loading">
          <Spin />
        </div>
      ) : levels.length === 0 ? (
        <Empty description="当前没有世界，请点击添加世界" />
      ) : (
        <>
          <Tabs
            className="world-level-tabs"
            type="editable-card"
            hideAdd
            activeKey={currentLevel?.uuid}
            onChange={setActiveLevel}
            items={levelTabItems}
            onEdit={(targetKey, action) => {
              if (action === 'add') {
                openAddModal()
                return
              }
              const level = levels.find((item) => item.uuid === targetKey)
              if (level) {
                void handleDeleteLevel(level)
              }
            }}
          />
          <Space className="world-action-bar" wrap>
            <Button
              type="primary"
              loading={actionLoading === 'save'}
              onClick={() => void handleSaveLevels()}
            >
              保存
            </Button>
            <Button
              type="primary"
              loading={actionLoading === 'create'}
              onClick={openAddModal}
            >
              添加世界
            </Button>
            <Upload {...uploadProps}>
              <Button type="primary" loading={actionLoading === 'import'}>
                导入
              </Button>
            </Upload>
            <Button type="primary" onClick={handleDownloadLevel}>
              下载
            </Button>
            {currentLevel && currentLevel.uuid !== 'Master' ? (
              <Popconfirm
                title={`是否删除 ${currentLevel.levelName} 世界`}
                description="删除之前请确认已经保存好数据。"
                okText="删除"
                cancelText="取消"
                onConfirm={() => void handleDeleteLevel(currentLevel)}
              >
                <Button danger loading={actionLoading === `delete:${currentLevel.uuid}`}>
                  删除当前世界
                </Button>
              </Popconfirm>
            ) : null}
          </Space>
        </>
      )}
      <Modal
        title="添加世界"
        open={addOpen}
        onOk={() => void handleAddLevel()}
        confirmLoading={actionLoading === 'create'}
        onCancel={() => setAddOpen(false)}
      >
        <Alert
          type="warning"
          showIcon
          message="特殊字符不要用，世界名称仅用于显示。文件名需以英文开头，且不能是已有文件名的一部分。"
        />
        <Form className="world-add-form" form={addForm} layout="vertical">
          <Form.Item label="世界名" name="levelName" rules={[{ required: true, message: '请输入世界名' }]}>
            <Input placeholder="请输入世界名" />
          </Form.Item>
          <Form.Item label="世界文件名" name="uuid" rules={[{ required: true, message: '请输入文件名' }]}>
            <Input placeholder="请输入文件名" />
          </Form.Item>
          <Form.Item label="世界类型" name="type" rules={[{ required: true, message: '请选择类型' }]}>
            <Radio.Group>
              <Radio value="forest">森林</Radio>
              <Radio value="cave">洞穴</Radio>
              <Radio value="porkland">猪镇</Radio>
            </Radio.Group>
          </Form.Item>
        </Form>
      </Modal>
    </ProCard>
  )
}

interface LevelEditorProps {
  level: WorldLevel
  settingGroups: {
    worldSettings: WorldSettingGroup[]
    worldGen: WorldSettingGroup[]
  }
  onLuaChange: (field: 'leveldataoverride' | 'modoverrides', value: string) => void
  onServerIniChange: (patch: Partial<ServerIniPayload>) => void
  onWorldSettingChange: (item: WorldSettingItem, value: string) => void
}

function LevelEditor({
  level,
  settingGroups,
  onLuaChange,
  onServerIniChange,
  onWorldSettingChange,
}: LevelEditorProps) {
  return (
    <Tabs
      items={[
        {
          key: 'leveldataoverride',
          label: '世界设置',
          children: (
            <Tabs
              items={[
                {
                  key: 'view',
                  label: '查看',
                  children: (
                    <Tabs
                      items={[
                        {
                          key: 'world-settings',
                          label: '世界设置',
                          children: renderSettingGroups(
                            settingGroups.worldSettings,
                            onWorldSettingChange,
                          ),
                        },
                        {
                          key: 'world-gen',
                          label: '世界生成',
                          children: renderSettingGroups(settingGroups.worldGen, onWorldSettingChange),
                        },
                      ]}
                    />
                  ),
                },
                {
                  key: 'edit',
                  label: '编辑',
                  children: (
                    <LuaCodeEditor
                      aria-label={`${level.levelName} leveldataoverride.lua`}
                      fileName="leveldataoverride.lua"
                      testId="lua-code-editor-leveldataoverride"
                      value={level.leveldataoverride}
                      onChange={(value) => onLuaChange('leveldataoverride', value)}
                    />
                  ),
                },
              ]}
            />
          ),
        },
        {
          key: 'modoverrides',
          label: '模组设置',
          children: (
            <div className="world-modoverrides-panel">
              <LuaCodeEditor
                aria-label={`${level.levelName} modoverrides.lua`}
                fileName="modoverrides.lua"
                testId="lua-code-editor-modoverrides"
                value={level.modoverrides}
                onChange={(value) => onLuaChange('modoverrides', value)}
              />
              <Button type="link">
                <Link to={routes.selectorMod}>打开多层选择器</Link>
              </Button>
            </div>
          ),
        },
        {
          key: 'server-ini',
          label: '端口设置',
          children: (
            <ServerIniForm
              level={level}
              serverIni={level.server_ini}
              onServerIniChange={onServerIniChange}
            />
          ),
        },
      ]}
    />
  )
}

function LuaCodeEditor({
  'aria-label': ariaLabel,
  fileName,
  testId,
  value,
  onChange,
}: {
  'aria-label': string
  fileName: string
  testId: string
  value: string
  onChange: (value: string) => void
}) {
  const lineNumbers = getCodeLineNumbers(value)

  return (
    <div className="world-code-editor" data-testid={testId}>
      <div className="world-code-editor-header">
        <span>{fileName}</span>
        <span>{lineNumbers.length} 行</span>
      </div>
      <div className="world-code-editor-body">
        <div className="world-code-line-numbers" aria-hidden="true">
          {lineNumbers.map((lineNumber) => (
            <span key={lineNumber}>{lineNumber}</span>
          ))}
        </div>
        <Input.TextArea
          aria-label={ariaLabel}
          autoCapitalize="off"
          autoComplete="off"
          autoCorrect="off"
          className="world-lua-editor"
          rows={18}
          spellCheck={false}
          value={value}
          wrap="off"
          onChange={(event) => onChange(event.target.value)}
        />
      </div>
    </div>
  )
}

function ServerIniForm({
  level,
  serverIni,
  onServerIniChange,
}: {
  level: WorldLevel
  serverIni: ServerIniPayload
  onServerIniChange: (patch: Partial<ServerIniPayload>) => void
}) {
  return (
    <Form className="world-server-form" layout="vertical">
      <section className="world-server-section">
        <div className="world-server-section-title">
          <h3>基础信息</h3>
          <span>
            {level.levelName} / {level.uuid}
          </span>
        </div>
        <div className="world-server-grid">
          <Form.Item className="world-server-field world-server-field-wide" label="名称">
            <Input
              value={serverIni.name}
              onChange={(event) => onServerIniChange({ name: event.target.value })}
            />
          </Form.Item>
          <Form.Item className="world-server-field" label="ID">
            <InputNumber
              value={serverIni.id}
              onChange={(value) => onServerIniChange({ id: Number(value ?? 0) })}
            />
          </Form.Item>
          <Form.Item className="world-server-field world-server-switch-field" label="是否主世界">
            <Switch
              checked={serverIni.is_master}
              checkedChildren="是"
              unCheckedChildren="否"
              onChange={(checked) => onServerIniChange({ is_master: checked })}
            />
          </Form.Item>
          <Form.Item className="world-server-field world-server-switch-field" label="用户路径编码">
            <Switch
              checked={serverIni.encode_user_path}
              checkedChildren="开"
              unCheckedChildren="关"
              onChange={(checked) => onServerIniChange({ encode_user_path: checked })}
            />
          </Form.Item>
        </div>
      </section>

      <section className="world-server-section">
        <div className="world-server-section-title">
          <h3>网络端口</h3>
          <span>保存后写入当前世界的 server.ini</span>
        </div>
        <div className="world-server-grid">
          <Form.Item className="world-server-field" label="服务器端口">
            <InputNumber
              aria-label={`${level.levelName} server_port`}
              min={1}
              max={65535}
              value={serverIni.server_port}
              onChange={(value) => onServerIniChange({ server_port: Number(value ?? 0) })}
            />
          </Form.Item>
          <Form.Item className="world-server-field" label="认证端口">
            <InputNumber
              min={1}
              max={65535}
              value={serverIni.authentication_port}
              onChange={(value) => onServerIniChange({ authentication_port: Number(value ?? 0) })}
            />
          </Form.Item>
          <Form.Item className="world-server-field" label="主服务器端口">
            <InputNumber
              min={1}
              max={65535}
              value={serverIni.master_server_port}
              onChange={(value) => onServerIniChange({ master_server_port: Number(value ?? 0) })}
            />
          </Form.Item>
        </div>
      </section>
    </Form>
  )
}

function renderSettingGroups(
  groups: WorldSettingGroup[],
  onWorldSettingChange: (item: WorldSettingItem, value: string) => void,
) {
  if (groups.length === 0) {
    return <Empty description="暂无配置项" />
  }

  return (
    <div className="world-setting-groups">
      {groups.map((group) => (
        <section key={group.key} className="world-setting-section">
          <h3>{group.title}</h3>
          <div className="world-setting-grid">
            {group.items.map((item) => (
              <WorldSettingControl
                key={item.key}
                item={item}
                onChange={(value) => onWorldSettingChange(item, value)}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  )
}

function WorldSettingControl({
  item,
  onChange,
}: {
  item: WorldSettingItem
  onChange: (value: string) => void
}) {
  const iconStyle = getWorldIconStyle(item)

  return (
    <div className="world-setting-control">
      <span className="world-setting-icon" style={iconStyle} aria-hidden="true" />
      <div className="world-setting-field">
        <span title={item.label}>{item.label}</span>
        <Select size="middle" value={item.value} options={item.options} onChange={onChange} />
      </div>
    </div>
  )
}

function getCodeLineNumbers(value: string): number[] {
  const lineCount = Math.max(1, value.split(/\r\n|\r|\n/).length)
  return Array.from({ length: lineCount }, (_, index) => index + 1)
}

function getWorldIconStyle(item: WorldSettingItem) {
  const size = 54
  const scale = size / item.atlas.item_size
  const image = item.image ?? { x: 0, y: 0 }

  return {
    backgroundImage: `url(${getAtlasImageUrl(item.atlasName)})`,
    backgroundSize: `${item.atlas.width * scale}px ${item.atlas.height * scale}px`,
    backgroundPosition: `${-(image.x * item.atlas.width * scale)}px ${-(
      image.y *
      item.atlas.height *
      scale
    )}px`,
  }
}

function createDefaultLevel(levels: WorldLevel[], values: AddLevelFormValues): WorldLevel {
  const uuid = values.uuid?.trim() || `World${nextLevelIndex(levels)}`
  const id = Math.max(0, ...levels.map((level) => level.server_ini?.id ?? 0)) + 1
  const isCave = values.type === 'cave'

  return {
    levelName: values.levelName.trim(),
    is_master: false,
    uuid,
    leveldataoverride: defaultLevelData(values.type),
    modoverrides: copyMasterModOverrides(levels),
    server_ini: {
      server_port: nextPort(levels, 'server_port', isCave ? 11000 : 11001),
      is_master: false,
      name: uuid,
      id,
      encode_user_path: true,
      authentication_port: nextPort(levels, 'authentication_port', 8766),
      master_server_port: nextPort(levels, 'master_server_port', 27016),
    },
  }
}

function defaultLevelData(type: LevelType): string {
  const location = type === 'cave' ? 'cave' : type === 'porkland' ? 'porkland' : 'forest'
  return `return {
  location = "${location}",
  overrides = {},
}`
}

function copyMasterModOverrides(levels: WorldLevel[]): string {
  return levels.find((level) => level.uuid === 'Master')?.modoverrides || 'return {}'
}

function nextLevelIndex(levels: WorldLevel[]): number {
  let index = levels.length + 1
  while (levels.some((level) => level.uuid === `World${index}`)) {
    index += 1
  }

  return index
}

function nextPort(
  levels: WorldLevel[],
  key: 'server_port' | 'authentication_port' | 'master_server_port',
  fallback: number,
): number {
  return Math.max(fallback - 1, ...levels.map((level) => level.server_ini?.[key] ?? 0)) + 1
}
