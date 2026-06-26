<template>
  <PageState title="备份" description="查看存档备份、创建备份并恢复指定时间点。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>备份文件</span>
          <el-space wrap>
            <input
              ref="backupUploadInput"
              class="visually-hidden"
              data-test="backup-upload-input"
              type="file"
              @change="handleUploadBackup"
            />
            <el-button :icon="Upload" :loading="uploading" size="small" @click="openUploadDialog">
              上传备份
            </el-button>
            <el-button
              :icon="Plus"
              :loading="creating"
              size="small"
              type="primary"
              @click="handleCreateBackup"
            >
              {{ getBackupActionLabel('create') }}
            </el-button>
          </el-space>
        </div>
      </template>

      <el-table
        v-loading="loading"
        :data="backups"
        :empty-text="emptyText"
        :row-key="getBackupDisplayName"
      >
        <el-table-column label="文件名" min-width="220">
          <template #default="{ row }">
            {{ getBackupDisplayName(row) }}
          </template>
        </el-table-column>
        <el-table-column label="大小" width="120">
          <template #default="{ row }">
            {{ formatBackupSize(getBackupSize(row)) }}
          </template>
        </el-table-column>
        <el-table-column label="创建时间" min-width="180">
          <template #default="{ row }">
            {{ formatBackupTime(row) }}
          </template>
        </el-table-column>
        <el-table-column fixed="right" label="操作" width="320">
          <template #default="{ row }">
            <el-button-group>
              <el-button
                :disabled="!getBackupActionFileName(row)"
                :loading="isRestoring(row)"
                size="small"
                @click="confirmRestoreBackup(row)"
              >
                {{ getBackupActionLabel('restore') }}
              </el-button>
              <el-button
                :disabled="!getBackupActionFileName(row)"
                :icon="Edit"
                :loading="isRenaming(row)"
                size="small"
                @click="openRenameDialog(row)"
              >
                重命名
              </el-button>
              <el-button
                :disabled="!getBackupActionFileName(row)"
                :icon="Download"
                :loading="isDownloading(row)"
                size="small"
                @click="handleDownloadBackup(row)"
              >
                下载
              </el-button>
              <el-button
                :disabled="!getBackupActionFileName(row)"
                :loading="isDeleting(row)"
                size="small"
                type="danger"
                @click="confirmDeleteBackup(row)"
              >
                {{ getBackupActionLabel('delete') }}
              </el-button>
            </el-button-group>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="renameDialogVisible" :teleported="false" title="重命名备份" width="420px">
      <el-form label-position="top" @submit.prevent>
        <el-form-item label="新文件名">
          <div data-test="backup-rename-input">
            <el-input v-model="renameForm.newName" />
          </div>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="renameDialogVisible = false">取消</el-button>
        <el-button :loading="renamingFile.length > 0" type="primary" @click="saveRename">
          保存名称
        </el-button>
      </template>
    </el-dialog>
  </PageState>
</template>

<script setup lang="ts">
import { Download, Edit, Plus, Upload } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, onMounted, reactive, ref } from 'vue'

import {
  createBackup,
  deleteBackups,
  downloadBackup,
  listBackups,
  renameBackup,
  restoreBackup,
  uploadBackup,
} from '@/features/backups/backup.api'
import { formatBackupSize, getBackupActionLabel } from '@/features/backups/backup-format'
import { assertApiSuccess, getErrorMessage, readApiData } from '@/shared/api/envelope'
import PageState from '@/shared/components/PageState.vue'
import type { BackupFile } from '@/shared/types/domain'
import { confirmAction } from '@/shared/ui/confirm'

const backups = ref<BackupFile[]>([])
const loading = ref(false)
const creating = ref(false)
const uploading = ref(false)
const restoringFile = ref('')
const renamingFile = ref('')
const downloadingFile = ref('')
const deletingFile = ref('')
const renameDialogVisible = ref(false)
const backupUploadInput = ref<HTMLInputElement>()
const renameForm = reactive({
  fileName: '',
  newName: '',
})

const emptyText = computed(() => (loading.value ? '正在加载备份列表' : '暂无备份数据'))

onMounted(() => {
  void loadBackups()
})

async function loadBackups(): Promise<void> {
  loading.value = true

  try {
    const response = await listBackups()
    const data = readApiData(response, '备份列表加载失败')
    backups.value = Array.isArray(data) ? data : []
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '备份列表加载失败'))
  } finally {
    loading.value = false
  }
}

async function handleCreateBackup(): Promise<void> {
  creating.value = true

  try {
    assertApiSuccess(await createBackup())
    await loadBackups()
    ElMessage.success('备份已创建')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '备份创建失败'))
  } finally {
    creating.value = false
  }
}

function openUploadDialog(): void {
  backupUploadInput.value?.click()
}

async function handleUploadBackup(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]

  if (!file) {
    return
  }

  uploading.value = true

  try {
    assertApiSuccess(await uploadBackup(file))
    await loadBackups()
    ElMessage.success('备份已上传')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '备份上传失败'))
  } finally {
    input.value = ''
    uploading.value = false
  }
}

