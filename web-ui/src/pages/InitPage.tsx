import { ProCard } from '@ant-design/pro-components'
import { Button, Form, Input } from 'antd'
import { useNavigate } from 'react-router'

import { markAuthenticated } from '@/features/auth/auth-state'
import { routes } from '@/shared/config/routes'

export default function InitPage() {
  const navigate = useNavigate()

  return (
    <ProCard className="auth-card" title="初始化管理员" bordered={false}>
      <Form
        layout="vertical"
        onFinish={() => {
          markAuthenticated()
          navigate(routes.panel, { replace: true })
        }}
      >
        <Form.Item
          label="用户名"
          name="username"
          rules={[{ required: true, message: '请输入用户名' }]}
        >
          <Input placeholder="请输入用户名" />
        </Form.Item>
        <Form.Item label="密码" name="password" rules={[{ required: true, message: '请输入密码' }]}>
          <Input.Password placeholder="请输入密码" />
        </Form.Item>
        <Button type="primary" htmlType="submit" block>
          初始化
        </Button>
      </Form>
    </ProCard>
  )
}
