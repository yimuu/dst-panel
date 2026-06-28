import { ProCard } from '@ant-design/pro-components'
import {
  Alert,
  App as AntApp,
  Button,
  Empty,
  Form,
  Input,
  InputNumber,
  Space,
  Spin,
  Switch,
  Table,
  Tabs,
} from 'antd'
import type { InputRef } from 'antd/es/input'
import { useEffect, useMemo, useRef, useState } from 'react'

import { getLevels, saveLevels, type WorldLevel } from '@/features/levels/level.api'
import {
  DEFAULT_SELECTOR_WORKSHOP_ID,
  applySelectorWorldConfigToModoverrides,
  normalizeSelectorWorkshopId,
  parseSelectorWorldConfig,
  worldConfigFromLevels,
  type SelectorWorldConfig,
} from '@/features/levels/selector-mod'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

export default function WorldModSelectionPage() {
  const { message } = AntApp.useApp()
  const [form] = Form.useForm<{ world_config: SelectorWorldConfig[] }>()
  const inputRef = useRef<InputRef>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [levels, setLevels] = useState<WorldLevel[]>([])
  const [workshopId, setWorkshopId] = useState(() => storedWorkshopId())

  useEffect(() => {
    let ignore = false

    async function loadLevels() {
      try {
        setLoading(true)
        const nextLevels = assertApiSuccess(await getLevels())
        if (ignore) {
          return
        }
        setLevels(nextLevels)
        form.setFieldsValue({
          world_config: parseSelectorWorldConfig(
            nextLevels[0]?.modoverrides ?? 'return {}',
            workshopId,
          ),
        })
      } catch (error) {
        if (!ignore) {
          message.error(getErrorMessage(error, '加载多层选择器配置失败'))
        }
      } finally {
        if (!ignore) {
          setLoading(false)
        }
      }
    }

    void loadLevels()

    return () => {
      ignore = true
    }
  }, [form, message, workshopId])

  const previewRows = useMemo(() => worldConfigFromLevels(levels), [levels])

  function refreshWorkshopId(value?: string) {
    const normalized = normalizeSelectorWorkshopId(value ?? inputRef.current?.input?.value)
    setWorkshopId(normalized)
    form.setFieldsValue({
      world_config: parseSelectorWorldConfig(levels[0]?.modoverrides ?? 'return {}', normalized),
    })
  }

  function setDefaultWorkshopId() {
    localStorage.setItem('workshop', DEFAULT_SELECTOR_WORKSHOP_ID)
    refreshWorkshopId(DEFAULT_SELECTOR_WORKSHOP_ID)
    message.success('默认多层选择器已设置')
  }

  function syncCurrentLevels() {
    form.setFieldsValue({ world_config: previewRows })
    message.success('已同步当前世界列表')
  }

  async function saveSelectorConfig() {
    try {
      const values = await form.validateFields()
      const rows = (values.world_config ?? []).filter((row) => row?.id && row?.name)
      if (rows.length === 0) {
        message.warning('请至少添加一个世界配置')
        return
      }

      setSaving(true)
      const nextLevels = levels.map((level) => ({
        ...level,
        modoverrides: applySelectorWorldConfigToModoverrides(
          level.modoverrides,
          workshopId,
          rows,
        ),
      }))
      assertApiSuccess(await saveLevels(nextLevels))
      setLevels(nextLevels)
      message.success('保存成功')
    } catch (error) {
      if (error instanceof Error) {
        message.error(getErrorMessage(error, '保存失败'))
      }
    } finally {
      setSaving(false)
    }
  }

  return (
    <ProCard className="selector-mod-card" bordered={false}>
      <Spin spinning={loading}>
        <Tabs
          items={[
            {
              key: 'selector',
              label: '多层选择器',
              children: (
                <>
                  <Alert
                    type="info"
                    showIcon
                    message="目前只兼容 [WIP] 又是一个世界选择器 workshop-1754389029 这种格式的配置"
                    action={
                      <a
                        target="_blank"
                        rel="noreferrer"
                        href="https://steamcommunity.com/sharedfiles/filedetails/?id=1754389029"
                      >
                        详细
                      </a>
                    }
                  />
                  <Space className="selector-toolbar" wrap>
                    <Input
                      ref={inputRef}
                      defaultValue={workshopId}
                      placeholder="多层选择器模组id"
                    />
                    <Button type="primary" onClick={() => refreshWorkshopId()}>
                      刷新
                    </Button>
                    <Button type="primary" onClick={setDefaultWorkshopId}>
                      设置默认多层选择器
                    </Button>
                  </Space>
                  <SelectorConfigForm form={form} />
                  <Button type="primary" loading={saving} onClick={() => void saveSelectorConfig()}>
                    保存配置
                  </Button>
                </>
              ),
            },
            {
              key: 'sync',
              label: '世界配置同步',
              children: (
                <div className="selector-sync-panel">
                  {levels.length === 0 ? (
                    <Empty description="暂无世界" />
                  ) : (
                    <>
                      <Table
                        size="small"
                        pagination={false}
                        rowKey="id"
                        dataSource={previewRows}
                        columns={[
                          { title: '世界id', dataIndex: 'id' },
                          { title: '世界名称', dataIndex: 'name' },
                          { title: '分类', dataIndex: 'category' },
                          {
                            title: '洞穴',
                            dataIndex: 'is_cave',
                            render: (value: boolean) => (value ? '是' : '否'),
                          },
                        ]}
                      />
                      <Button
                        className="selector-sync-button"
                        type="primary"
                        onClick={syncCurrentLevels}
                      >
                        同步当前世界列表
                      </Button>
                    </>
                  )}
                </div>
              ),
              forceRender: true,
            },
          ]}
        />
      </Spin>
    </ProCard>
  )
}

