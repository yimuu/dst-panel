import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Button, Empty, Spin } from 'antd'
import { useEffect, useState } from 'react'

import { applyPreinstall } from '@/features/maps/map-state'
import { getErrorMessage } from '@/shared/api/envelope'

interface PreinstallTemplate {
  name: string
  description: string
  value: string
  src: string
}

export default function PreinstallPage() {
  const { message } = AntApp.useApp()
  const [loading, setLoading] = useState(true)
  const [templates, setTemplates] = useState<PreinstallTemplate[]>([])

  useEffect(() => {
    let ignore = false

    async function loadTemplates() {
      try {
        setLoading(true)
        const response = await fetch('/misc/preinstall.json')
        if (!response.ok) {
          throw new Error('加载世界模板失败')
        }
        const values = (await response.json()) as PreinstallTemplate[]
        if (!ignore) {
          setTemplates(values)
        }
      } catch (error) {
        if (!ignore) {
          message.error(getErrorMessage(error, '加载世界模板失败'))
        }
      } finally {
        if (!ignore) {
          setLoading(false)
        }
      }
    }

    void loadTemplates()

    return () => {
      ignore = true
    }
  }, [message])

  return (
    <ProCard title="世界模板" className="preinstall-card" bordered={false}>
      {loading ? (
        <div className="page-loading">
          <Spin />
        </div>
      ) : templates.length === 0 ? (
        <Empty description="暂无世界模板" />
      ) : (
        <div className="preinstall-grid">
          {templates.map((template) => (
            <article className="preinstall-template" key={template.value}>
              <div
                className="preinstall-cover"
                aria-label={template.name}
                style={{
                  backgroundImage: `url(${template.src}), url(/assets/dst/Celestial_Portal_Build.webp)`,
                }}
              />
              <div className="preinstall-body">
                <h3>{template.name}</h3>
                <p>{template.description}</p>
                <Button type="primary" onClick={() => void applyPreinstall(template.value)}>
                  应用模板
                </Button>
              </div>
            </article>
          ))}
        </div>
      )}
    </ProCard>
  )
}
