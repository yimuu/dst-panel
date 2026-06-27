import {
  BellOutlined,
  GithubOutlined,
  QuestionCircleOutlined,
  UserOutlined,
} from '@ant-design/icons'
import { Avatar, Button, Space, Tooltip, Typography } from 'antd'

export function AppHeader() {
  return (
    <Space size={16} className="app-header-actions">
      <Tooltip title="通知">
        <Button type="text" shape="circle" icon={<BellOutlined />} />
      </Tooltip>
      <Tooltip title="帮助">
        <Button type="text" shape="circle" icon={<QuestionCircleOutlined />} />
      </Tooltip>
      <Tooltip title="Github">
        <Button
          type="text"
          shape="circle"
          href="https://github.com/carrot-hu23/dst-admin-go"
          target="_blank"
          rel="noreferrer"
          icon={<GithubOutlined />}
        />
      </Tooltip>
      <Space size={8}>
        <Avatar size={28} icon={<UserOutlined />} />
        <Typography.Text strong>admin</Typography.Text>
      </Space>
    </Space>
  )
}
