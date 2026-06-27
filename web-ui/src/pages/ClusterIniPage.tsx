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
  Typography,
} from 'antd'
import { QuestionCircleOutlined } from '@ant-design/icons'

import { saveClusterIni, type ClusterIniEnvelope } from '@/features/room/room.api'

const defaultRoomValues: ClusterIniEnvelope = {
  cluster: {
    game_mode: 'survival',
    max_players: 8,
    pvp: false,
    pause_when_nobody: true,
    vote_enabled: true,
    vote_kick_enabled: true,
    lan_only_cluster: false,
    cluster_intention: 'cooperative',
    cluster_description: '123',
    cluster_password: '',
    cluster_name: 'huhuhu-test',
    offline_cluster: false,
    cluster_language: 'zh',
    whitelist_slots: 0,
    tick_rate: 15,
    console_enabled: true,
    max_snapshots: 6,
    shard_enabled: true,
    bind_ip: '0.0.0.0',
    master_ip: '127.0.0.1',
    master_port: 10888,
    cluster_key: '',
    steam_group_id: '',
    steam_group_only: false,
    steam_group_admins: false,
  },
  token: '................................................',
}

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
          initialValues={defaultRoomValues}
          labelCol={{ flex: '150px' }}
          labelAlign="right"
          colon={false}
          onFinish={async (values) => {
            await saveClusterIni(values)
            message.success('保存成功')
          }}
        >
          <Form.Item
            label={withHelp('名称', '房间在大厅中展示的名称')}
            name={['cluster', 'cluster_name']}
            rules={[{ required: true, message: '请输入名称' }]}
          >
            <Input allowClear />
          </Form.Item>
          <Form.Item label="-" className="room-emoji-row">
            <Typography.Link>emoji</Typography.Link>
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
            <Button type="primary" htmlType="submit">
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
