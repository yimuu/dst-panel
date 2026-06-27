import { ProCard } from '@ant-design/pro-components'
import { App as AntApp, Button, Form, Input, Radio, Spin } from 'antd'
import { SaveOutlined } from '@ant-design/icons'
import { useEffect, useState } from 'react'

import { getDstConfig, saveDstConfig } from '@/features/settings/settings.api'
import {
  normalizeDstConfig,
  serializeDstConfig,
  validateDstConfig,
} from '@/features/settings/settings-form'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'
import type { DstConfig } from '@/shared/types/domain'

export default function SettingsPage() {
  const { message } = AntApp.useApp()
  const [form] = Form.useForm<DstConfig>()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    let ignore = false

    async function loadConfig() {
      try {
        setLoading(true)
        const config = normalizeDstConfig(assertApiSuccess(await getDstConfig()))
        if (!ignore) {
          form.setFieldsValue(config)
        }
      } catch (error) {
        message.error(getErrorMessage(error, '加载系统设置失败'))
      } finally {
        if (!ignore) {
          setLoading(false)
        }
      }
    }

    void loadConfig()
    return () => {
      ignore = true
    }
  }, [form, message])

  async function handleSave(values: DstConfig) {
    const nextValues = normalizeDstConfig(values)
    const invalidFields = validateDstConfig(nextValues)
    if (invalidFields.length > 0) {
      message.warning(`请填写必填项: ${invalidFields.join(', ')}`)
      return
    }

    try {
      setSaving(true)
      assertApiSuccess(await saveDstConfig(serializeDstConfig(nextValues)))
      message.success('系统设置已保存')
    } catch (error) {
      message.error(getErrorMessage(error, '保存系统设置失败'))
    } finally {
      setSaving(false)
    }
  }

  return (
    <ProCard title="系统设置" className="settings-page-card" bordered={false}>
      {loading ? (
        <div className="page-loading">
          <Spin />
        </div>
      ) : (
        <Form form={form} layout="vertical" className="settings-form" onFinish={handleSave}>
          <section>
            <h3>基础路径</h3>
            <Form.Item label="SteamCMD" name="steamcmd" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item label="游戏安装目录" name="force_install_dir" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item label="备份目录" name="backup" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item label="模组下载目录" name="mod_download_path" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
          </section>
          <section>
            <h3>Klei 配置</h3>
            <Form.Item label="集群目录" name="cluster" rules={[{ required: true }]}>
              <Input />
            </Form.Item>
            <Form.Item label="持久化根目录" name="persistent_storage_root">
              <Input />
            </Form.Item>
            <Form.Item label="配置目录名" name="conf_dir">
              <Input />
            </Form.Item>
            <Form.Item label="UGC 目录" name="ugc_directory">
              <Input />
            </Form.Item>
          </section>
          <section>
            <h3>运行参数</h3>
            <Form.Item label="服务端目录名" name="donot_starve_server_directory">
              <Input />
            </Form.Item>
            <Form.Item label="运行架构" name="bin">
              <Radio.Group
                options={[
                  { label: '32 位', value: '32' },
                  { label: '64 位', value: '64' },
                ]}
              />
            </Form.Item>
            <Form.Item label="测试分支" name="beta">
              <Radio.Group
                options={[
                  { label: '关闭', value: 0 },
                  { label: '开启', value: 1 },
                ]}
              />
            </Form.Item>
          </section>
          <div className="settings-save-bar">
            <Button type="primary" htmlType="submit" icon={<SaveOutlined />} loading={saving}>
              保存设置
            </Button>
          </div>
        </Form>
      )}
    </ProCard>
  )
}
