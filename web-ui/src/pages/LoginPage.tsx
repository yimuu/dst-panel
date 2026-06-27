import { ProCard } from '@ant-design/pro-components'
import { Button, Form, Input } from 'antd'

export default function LoginPage() {
  return (
    <ProCard className="auth-card" title="登录" bordered={false}>
      <Form layout="vertical">
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
