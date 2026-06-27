import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Button, Form, Input } from 'antd'
import { useNavigate } from 'react-router'

import { login, type LoginPayload } from '@/features/auth/auth.api'
import { markAuthenticated } from '@/features/auth/auth-state'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'
import { routes } from '@/shared/config/routes'

export default function LoginPage() {
  const navigate = useNavigate()
  const { message } = AntApp.useApp()

  return (
    <ProCard className="auth-card" title="登录" bordered={false}>
      <Form
        layout="vertical"
        onFinish={async (values: LoginPayload) => {
          try {
            assertApiSuccess(await login(values))
            markAuthenticated()
            navigate(routes.panel, { replace: true })
          } catch (error) {
            message.error(getErrorMessage(error, '登录失败'))
          }
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
          登录
        </Button>
      </Form>
    </ProCard>
  )
}
