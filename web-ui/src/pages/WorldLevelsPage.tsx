import { ProCard } from '@ant-design/pro-components'
import { Alert, App as AntApp, Button, Empty, Select, Space, Spin, Tabs } from 'antd'
import { useEffect, useMemo, useState } from 'react'

import { getLevels, type WorldLevel } from '@/features/levels/level.api'
import {
  buildWorldSettingGroups,
  getAtlasImageUrl,
  type WorldSettingGroup,
  type WorldSettingItem,
  type WorldSettingsDefinition,
} from '@/features/worlds/world-settings-model'
import { getWorldSettingsDefinition } from '@/features/maps/map.api'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

export default function WorldLevelsPage() {
  const { message } = AntApp.useApp()
  const [loading, setLoading] = useState(true)
  const [levels, setLevels] = useState<WorldLevel[]>([])
  const [definition, setDefinition] = useState<WorldSettingsDefinition | null>(null)
  const [activeLevel, setActiveLevel] = useState<string>()

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
    if (!definition) {
      return { worldSettings: [], worldGen: [] }
    }

    const worldKind = currentLevel?.uuid === 'Caves' ? 'cave' : 'forest'
    const source = definition.zh[worldKind] ?? definition.zh.forest
    return {
      worldSettings: buildWorldSettingGroups(source.WORLDSETTINGS_GROUP ?? {}),
      worldGen: buildWorldSettingGroups(source.WORLDGEN_GROUP ?? {}),
    }
  }, [currentLevel?.uuid, definition])

  return (
    <ProCard className="world-levels-card" bordered={false}>
      <Alert
        className="world-levels-alert"
        type="info"
        showIcon
        message="您可以双击世界标签名或右键点击标签选择「重命名」来修改世界名称，修改后记得点击保存按钮。"
      />
      {loading ? (
        <div className="page-loading">
          <Spin />
        </div>
      ) : levels.length === 0 ? (
        <Empty description="暂无世界" />
      ) : (
        <>
          <Tabs
            activeKey={currentLevel?.uuid}
            onChange={setActiveLevel}
            items={levels.map((level) => ({ key: level.uuid, label: level.levelName }))}
          />
          <Tabs
            items={[
              {
                key: 'settings',
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
                                children: renderSettingGroups(settingGroups.worldSettings),
                              },
                              {
                                key: 'world-gen',
                                label: '世界生成',
                                children: renderSettingGroups(settingGroups.worldGen),
                              },
                            ]}
                          />
                        ),
                      },
                      {
                        key: 'edit',
                        label: '编辑',
                        children: renderSettingGroups(settingGroups.worldSettings),
                      },
                    ]}
                  />
                ),
              },
              {
                key: 'mods',
                label: '模组设置',
                children: <Empty description="请选择多层选择器或模组设置页面管理世界模组" />,
              },
              {
                key: 'ports',
                label: '端口设置',
                children: (
                  <div className="world-port-grid">
                    <span>世界</span>
                    <strong>{currentLevel?.levelName}</strong>
                    <span>目录</span>
                    <strong>{currentLevel?.uuid}</strong>
                  </div>
                ),
              },
            ]}
          />
          <Space className="world-action-bar" wrap>
            <Button type="primary">保存</Button>
            <Button type="primary">添加世界</Button>
            <Button type="primary">导入</Button>
            <Button type="primary">下载</Button>
          </Space>
        </>
      )}
    </ProCard>
  )
}

function renderSettingGroups(groups: WorldSettingGroup[]) {
  if (groups.length === 0) {
    return <Empty description="暂无配置项" />
  }

  return (
    <div className="world-setting-groups">
      {groups.map((group) => (
        <section key={group.key} className="world-setting-section">
          <h3>{group.title}</h3>
          <div className="world-setting-grid">
            {group.items.slice(0, 18).map((item) => (
              <WorldSettingControl key={item.key} item={item} />
            ))}
          </div>
        </section>
      ))}
    </div>
  )
}

function WorldSettingControl({ item }: { item: WorldSettingItem }) {
  const iconStyle = getWorldIconStyle(item)

  return (
    <div className="world-setting-control">
      <span className="world-setting-icon" style={iconStyle} aria-hidden="true" />
      <div className="world-setting-field">
        <span>{item.label}</span>
        <Select size="middle" value={item.value} options={item.options} />
      </div>
    </div>
  )
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
