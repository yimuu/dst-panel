<template>
  <PageState title="世界" description="查看世界分片、配置文件和地图相关功能入口。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>世界列表</span>
          <el-space wrap>
            <el-button :icon="Plus" size="small" type="primary" @click="openCreateDialog">
              新建世界
            </el-button>
            <el-button
              :icon="Refresh"
              :loading="levelStore.loading"
              size="small"
              @click="refreshLevels"
            >
              刷新
            </el-button>
          </el-space>
        </div>
      </template>

      <el-table :data="levelStore.levels" :empty-text="emptyText" row-key="uuid">
        <el-table-column label="世界名称" min-width="180">
          <template #default="{ row }">
            {{ formatLevelName(row) }}
          </template>
        </el-table-column>
        <el-table-column label="分片标识" min-width="140">
          <template #default="{ row }">
            {{ row.uuid || '未设置' }}
          </template>
        </el-table-column>
        <el-table-column label="类型" width="140">
          <template #default="{ row }">
            {{ row.is_master ? '地上主世界' : '洞穴或分片' }}
          </template>
        </el-table-column>
        <el-table-column label="配置" min-width="180">
          <template #default="{ row }">
            <el-space wrap>
              <el-tag v-if="row.server_ini" type="info">server.ini</el-tag>
              <el-tag v-if="row.leveldataoverride" type="info">世界配置</el-tag>
              <el-tag v-if="row.modoverrides" type="info">模组配置</el-tag>
              <span v-if="!row.server_ini && !row.leveldataoverride && !row.modoverrides"
                >待接入</span
              >
            </el-space>
          </template>
        </el-table-column>
        <el-table-column fixed="right" label="操作" width="260">
          <template #default="{ row }">
            <el-button-group>
              <el-button :icon="Edit" size="small" @click="openEditDialog(row)">编辑</el-button>
              <el-button :icon="CopyDocument" size="small" @click="openCopyDialog(row)">
                复制
              </el-button>
              <el-button
                :icon="Delete"
                :loading="isDeleting(row)"
                size="small"
                type="danger"
                @click="confirmDeleteWorld(row)"
              >
                删除
              </el-button>
            </el-button-group>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" destroy-on-close width="720px">
      <el-form class="world-form" label-position="top" @submit.prevent>
        <div class="form-grid">
          <el-form-item label="世界显示名" required>
            <el-input v-model="worldForm.levelName" placeholder="例如：森林" />
          </el-form-item>
          <el-form-item label="分片标识" required>
            <el-input
              v-model="worldForm.uuid"
              :disabled="dialogMode === 'edit'"
              placeholder="Master"
            />
            <div class="form-help">
              {{
                dialogMode === 'edit'
                  ? '编辑时不能修改分片标识；如需新分片请使用复制'
                  : '例如：Master、Caves。用于分片目录和保存接口。'
              }}
            </div>
          </el-form-item>
        </div>

        <el-form-item label="主世界">
          <el-switch v-model="worldForm.is_master" active-text="是" inactive-text="否" />
        </el-form-item>

        <el-form-item label="server.ini">
          <el-input
            v-model="worldForm.server_ini"
            :autosize="{ minRows: 4, maxRows: 8 }"
            spellcheck="false"
            type="textarea"
          />
        </el-form-item>

        <el-form-item label="leveldataoverride.lua">
          <el-input
            v-model="worldForm.leveldataoverride"
            :autosize="{ minRows: 5, maxRows: 10 }"
            spellcheck="false"
            type="textarea"
          />
        </el-form-item>

        <el-form-item label="modoverrides.lua">
          <el-input
            v-model="worldForm.modoverrides"
            :autosize="{ minRows: 5, maxRows: 10 }"
            spellcheck="false"
            type="textarea"
          />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button :loading="saving" type="primary" @click="saveWorldForm">保存</el-button>
      </template>
    </el-dialog>
  </PageState>
</template>

