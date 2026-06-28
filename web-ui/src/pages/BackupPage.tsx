import { ProCard } from '@ant-design/pro-components'
import {
  App as AntApp,
  Button,
  Popconfirm,
  Space,
  Table,
  Tag,
  Upload,
  type UploadProps,
} from 'antd'
import {
  CloudDownloadOutlined,
  DeleteOutlined,
  PlusOutlined,
  ReloadOutlined,
  UploadOutlined,
} from '@ant-design/icons'
import { useEffect, useState } from 'react'

import {
  createBackup,
  deleteBackups,
  getBackupDownloadUrl,
  getBackups,
  restoreBackup,
  uploadBackup,
  type BackupEntry,
} from '@/features/backups/backup.api'
import { formatBackupSize, formatBackupTime } from '@/features/backups/backup-format'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'

export default function BackupPage() {
  const { message } = AntApp.useApp()
  const [loading, setLoading] = useState(true)
  const [actionLoading, setActionLoading] = useState<string>()
  const [backups, setBackups] = useState<BackupEntry[]>([])
  const [selectedFileNames, setSelectedFileNames] = useState<string[]>([])

  async function loadBackups() {
    try {
      setLoading(true)
      setBackups(assertApiSuccess(await getBackups()))
    } catch (error) {
      message.error(getErrorMessage(error, '加载备份失败'))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadBackups()
  }, [])

  async function handleCreateBackup() {
    try {
      setActionLoading('create')
      assertApiSuccess(await createBackup())
      message.success('备份任务已提交')
      await loadBackups()
    } catch (error) {
      message.error(getErrorMessage(error, '创建备份失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleDeleteSelected() {
    if (selectedFileNames.length === 0) {
      message.warning('请选择备份文件')
      return
    }

    try {
      setActionLoading('delete')
      assertApiSuccess(await deleteBackups(selectedFileNames))
      setSelectedFileNames([])
      await loadBackups()
    } catch (error) {
      message.error(getErrorMessage(error, '删除备份失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  async function handleRestore(fileName: string) {
    try {
      setActionLoading(`restore-${fileName}`)
      assertApiSuccess(await restoreBackup(fileName))
      message.success('恢复任务已提交')
    } catch (error) {
      message.error(getErrorMessage(error, '恢复备份失败'))
    } finally {
      setActionLoading(undefined)
    }
  }

  const uploadProps: UploadProps = {
    accept: '.zip,.tar',
    beforeUpload: (file) => {
      void (async () => {
        try {
          setActionLoading('upload')
          assertApiSuccess(await uploadBackup(file))
          await loadBackups()
        } catch (error) {
          message.error(getErrorMessage(error, '上传备份失败'))
        } finally {
          setActionLoading(undefined)
        }
      })()
      return Upload.LIST_IGNORE
    },
    showUploadList: false,
  }

  return (
    <ProCard
      title="存档备份"
      className="data-page-card"
      bordered={false}
      extra={
        <Space wrap>
          <Button
            type="primary"
            icon={<PlusOutlined />}
            loading={actionLoading === 'create'}
            onClick={() => void handleCreateBackup()}
          >
            创建备份
          </Button>
          <Upload {...uploadProps}>
            <Button icon={<UploadOutlined />} loading={actionLoading === 'upload'}>
              上传备份
            </Button>
          </Upload>
          <Button icon={<ReloadOutlined />} onClick={() => void loadBackups()}>
            刷新
          </Button>
          <Popconfirm
            title="确认删除选中的备份"
            description="删除后无法从面板恢复，请确认已经不再需要。"
            okText="确认"
            cancelText="取消"
            disabled={selectedFileNames.length === 0}
            onConfirm={() => void handleDeleteSelected()}
          >
            <Button
              danger
              icon={<DeleteOutlined />}
              loading={actionLoading === 'delete'}
              onClick={() => {
                if (selectedFileNames.length === 0) {
                  message.warning('请选择备份文件')
                }
              }}
            >
              删除选中
            </Button>
          </Popconfirm>
        </Space>
      }
    >
      <Table
        rowKey="fileName"
        loading={loading}
        dataSource={backups}
        rowSelection={{
          selectedRowKeys: selectedFileNames,
          onChange: (keys) => setSelectedFileNames(keys.map(String)),
        }}
        pagination={{ pageSize: 8 }}
        columns={[
          { title: '备份列表', dataIndex: 'fileName' },
          {
            title: '大小',
            dataIndex: 'fileSize',
            render: (value) => formatBackupSize(Number(value)),
          },
          {
            title: '时间',
            dataIndex: 'time',
            render: (value) => formatBackupTime(value),
          },
          {
            title: '状态',
            render: () => <Tag color="success">可用</Tag>,
          },
          {
            title: '操作',
            render: (_, row) => (
              <Space>
                <Popconfirm
                  title={`确认恢复 ${row.fileName}`}
                  description="恢复备份会覆盖当前存档，请先确认已创建必要备份。"
                  okText="确认"
                  cancelText="取消"
                  onConfirm={() => void handleRestore(row.fileName)}
                >
                  <Button size="small" loading={actionLoading === `restore-${row.fileName}`}>
                    恢复
                  </Button>
                </Popconfirm>
                <Button
                  size="small"
                  icon={<CloudDownloadOutlined />}
                  href={getBackupDownloadUrl(row.fileName)}
                >
                  下载
                </Button>
              </Space>
            ),
          },
        ]}
      />
    </ProCard>
  )
}
