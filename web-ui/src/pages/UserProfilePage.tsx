import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Avatar, Button, Form, Input, Space, Spin, Typography } from 'antd'
import { SaveOutlined, UserOutlined } from '@ant-design/icons'
import { useEffect, useState } from 'react'

import {
  changePassword,
  getCurrentUser,
  updateCurrentUser,
  type UpdateUserPayload,
} from '@/features/auth/auth.api'
import { normalizeUserProfileName, validateNewPassword } from '@/features/auth/user-profile'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'
import type { CurrentUser } from '@/shared/types/domain'

interface ProfileFormValues {
  displayName: string
  newPassword?: string
  password?: string
  photoURL: string
  username: string
}

export default function UserProfilePage() {
  const { message } = AntApp.useApp()
  const [form] = Form.useForm<ProfileFormValues>()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [user, setUser] = useState<CurrentUser>()

  useEffect(() => {
    let ignore = false

    async function loadUser() {
      try {
        setLoading(true)
        const nextUser = assertApiSuccess(await getCurrentUser())
        if (!ignore) {
          setUser(nextUser)
          form.setFieldsValue({
            username: nextUser.username ?? '',
            displayName: normalizeUserProfileName(nextUser),
            photoURL: nextUser.photoURL ?? '',
          })
        }
      } catch (error) {
        message.error(getErrorMessage(error, '加载用户资料失败'))
      } finally {
        if (!ignore) {
          setLoading(false)
        }
      }
    }

    void loadUser()
    return () => {
      ignore = true
    }
  }, [form, message])

  async function handleSave(values: ProfileFormValues) {
    const passwordError = validateNewPassword(values.newPassword ?? '')
    if (passwordError) {
      message.warning(passwordError)
      return
    }

    try {
      setSaving(true)
      if (values.newPassword?.trim()) {
        assertApiSuccess(await changePassword(values.newPassword.trim()))
      }
      const payload: UpdateUserPayload = {
        username: values.username,
        displayName: values.displayName,
        photoURL: values.photoURL,
        password: values.password ?? '',
      }
      if (payload.password) {
        assertApiSuccess(await updateCurrentUser(payload))
      }
      message.success('用户资料已保存')
    } catch (error) {
      message.error(getErrorMessage(error, '保存用户资料失败'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <ProCard title="个人资料" className="profile-page-card" bordered={false}>
      {loading ? (
        <div className="page-loading">
          <Spin />
        </div>
      ) : (
        <>
          <Space className="profile-summary" size={16}>
            <Avatar size={56} src={user?.photoURL} icon={<UserOutlined />} />
            <div>
              <Typography.Title level={4}>{normalizeUserProfileName(user ?? {})}</Typography.Title>
              <Typography.Text type="secondary">{user?.username ?? 'admin'}</Typography.Text>
            </div>
          </Space>
          <Form form={form} layout="vertical" className="profile-form" onFinish={handleSave}>
            <Form.Item label="用户名" name="username" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item label="显示名称" name="displayName" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item label="头像地址" name="photoURL">
              <Input />
            </Form.Item>
            <Form.Item label="保存资料需要输入当前密码" name="password">
              <Input.Password />
            </Form.Item>
            <Form.Item label="新密码" name="newPassword">
              <Input.Password placeholder="不修改则留空" />
            </Form.Item>
            <Button type="primary" htmlType="submit" icon={<SaveOutlined />} loading={saving}>
              保存资料
            </Button>
          </Form>
        </>
      )}
    </ProCard>
  )
}
