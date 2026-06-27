import { ProCard } from '@ant-design/pro-components'
import {
  Button,
  Col,
  Input,
  Progress,
  Row,
  Select,
  Space,
  Statistic,
  Table,
  Tabs,
  Tag,
  Typography,
} from 'antd'
import {
  CloudDownloadOutlined,
  CloudUploadOutlined,
  PlayCircleOutlined,
  PoweroffOutlined,
  SaveOutlined,
  SendOutlined,
  SyncOutlined,
} from '@ant-design/icons'

import { getLevelActionTarget, getPanelActionLabel } from '@/features/panel/panel-model'

const levelRows = [
  { key: 'forest', levelName: '森林', memory: '1505.10MB', statusLabel: '运行' },
  { key: 'caves', levelName: '洞穴', memory: '0MB', statusLabel: '停止' },
]

const playerRows = [
  { key: '1', name: 'KU_qE7e8rv1', role: '威尔逊', online: '在线' },
  { key: '2', name: 'KU_panel', role: '温蒂', online: '离线' },
]

const logLines = [
  '[20:40:38]: RemoteCommandInput: "for i, v in ipairs(TheNet:GetClientTable()) do"',
  '[20:40:39]: player: { [1772604734] [0] [000] [KU_qE7e8rv1] [[Host]] []}',
  '[20:40:40]: RemoteCommandInput: "for i, v in ipairs(TheNet:GetClientTable()) do"',
  '[20:41:25]: player: { [1772604781] [0] [000] [KU_qE7e8rv1] [[Host]] []}',
  '[20:42:08]: player: { [1772604824] [0] [000] [KU_qE7e8rv1] [[Host]] []}',
]

export default function PanelPage() {
  return (
    <div className="panel-page">
      <Tabs
        className="panel-tabs"
        defaultActiveKey="panel"
        items={[
          { key: 'panel', label: '面板' },
          { key: 'remote', label: '远程' },
          { key: 'tooManyItemsPlus', label: 'TooManyItemsPlus' },
          { key: 'custom', label: '自定义指令' },
          { key: 'customEdit', label: '自定义指令-编辑' },
        ]}
      />

      <ProCard className="panel-resource-card" bordered={false}>
        <Row gutter={[24, 16]} align="middle">
          <Col xs={24} lg={6}>
            <Statistic title="面板" value="11.55 M" />
            <Typography.Text type="secondary">linux /x86_64-ubuntu</Typography.Text>
          </Col>
          <Col xs={24} lg={6}>
            <Space>
              <Progress type="circle" percent={39.29} size={72} strokeColor="#73d13d" />
              <Statistic
                title="内存使用"
                value="2.95 GB"
                suffix={<span className="stat-sub">总内存 4.25 / 7.51 GB</span>}
              />
            </Space>
          </Col>
          <Col xs={24} lg={6}>
            <Space>
              <Progress type="circle" percent={7.95} size={72} />
              <Statistic title="CPU使用" value={7.95} suffix="%" />
            </Space>
          </Col>
          <Col xs={24} lg={6}>
            <Space>
              <Progress type="circle" percent={11.65} size={72} />
              <Statistic title="空闲磁盘" value="156.42 GB" />
            </Space>
          </Col>
        </Row>
      </ProCard>

      <div className="panel-content-grid">
        <div className="panel-left-column">
          <ProCard title="服务器信息" bordered={false}>
            <Space className="panel-action-row" wrap>
              <Button type="primary" icon={<SyncOutlined />}>
                更新游戏
              </Button>
              <Button type="primary" icon={<SaveOutlined />}>
                创建备份
              </Button>
              <Button type="primary" icon={<CloudUploadOutlined />}>
                上传存档
              </Button>
              <Button type="primary" icon={<CloudDownloadOutlined />}>
                地图预览
              </Button>
            </Space>
            <div className="server-info-grid">
              <InfoItem label="房间名称" value="huhuhu-test" />
              <InfoItem label="游戏模式" value="生存" />
              <InfoItem label="模组数量" value="2" />
              <InfoItem label="天数进度" value="1天 / 早 (0/0)" />
              <InfoItem label="玩家数量" value="0/8" />
              <InfoItem label="游戏版本" value="714014 / 714014" />
              <InfoItem label="IP连接" value="******" />
              <InfoItem label="房间密码" value="******" />
            </div>
          </ProCard>

          <ProCard
            title="世界列表"
            bordered={false}
            extra={
              <Space>
                <Button type="primary" icon={<PlayCircleOutlined />}>
                  {getPanelActionLabel('start')}
                </Button>
                <Button type="primary" icon={<PoweroffOutlined />}>
                  {getPanelActionLabel('stop')}
                </Button>
              </Space>
            }
          >
            <Table
              rowKey="key"
              pagination={false}
              dataSource={levelRows}
              columns={[
                {
                  title: '世界',
                  dataIndex: 'levelName',
                  render: (_, row) => (
                    <Space>
                      <span className="level-badge">{getLevelActionTarget(row)}</span>
                      <span>{row.levelName}</span>
                    </Space>
                  ),
                },
                { title: '内存', dataIndex: 'memory' },
                {
                  title: '状态',
                  dataIndex: 'statusLabel',
                  render: (status) => (
                    <Tag color={status === '运行' ? 'processing' : 'default'}>{status}</Tag>
                  ),
                },
              ]}
            />
          </ProCard>
        </div>

        <div className="panel-right-column">
          <ProCard
            title="服务器日志"
            bordered={false}
            extra={
              <Space>
                <Select value="森林" options={[{ value: '森林', label: '森林' }]} />
                <Button type="primary" icon={<CloudDownloadOutlined />}>
                  下载日志
                </Button>
              </Space>
            }
          >
            <div className="server-log">
              {logLines.map((line, index) => (
                <div key={line} className="server-log-line">
                  <span>{index + 89}</span>
                  <code>{line}</code>
                </div>
              ))}
            </div>
            <Input.Search
              className="command-input"
              enterButton={<SendOutlined />}
              placeholder="输入远程指令"
            />
            <Space className="rollback-row" wrap>
              <Button type="primary" icon={<SaveOutlined />}>
                保存存档
              </Button>
              <Button danger type="primary">
                重置世界
              </Button>
              {[1, 2, 3, 4, 5, 6].map((day) => (
                <Button key={day}>回档({day})天</Button>
              ))}
            </Space>
          </ProCard>

          <ProCard title="玩家列表" bordered={false}>
            <Space className="player-toolbar" wrap>
              <Select value="森林" options={[{ value: '森林', label: '森林' }]} />
              <Button type="primary">查询</Button>
              <Button type="primary">查询所有</Button>
              <Tag color="success">0</Tag>
            </Space>
            <Table
              rowKey="key"
              pagination={false}
              dataSource={playerRows}
              columns={[
                { title: '玩家', dataIndex: 'name' },
                { title: '角色', dataIndex: 'role' },
                { title: '状态', dataIndex: 'online' },
              ]}
            />
          </ProCard>
        </div>
      </div>
    </div>
  )
}

function InfoItem({ label, value }: { label: string; value: string }) {
  return (
    <div className="server-info-item">
      <Typography.Text strong>{label}</Typography.Text>
      <span>{value}</span>
    </div>
  )
}
