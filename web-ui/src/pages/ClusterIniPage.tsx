import { ProCard } from '@ant-design/pro-components'
import {
  App as AntApp,
  Button,
  Form,
  Input,
  InputNumber,
  Radio,
  Space,
  Switch,
  Tooltip,
} from 'antd'
import { QuestionCircleOutlined } from '@ant-design/icons'
import { useEffect, useState } from 'react'

import { getClusterIni, saveClusterIni, type ClusterIniEnvelope } from '@/features/room/room.api'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

const gameModes = [
  { label: '无尽', value: 'endless' },
  { label: '生存', value: 'survival' },
  { label: '荒野', value: 'wilderness' },
  { label: '暗无天日', value: 'lightsout' },
  { label: '熔炉', value: 'lavaarena' },
  { label: '暴食', value: 'quagmire' },
  { label: '海钓模式', value: 'oceanfishing' },
  { label: '闯关模式', value: 'quagmire_portal' },
  { label: '自定义模式', value: 'custom' },
]

export default function ClusterIniPage() {
  const [form] = Form.useForm<ClusterIniEnvelope>()
  const { message } = AntApp.useApp()
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [loadedValues, setLoadedValues] = useState<ClusterIniEnvelope | null>(null)

  useEffect(() => {
    let ignore = false

    async function loadClusterIni() {
      try {
        setLoading(true)
        const values = assertApiSuccess(await getClusterIni())
        if (!ignore) {
          setLoadedValues(values)
          form.setFieldsValue(values)
        }
      } catch (error) {
        if (!ignore) {
          message.error(getErrorMessage(error, '加载房间设置失败'))
        }
      } finally {
        if (!ignore) {
          setLoading(false)
        }
      }
    }

    void loadClusterIni()

    return () => {
      ignore = true
    }
  }, [form, message])

  return (
    <div className="room-settings-page">
      <ProCard
        className="room-settings-card"
        title={
          <Space size={4}>
            基础设置
            <Tooltip title="保存后写入 cluster.ini 和 cluster_token.txt">
              <QuestionCircleOutlined />
            </Tooltip>
          </Space>
        }
        bordered={false}
      >
        <Form
          className="room-settings-form"
          form={form}
          disabled={loading}
          labelCol={{ flex: '150px' }}
          labelAlign="right"
          colon={false}
          onFinish={async (values) => {
            if (!loadedValues) {
              message.error('请先加载房间设置')
              return
            }
            const payload: ClusterIniEnvelope = {
              ...loadedValues,
              ...values,
              cluster: {
                ...loadedValues.cluster,
                ...values.cluster,
              },
            }
            try {
              setSaving(true)
              assertApiSuccess(await saveClusterIni(payload))
              setLoadedValues(payload)
              message.success('保存成功')
            } catch (error) {
              message.error(getErrorMessage(error, '保存失败'))
            } finally {
              setSaving(false)
            }
          }}
        >
          <Form.Item
            label={withHelp('名称', '房间在大厅中展示的名称')}
            name={['cluster', 'cluster_name']}
            rules={[{ required: true, message: '请输入名称' }]}
          >
            <Input allowClear />
          </Form.Item>
          <Form.Item label="表情符号" className="room-emoji-row">
            <span className="room-muted-value">默认</span>
          </Form.Item>
          <Form.Item label="描述" name={['cluster', 'cluster_description']}>
            <Input.TextArea rows={4} />
          </Form.Item>
          <Form.Item
            label={withHelp('游戏模式', '选择 DST 官方支持的房间模式')}
            name={['cluster', 'game_mode']}
            rules={[{ required: true, message: '请选择游戏模式' }]}
          >
            <Radio.Group options={gameModes} />
          </Form.Item>
          <Form.Item label={withHelp('最大玩家数')} name={['cluster', 'max_players']}>
            <InputNumber min={1} max={64} />
          </Form.Item>
          <Form.Item label="密码" name={['cluster', 'cluster_password']}>
            <Input.Password maxLength={20} placeholder="最大长度20" />
          </Form.Item>
          <Form.Item
            label={withHelp('令牌', '用于连接 Klei 服务的服务器令牌')}
            name="token"
            rules={[{ required: true, message: '请输入令牌' }]}
          >
            <Input.Password />
          </Form.Item>
          <Form.Item label={withHelp('PVP')} name={['cluster', 'pvp']} valuePropName="checked">
            <Switch checkedChildren="开启" unCheckedChildren="关闭" />
          </Form.Item>
          <Form.Item
            label={withHelp('投票启用')}
            name={['cluster', 'vote_enabled']}
            valuePropName="checked"
          >
            <Switch checkedChildren="开启" unCheckedChildren="关闭" />
          </Form.Item>
          <Form.Item
            label={withHelp('无人时暂停')}
            name={['cluster', 'pause_when_nobody']}
            valuePropName="checked"
          >
            <Switch checkedChildren="开启" unCheckedChildren="关闭" />
          </Form.Item>
          <Form.Item
            label={withHelp('控制台启用', '面板远程控制台依赖该开关')}
            name={['cluster', 'console_enabled']}
            valuePropName="checked"
          >
            <Switch checkedChildren="开启" unCheckedChildren="关闭" />
          </Form.Item>
          <Form.Item label={withHelp('白名单名额')} name={['cluster', 'whitelist_slots']}>
            <InputNumber min={0} max={64} />
          </Form.Item>
          <div className="room-save-bar">
            <Button type="primary" htmlType="submit" loading={saving} disabled={!loadedValues}>
              保存
            </Button>
          </div>
        </Form>
      </ProCard>
    </div>
  )
}

function withHelp(label: string, title = label) {
  return (
    <Space size={4}>
      <span>{label}</span>
      <Tooltip title={title}>
        <QuestionCircleOutlined />
      </Tooltip>
    </Space>
  )
}
