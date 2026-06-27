import { ProCard } from '@ant-design/pro-components'
import { Button, Typography } from 'antd'
import { BookOutlined } from '@ant-design/icons'

const helpDocs = [
  {
    title: 'Docker Compose',
    description: '容器部署、目录映射和启动配置说明。',
    href: '/misc/Docker-compose.md',
  },
  {
    title: '多世界教程',
    description: '多机器、多世界和常见联机配置说明。',
    href: '/misc/DontStarveMultiWorldTotorial.md',
  },
  {
    title: '常见问题',
    description: '面板使用、服务器安装和路径配置问题。',
    href: '/misc/FQA.md',
  },
]

export default function HelpPage() {
  return (
    <ProCard title="帮助文档" className="help-page-card" bordered={false}>
      <div className="help-grid">
        {helpDocs.map((doc) => (
          <article key={doc.href} className="help-doc-card">
            <BookOutlined />
            <div>
              <h3>{doc.title}</h3>
              <Typography.Paragraph type="secondary">{doc.description}</Typography.Paragraph>
              <Button href={doc.href} target="_blank" rel="noreferrer">
                打开文档
              </Button>
            </div>
          </article>
        ))}
      </div>
    </ProCard>
  )
}
