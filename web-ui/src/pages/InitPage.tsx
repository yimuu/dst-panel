import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Button, Form, Input } from 'antd'
import { useNavigate } from 'react-router'

import { initFirstRun } from '@/features/auth/auth.api'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'
import { routes } from '@/shared/config/routes'

interface InitFormValues {
  username: string
  password: string
}

export default function InitPage() {
  const navigate = useNavigate()
  const { message } = AntApp.useApp()

  return (
    <ProCard className="auth-card" title="初始化管理员" bordered={false}>
      <Form
        layout="vertical"
        onFinish={async (values: InitFormValues) => {
          try {
            assertApiSuccess(
              await initFirstRun({
                userInfo: {
                  username: values.username,
                  password: values.password,
                  displayName: values.username,
                  photoURL: '',
                },
              }),
            )
            message.success('初始化完成，请登录')
            navigate(routes.login, { replace: true })
          } catch (error) {
            message.error(getErrorMessage(error, '初始化失败'))
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
          初始化
        </Button>
      </Form>
    </ProCard>
  )
}
