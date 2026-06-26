import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus, { ElMessageBox } from 'element-plus'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as backupApi from '@/features/backups/backup.api'
import BackupPage from '@/pages/BackupPage.vue'
import type { ApiEnvelope } from '@/shared/api/types'
import { useClusterStore } from '@/shared/stores/cluster'
import type { BackupFile } from '@/shared/types/domain'

vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')

  return {
    ...actual,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
    },
    ElMessageBox: {
      confirm: vi.fn(),
    },
  }
})

vi.mock('@/features/backups/backup.api', () => ({
  listBackups: vi.fn(),
  createBackup: vi.fn(),
  restoreBackup: vi.fn(),
  deleteBackups: vi.fn(),
}))

const listBackups = vi.mocked(backupApi.listBackups)
const createBackup = vi.mocked(backupApi.createBackup)
const restoreBackup = vi.mocked(backupApi.restoreBackup)
const deleteBackups = vi.mocked(backupApi.deleteBackups)
const confirmMessageBox = vi.mocked(ElMessageBox.confirm)

let wrapper: VueWrapper | undefined

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function mountBackupPage(backups: BackupFile[]): VueWrapper {
  const pinia = createPinia()
  setActivePinia(pinia)
  useClusterStore().setSelectedCluster('Cluster_1')
  listBackups.mockResolvedValue(success(backups))

  wrapper = mount(BackupPage, {
    global: {
      plugins: [pinia, ElementPlus],
    },
  })

  return wrapper
}

function findButton(label: string): DOMWrapper<HTMLButtonElement> {
  const button = wrapper
    ?.findAll<HTMLButtonElement>('button')
    .find((candidate) => candidate.text().includes(label))

  if (!button) {
    throw new Error(`未找到按钮：${label}`)
  }

  return button
}

describe('backup page workflow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    createBackup.mockResolvedValue(success(null))
    restoreBackup.mockResolvedValue(success(null))
    deleteBackups.mockResolvedValue(success(null))
    confirmMessageBox.mockResolvedValue({} as never)
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('loads backups on mount and renders file name and size', async () => {
    mountBackupPage([
      {
        fileName: 'cluster-backup.zip',
        fileSize: 1048576,
        createTime: '2026-06-26T10:00:00Z',
      },
    ])
    await flushPromises()

    expect(listBackups).toHaveBeenCalledWith('Cluster_1')
    expect(wrapper?.text()).toContain('cluster-backup.zip')
    expect(wrapper?.text()).toContain('1.0 MB')
  })

  it('renders display-only names without enabling backup actions', async () => {
    mountBackupPage([
      {
        name: 'display-only.zip',
        fileSize: 1024,
      },
    ])
    await flushPromises()

    const restoreButton = findButton('恢复')
    const deleteButton = findButton('删除')

    expect(wrapper?.text()).toContain('display-only.zip')
    expect(restoreButton.element.disabled).toBe(true)
    expect(deleteButton.element.disabled).toBe(true)

    await restoreButton.trigger('click')
    await deleteButton.trigger('click')
    await flushPromises()

    expect(confirmMessageBox).not.toHaveBeenCalled()
    expect(restoreBackup).not.toHaveBeenCalled()
    expect(deleteBackups).not.toHaveBeenCalled()
  })

  it('preserves zero byte file sizes before falling back to size', async () => {
    mountBackupPage([
      {
        fileName: 'empty.zip',
        fileSize: 0,
        size: 1048576,
      },
    ])
    await flushPromises()

    expect(wrapper?.text()).toContain('0 B')
    expect(wrapper?.text()).not.toContain('1.0 MB')
  })

  it('creates a backup and reloads the list', async () => {
    mountBackupPage([])
    await flushPromises()

    await findButton('创建备份').trigger('click')
    await flushPromises()

    expect(createBackup).toHaveBeenCalledWith(undefined, 'Cluster_1')
    expect(listBackups).toHaveBeenCalledTimes(2)
  })

  it('restores a backup after confirmation', async () => {
    mountBackupPage([
      {
        fileName: 'restore-me.zip',
        fileSize: 1024,
      },
    ])
    await flushPromises()

    await findButton('恢复').trigger('click')
    await flushPromises()

    expect(confirmMessageBox).toHaveBeenCalledWith(
      expect.stringContaining('restore-me.zip'),
      '恢复备份',
      expect.any(Object),
    )
    expect(restoreBackup).toHaveBeenCalledWith('restore-me.zip', 'Cluster_1')
    expect(listBackups).toHaveBeenCalledTimes(2)
  })

  it('deletes a backup after confirmation', async () => {
    mountBackupPage([
      {
        fileName: 'delete-me.zip',
        fileSize: 1024,
      },
    ])
    await flushPromises()

    await findButton('删除').trigger('click')
    await flushPromises()

    expect(confirmMessageBox).toHaveBeenCalledWith(
      expect.stringContaining('delete-me.zip'),
      '删除备份',
      expect.any(Object),
    )
    expect(deleteBackups).toHaveBeenCalledWith({ fileNames: ['delete-me.zip'] }, 'Cluster_1')
    expect(listBackups).toHaveBeenCalledTimes(2)
  })
})