function openRenameDialog(backup: BackupFile): void {
  const fileName = getBackupActionFileName(backup)

  if (!fileName) {
    ElMessage.error('缺少备份文件名，无法重命名')
    return
  }

  renameForm.fileName = fileName
  renameForm.newName = fileName
  renameDialogVisible.value = true
}

async function saveRename(): Promise<void> {
  const newName = renameForm.newName.trim()

  if (!renameForm.fileName || !newName) {
    ElMessage.error('请填写新文件名')
    return
  }

  renamingFile.value = renameForm.fileName

  try {
    assertApiSuccess(await renameBackup({ fileName: renameForm.fileName, newName }))
    renameDialogVisible.value = false
    await loadBackups()
    ElMessage.success('备份已重命名')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '备份重命名失败'))
  } finally {
    renamingFile.value = ''
  }
}

async function confirmRestoreBackup(backup: BackupFile): Promise<void> {
  const fileName = getBackupActionFileName(backup)

  if (!fileName) {
    ElMessage.error('缺少备份文件名，无法恢复')
    return
  }

  const confirmed = await confirmAction(`确定恢复备份「${fileName}」吗？`, '恢复备份', {
    confirmButtonText: getBackupActionLabel('restore'),
  })

  if (!confirmed) {
    return
  }

  restoringFile.value = fileName

  try {
    assertApiSuccess(await restoreBackup(fileName))
    await loadBackups()
    ElMessage.success('备份已恢复')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '备份恢复失败'))
  } finally {
    restoringFile.value = ''
  }
}

async function confirmDeleteBackup(backup: BackupFile): Promise<void> {
  const fileName = getBackupActionFileName(backup)

  if (!fileName) {
    ElMessage.error('缺少备份文件名，无法删除')
    return
  }

  const confirmed = await confirmAction(`确定删除备份「${fileName}」吗？`, '删除备份', {
    confirmButtonText: getBackupActionLabel('delete'),
  })

  if (!confirmed) {
    return
  }

  deletingFile.value = fileName

  try {
    assertApiSuccess(await deleteBackups({ fileNames: [fileName] }))
    await loadBackups()
    ElMessage.success('备份已删除')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '备份删除失败'))
  } finally {
    deletingFile.value = ''
  }
}

async function handleDownloadBackup(backup: BackupFile): Promise<void> {
  const fileName = getBackupActionFileName(backup)

  if (!fileName) {
    ElMessage.error('缺少备份文件名，无法下载')
    return
  }

  downloadingFile.value = fileName

  try {
    triggerBlobDownload(await downloadBackup(fileName), fileName)
    ElMessage.success('备份下载已开始')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '备份下载失败'))
  } finally {
    downloadingFile.value = ''
  }
}

function getBackupDisplayName(backup: BackupFile): string {
  return backup.fileName || backup.name || '未命名备份'
}

function getBackupActionFileName(backup: BackupFile): string {
  return backup.fileName || ''
}

function getBackupSize(backup: BackupFile): number {
  return backup.fileSize ?? backup.size ?? 0
}

function formatBackupTime(backup: BackupFile): string {
  const value = backup.createTime ?? backup.time

  if (value === undefined || value === null || value === '') {
    return '未知'
  }

  const timestamp = parseBackupTimestamp(value)

  if (timestamp === undefined) {
    return String(value)
  }

  return new Date(timestamp).toLocaleString('zh-CN', { hour12: false })
}

function parseBackupTimestamp(value: number | string): number | undefined {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value > 10_000_000_000 ? value : value * 1000
  }

  if (typeof value !== 'string' || value.trim().length === 0) {
    return undefined
  }

  const numericValue = Number(value)
  if (Number.isFinite(numericValue)) {
    return numericValue > 10_000_000_000 ? numericValue : numericValue * 1000
  }

  const parsedValue = Date.parse(value)
  return Number.isNaN(parsedValue) ? undefined : parsedValue
}

function isRestoring(backup: BackupFile): boolean {
  const fileName = getBackupActionFileName(backup)
  return Boolean(fileName) && restoringFile.value === fileName
}

function isRenaming(backup: BackupFile): boolean {
  const fileName = getBackupActionFileName(backup)
  return Boolean(fileName) && renamingFile.value === fileName
}

function isDownloading(backup: BackupFile): boolean {
  const fileName = getBackupActionFileName(backup)
  return Boolean(fileName) && downloadingFile.value === fileName
}

function isDeleting(backup: BackupFile): boolean {
  const fileName = getBackupActionFileName(backup)
  return Boolean(fileName) && deletingFile.value === fileName
}

function triggerBlobDownload(blob: Blob, fileName: string): void {
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')

  link.href = url
  link.download = fileName
  document.body.appendChild(link)
  link.click()
  link.remove()
  URL.revokeObjectURL(url)
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}
</style>
