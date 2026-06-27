import { ProCard } from '@ant-design/pro-components'
import { Alert, Button, Form, Input, InputNumber, Space, Switch, Tabs } from 'antd'

export default function WorldModSelectionPage() {
  return (
    <ProCard className="selector-mod-card" bordered={false}>
      <Tabs
        items={[
          {
            key: 'selector',
            label: '多层选择器',
            children: (
              <>
                <Alert
                  type="info"
                  showIcon
                  message="目前只兼容 [WIP] 又是一个世界选择器 workshop-1754389029 这种格式的配置"
                  action={<Button type="link">详细</Button>}
                />
                <Space className="selector-toolbar" wrap>
                  <Input value="workshop-1754389029" readOnly />
                  <Button type="primary">刷新</Button>
                  <Button type="primary">设置默认多层选择器</Button>
                </Space>
                <Form className="selector-config-form" layout="inline">
                  {[0, 1].map((index) => (
                    <div className="selector-config-row" key={index}>
                      <Form.Item label="世界id" required>
                        <Input placeholder="世界id" />
                      </Form.Item>
                      <Form.Item label="世界名称" required>
                        <Input placeholder="世界名称，不允许换行" />
                      </Form.Item>
                      <Form.Item label="分类">
                        <Input placeholder="世界类别，用于筛选" />
                      </Form.Item>
                      <Form.Item label="提示信息">
                        <Input placeholder="鼠标悬停显示的提示信息" />
                      </Form.Item>
                      <Form.Item label="人数">
                        <InputNumber min={1} max={64} placeholder="人数" />
                      </Form.Item>
                      <Form.Item label="不分流">
                        <Switch checkedChildren="是" unCheckedChildren="否" />
                      </Form.Item>
                      <Form.Item label="洞穴">
                        <Switch checkedChildren="是" unCheckedChildren="否" />
                      </Form.Item>
                      <Form.Item label="不可见">
                        <Switch checkedChildren="是" unCheckedChildren="否" />
                      </Form.Item>
                    </div>
                  ))}
                  <Button className="selector-add-field">添加字段</Button>
                </Form>
                <Button type="primary">保存配置</Button>
              </>
            ),
          },
          {
            key: 'sync',
            label: '世界配置同步',
            children: (
              <div className="selector-sync-panel">
                <Button type="primary">同步世界配置</Button>
              </div>
            ),
          },
        ]}
      />
    </ProCard>
  )
}