function SelectorConfigForm({
  form,
}: {
  form: ReturnType<typeof Form.useForm<{ world_config: SelectorWorldConfig[] }>>[0]
}) {
  return (
    <Form className="selector-config-form" form={form}>
      <Form.List name="world_config">
        {(fields, { add, remove }) => (
          <>
            {fields.map(({ key, name, ...restField }) => (
              <div className="selector-config-row" key={key}>
                <Form.Item
                  {...restField}
                  label="世界id"
                  name={[name, 'id']}
                  rules={[{ required: true, message: '缺失世界id' }]}
                >
                  <Input placeholder="世界id" />
                </Form.Item>
                <Form.Item
                  {...restField}
                  label="世界名称"
                  name={[name, 'name']}
                  rules={[{ required: true, message: '缺失世界名称' }]}
                >
                  <Input placeholder="世界名称，不允许换行" />
                </Form.Item>
                <Form.Item {...restField} label="分类" name={[name, 'category']}>
                  <Input placeholder="世界类别，用于筛选，将显示于左侧菜单" />
                </Form.Item>
                <Form.Item {...restField} label="提示信息" name={[name, 'note']}>
                  <Input placeholder="鼠标悬停显示的提示信息" />
                </Form.Item>
                <Form.Item {...restField} label="人数" name={[name, 'galleryful']}>
                  <InputNumber min={1} max={64} placeholder="世界人数限制" />
                </Form.Item>
                <Form.Item {...restField} label="不分流" name={[name, 'extra']} valuePropName="checked">
                  <Switch checkedChildren="是" unCheckedChildren="否" />
                </Form.Item>
                <Form.Item {...restField} label="洞穴" name={[name, 'is_cave']} valuePropName="checked">
                  <Switch checkedChildren="是" unCheckedChildren="否" />
                </Form.Item>
                <Form.Item
                  {...restField}
                  label="不可见"
                  name={[name, 'invisible']}
                  valuePropName="checked"
                >
                  <Switch checkedChildren="是" unCheckedChildren="否" />
                </Form.Item>
                <Button danger onClick={() => remove(name)}>
                  删除
                </Button>
              </div>
            ))}
            <Form.Item>
              <Button className="selector-add-field" type="dashed" onClick={() => add()} block>
                添加字段
              </Button>
            </Form.Item>
          </>
        )}
      </Form.List>
    </Form>
  )
}

function storedWorkshopId(): string {
  if (typeof localStorage === 'undefined') {
    return DEFAULT_SELECTOR_WORKSHOP_ID
  }
  return normalizeSelectorWorkshopId(localStorage.getItem('workshop') ?? DEFAULT_SELECTOR_WORKSHOP_ID)
}
