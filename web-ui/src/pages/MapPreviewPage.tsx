import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Button, Select, Space } from 'antd'
import { useMemo, useState } from 'react'

import { generateMap, getMapImageUrl } from '@/features/maps/map.api'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

export default function MapPreviewPage() {
  const { message } = AntApp.useApp()
  const [levelName, setLevelName] = useState('Master')
  const [version, setVersion] = useState(0)
  const [generating, setGenerating] = useState(false)
  const imageUrl = useMemo(() => {
    const baseUrl = getMapImageUrl(levelName)
    return version === 0 ? baseUrl : `${baseUrl}&v=${version}`
  }, [levelName, version])

  async function handleGenerate() {
    try {
      setGenerating(true)
      assertApiSuccess(await generateMap(levelName))
      setVersion((current) => current + 1)
      message.success('地图生成成功')
    } catch (error) {
      message.error(getErrorMessage(error, '地图生成失败'))
    } finally {
      setGenerating(false)
    }
  }

  return (
    <ProCard
      className="map-preview-card"
      title="预览地图"
      bordered={false}
      extra={
        <Space wrap>
          <Select
            value={levelName}
            onChange={setLevelName}
            options={[
              { label: '森林', value: 'Master' },
              { label: '洞穴', value: 'Caves' },
            ]}
          />
          <Button loading={generating} type="primary" onClick={() => void handleGenerate()}>
            生成地图
          </Button>
          <Button onClick={() => setVersion((current) => current + 1)}>刷新图片</Button>
        </Space>
      }
    >
      <div className="map-preview-panel">
        <img src={imageUrl} alt={`${levelName} 地图预览`} />
      </div>
    </ProCard>
  )
}