<script setup lang="ts">
import { CopyDocument, Delete, Edit, Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { computed, onMounted, reactive, ref } from 'vue'

import { createLevel, deleteLevel, saveLevels } from '@/features/levels/level.api'
import {
  createEmptyWorldForm,
  normalizeWorldForm,
  type WorldForm,
} from '@/features/worlds/world-form'
import type { ApiEnvelope } from '@/shared/api/types'
import { isApiSuccess } from '@/shared/api/http'
import PageState from '@/shared/components/PageState.vue'
import { useLevelStore } from '@/shared/stores/levels'
import type { LevelSummary } from '@/shared/types/domain'

const levelStore = useLevelStore()

const emptyText = computed(() => (levelStore.loading ? '正在加载世界列表' : '暂无世界数据'))
const dialogTitle = computed(() => (dialogMode.value === 'edit' ? '编辑世界' : '新建世界'))
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const editingUuid = ref('')
const saving = ref(false)
const deletingUuid = ref('')
const worldForm = reactive<WorldForm>(createEmptyWorldForm())

onMounted(() => {
  refreshLevels()
})

function refreshLevels(): void {
  void levelStore.refreshLevels().catch(() => undefined)
}

function formatLevelName(level: LevelSummary): string {
  return level.levelName || level.name || level.uuid || '未命名世界'
}

function openCreateDialog(): void {
  dialogMode.value = 'create'
  editingUuid.value = ''
  applyWorldForm(createEmptyWorldForm())
  dialogVisible.value = true
}

function openEditDialog(level: LevelSummary): void {
  dialogMode.value = 'edit'
  editingUuid.value = getLevelUuid(level)
  applyWorldForm(createWorldFormFromLevel(level))
  dialogVisible.value = true
}

function openCopyDialog(level: LevelSummary): void {
  const copiedForm = createWorldFormFromLevel(level)

  dialogMode.value = 'create'
  editingUuid.value = ''
  applyWorldForm({
    ...copiedForm,
    levelName: createCopyLevelName(copiedForm.levelName || copiedForm.uuid),
    uuid: createCopyUuid(copiedForm.uuid || copiedForm.levelName),
  })
  dialogVisible.value = true
}

async function saveWorldForm(): Promise<void> {
  let payload

  try {
    payload = normalizeWorldForm({
      ...worldForm,
      uuid: dialogMode.value === 'edit' ? editingUuid.value : worldForm.uuid,
    })
  } catch (error) {
    ElMessage.error(getErrorMessage(error, 'server.ini 格式无效'))
    return
  }

  if (!payload.levelName || !payload.uuid) {
    ElMessage.error('请填写世界显示名和分片标识')
    return
  }

  if (dialogMode.value === 'create' && hasDuplicateUuid(payload.uuid)) {
    ElMessage.error('分片标识已存在')
    return
  }

  saving.value = true

  try {
    if (dialogMode.value === 'edit') {
      const updatedLevels = levelStore.levels.map((level) =>
        getLevelUuid(level) === editingUuid.value ? { ...level, ...payload } : level,
      )

      assertApiSuccess(await saveLevels(updatedLevels))
    } else {
      assertApiSuccess(await createLevel(payload))
    }

    dialogVisible.value = false
    await levelStore.refreshLevels().catch(() => undefined)
    ElMessage.success('世界配置已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '世界配置保存失败'))
  } finally {
    saving.value = false
  }
}

async function confirmDeleteWorld(level: LevelSummary): Promise<void> {
  const targetUuid = getLevelUuid(level)

  if (!targetUuid) {
    ElMessage.error('缺少分片标识，无法删除')
    return
  }

  try {
    await ElMessageBox.confirm(`确定删除世界「${formatLevelName(level)}」吗？`, '删除世界', {
      cancelButtonText: '取消',
      confirmButtonText: '删除',
      type: 'warning',
    })
  } catch {
    return
  }

  deletingUuid.value = targetUuid

  try {
    assertApiSuccess(await deleteLevel(targetUuid))
    await levelStore.refreshLevels().catch(() => undefined)
    ElMessage.success('世界配置已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '世界配置删除失败'))
  } finally {
    deletingUuid.value = ''
  }
}

function applyWorldForm(nextForm: WorldForm): void {
  Object.assign(worldForm, nextForm)
}

function createWorldFormFromLevel(level: LevelSummary): WorldForm {
  return {
    levelName: level.levelName || level.name || level.uuid || '',
    uuid: level.uuid || level.levelName || level.name || '',
    is_master: Boolean(level.is_master),
    server_ini: stringifyConfig(level.server_ini),
    leveldataoverride: stringifyConfig(level.leveldataoverride),
    modoverrides: stringifyConfig(level.modoverrides),
  }
}

function stringifyConfig(value: unknown): string {
  if (typeof value === 'string') {
    return value
  }

  if (value == null) {
    return ''
  }

  if (typeof value === 'object') {
    return JSON.stringify(value, null, 2)
  }

  return String(value)
}

function getLevelUuid(level: LevelSummary): string {
  return level.uuid || ''
}

function hasDuplicateUuid(uuid: string): boolean {
  return levelStore.levels.some((level) => getLevelUuid(level) === uuid)
}

function createCopyLevelName(name: string): string {
  const baseName = name.trim() || '未命名世界'
  const existingNames = new Set(levelStore.levels.map((level) => formatLevelName(level)))
  let candidate = `${baseName} 副本`
  let count = 2

  while (existingNames.has(candidate)) {
    candidate = `${baseName} 副本 ${count}`
    count += 1
  }

  return candidate
}

function createCopyUuid(uuid: string): string {
  const baseUuid = uuid.trim() || 'World'
  const existingUuids = new Set(levelStore.levels.map((level) => getLevelUuid(level)))
  let candidate = `${baseUuid}_copy`
  let count = 2

  while (existingUuids.has(candidate)) {
    candidate = `${baseUuid}_copy_${count}`
    count += 1
  }

  return candidate
}

function isDeleting(level: LevelSummary): boolean {
  return deletingUuid.value === getLevelUuid(level)
}

function assertApiSuccess(response: ApiEnvelope<unknown>): void {
  if (!isApiSuccess(response)) {
    throw new Error(response.msg || response.message || '操作失败')
  }
}

function getErrorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.world-form {
  max-height: min(68vh, 760px);
  overflow-y: auto;
  padding-right: 4px;
}

.form-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}

.form-help {
  margin-top: 6px;
  color: var(--el-text-color-secondary);
  font-size: 12px;
  line-height: 1.4;
}

@media (max-width: 720px) {
  .form-grid {
    grid-template-columns: 1fr;
  }
}
</style>
