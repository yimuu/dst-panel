<template>
  <PageState title="模组" description="搜索创意工坊模组，保存到服务器模组列表。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>搜索模组</span>
          <el-button :icon="Refresh" :loading="storedLoading" size="small" @click="loadStoredMods">
            刷新已保存
          </el-button>
        </div>
      </template>

      <el-form class="search-form" @submit.prevent="searchWorkshopMods">
        <div data-test="mod-search-input">
          <el-input
            v-model="keyword"
            clearable
            placeholder="输入创意工坊 ID 或关键词"
            @keyup.enter="searchWorkshopMods"
          />
        </div>
        <el-button :icon="SearchIcon" :loading="searchLoading" type="primary" @click="searchWorkshopMods">
          搜索
        </el-button>
      </el-form>
    </el-card>

    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>本地 UGC 工具</span>
          <span class="section-meta">Master</span>
        </div>
      </template>

      <div class="ugc-tools">
        <input
          ref="ugcUploadInput"
          class="visually-hidden"
          data-test="ugc-upload-input"
          multiple
          type="file"
          @change="handleUgcUpload"
        />
        <el-button :icon="Upload" :loading="ugcUploading" @click="openUgcUploadDialog">
          上传本地 UGC
        </el-button>
        <el-button :icon="Refresh" :loading="ugcAcfLoading" @click="handleReadUgcAcf">
          读取 UGC ACF
        </el-button>
        <el-button :icon="Delete" :loading="setupDeleting" type="danger" @click="handleDeleteSetupWorkshop">
          清理 setup/workshop
        </el-button>
        <div class="delete-ugc-row">
          <div data-test="delete-ugc-input">
            <el-input v-model="deleteUgcWorkshopId" placeholder="workshop-123" />
          </div>
          <el-button :icon="Delete" :loading="ugcDeleting" type="danger" @click="handleDeleteUgcMod">
            删除本地 UGC
          </el-button>
        </div>
      </div>

      <el-table
        v-if="ugcAcfMods.length > 0"
        class="ugc-acf-table"
        :data="ugcAcfMods"
        row-key="workshopId"
      >
        <el-table-column label="UGC 模组" min-width="220">
          <template #default="{ row }">
            {{ formatModName(row) }}
          </template>
        </el-table-column>
        <el-table-column label="创意工坊 ID" min-width="140">
          <template #default="{ row }">
            {{ getModId(row) || row.workshopId || '未知' }}
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>搜索结果</span>
          <el-space wrap>
            <el-tag type="info">已选择 {{ selectedModIds.length }} 项</el-tag>
            <el-button
              :disabled="selectedModIds.length === 0"
              :icon="Check"
              :loading="saving"
              size="small"
              type="primary"
              @click="saveSelectedMods"
            >
              保存已选
            </el-button>
          </el-space>
        </div>
      </template>

      <el-table
        v-loading="searchLoading"
        :data="searchResults"
        :empty-text="searchEmptyText"
        :row-key="getModId"
      >
        <el-table-column label="选择" width="88">
          <template #default="{ row }">
            <el-checkbox
              :data-test="`mod-result-toggle-${getModId(row)}`"
              :disabled="!getModId(row)"
              :model-value="isSelected(row)"
              @change="() => toggleSearchResult(row)"
            />
          </template>
        </el-table-column>
        <el-table-column label="模组" min-width="240">
          <template #default="{ row }">
            <div class="mod-name">{{ formatModName(row) }}</div>
            <div class="mod-description">{{ formatModDescription(row) }}</div>
          </template>
        </el-table-column>
        <el-table-column label="创意工坊 ID" min-width="140">
          <template #default="{ row }">
            {{ getModId(row) || '未知' }}
          </template>
        </el-table-column>
        <el-table-column label="作者" min-width="140">
          <template #default="{ row }">
            {{ formatModAuthor(row) }}
          </template>
        </el-table-column>
        <el-table-column label="版本" width="120">
          <template #default="{ row }">
            {{ row.v || '未标记' }}
          </template>
        </el-table-column>
        <el-table-column label="更新时间" min-width="160">
          <template #default="{ row }">
            {{ formatModTime(row) }}
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>已保存模组</span>
          <span class="section-meta">{{ storedMods.length }} 个</span>
        </div>
      </template>

      <el-table
        v-loading="storedLoading"
        :data="storedMods"
        :empty-text="storedEmptyText"
        :row-key="getModId"
      >
        <el-table-column label="模组" min-width="240">
          <template #default="{ row }">
            <div class="mod-name">{{ formatModName(row) }}</div>
            <div class="mod-description">{{ formatModDescription(row) }}</div>
          </template>
        </el-table-column>
        <el-table-column label="创意工坊 ID" min-width="140">
          <template #default="{ row }">
            {{ getModId(row) || '未知' }}
          </template>
        </el-table-column>
        <el-table-column label="作者" min-width="140">
          <template #default="{ row }">
            {{ formatModAuthor(row) }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="row.enabled === false ? 'info' : 'success'">
              {{ row.enabled === false ? '未启用' : '已保存' }}
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { Check, Delete, Refresh, Search as SearchIcon, Upload } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, onMounted, ref } from 'vue'

import { formatWorkshopId, toggleModId } from '@/features/mods/mod-selection'
import {
  deleteSetupWorkshop,
  deleteUgcMod,
  listMods,
  readUgcAcf,
  saveModInfo,
  searchMods,
  uploadUgcMod,
  type ModPayload,
} from '@/features/mods/mod.api'
import { isApiSuccess } from '@/shared/api/http'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import PageState from '@/shared/components/PageState.vue'
import type { ModSummary } from '@/shared/types/domain'

const keyword = ref('')
const storedMods = ref<ModSummary[]>([])
const searchResults = ref<ModSummary[]>([])
const selectedModIds = ref<string[]>([])
const ugcAcfMods = ref<ModSummary[]>([])
const storedLoading = ref(false)
const searchLoading = ref(false)
const saving = ref(false)
const ugcUploading = ref(false)
const ugcAcfLoading = ref(false)
const setupDeleting = ref(false)
const ugcDeleting = ref(false)
const ugcUploadInput = ref<HTMLInputElement>()
const deleteUgcWorkshopId = ref('')

const searchEmptyText = computed(() => (searchLoading.value ? '正在搜索模组' : '暂无搜索结果'))
const storedEmptyText = computed(() =>
  storedLoading.value ? '正在加载模组列表' : '暂无已保存模组',
)

onMounted(() => {
  void loadStoredMods()
})

async function loadStoredMods(): Promise<void> {
  storedLoading.value = true

  try {
    storedMods.value = readApiData(await listMods(), '模组列表加载失败')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '模组列表加载失败'))
  } finally {
    storedLoading.value = false
  }
}

async function searchWorkshopMods(): Promise<void> {
  searchLoading.value = true

  try {
    const result = readApiData(
      await searchMods({
        text: formatWorkshopId(keyword.value),
        page: 1,
        size: 20,
        lang: 'zh',
      }),
      '模组搜索失败',
    )

    searchResults.value = normalizePageData(result)
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '模组搜索失败'))
  } finally {
    searchLoading.value = false
  }
}

function toggleSearchResult(mod: ModSummary): void {
  const modId = getModId(mod)

  if (!modId) {
    return
  }

  selectedModIds.value = toggleModId(selectedModIds.value, modId)
}

function isSelected(mod: ModSummary): boolean {
  const modId = getModId(mod)
  return Boolean(modId && selectedModIds.value.includes(modId))
}

async function saveSelectedMods(): Promise<void> {
  const storedIds = new Set(storedMods.value.map(getModId).filter(Boolean))
  const saveIds = new Set<string>()
  const modsToSave = searchResults.value.filter((mod) => {
    const modId = getModId(mod)

    if (!modId || !selectedModIds.value.includes(modId) || storedIds.has(modId)) {
      return false
    }

    if (saveIds.has(modId)) {
      return false
    }

    saveIds.add(modId)
    return true
  })

  if (modsToSave.length === 0) {
    return
  }

  saving.value = true

  try {
    for (const mod of modsToSave) {
      readApiData(await saveModInfo(createModPayload(mod)), '模组保存失败')
    }

    await loadStoredMods()
    const savedIds = new Set(modsToSave.map(getModId))
    selectedModIds.value = selectedModIds.value.filter((id) => !savedIds.has(id))
    ElMessage.success('模组已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '模组保存失败'))
  } finally {
    saving.value = false
  }
}

function openUgcUploadDialog(): void {
  ugcUploadInput.value?.click()
}

async function handleUgcUpload(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const files = [...(input.files ?? [])]

  if (files.length === 0) {
    return
  }

  const formData = new FormData()
  for (const file of files) {
    formData.append('files', file)
    formData.append('filePaths', file.webkitRelativePath || file.name)
  }

  ugcUploading.value = true

  try {
    assertApiSuccess(await uploadUgcMod(formData))
    ElMessage.success('UGC 文件已上传')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, 'UGC 文件上传失败'))
  } finally {
    input.value = ''
    ugcUploading.value = false
  }
}

async function handleReadUgcAcf(): Promise<void> {
  ugcAcfLoading.value = true

  try {
    ugcAcfMods.value = readApiData(await readUgcAcf(), 'UGC ACF 读取失败')
    ElMessage.success('UGC ACF 已读取')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, 'UGC ACF 读取失败'))
  } finally {
    ugcAcfLoading.value = false
  }
}

async function handleDeleteSetupWorkshop(): Promise<void> {
  setupDeleting.value = true

  try {
    assertApiSuccess(await deleteSetupWorkshop())
    ElMessage.success('setup/workshop 已清理')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, 'setup/workshop 清理失败'))
  } finally {
    setupDeleting.value = false
  }
}

async function handleDeleteUgcMod(): Promise<void> {
  const workshopId = deleteUgcWorkshopId.value.trim()

  if (!workshopId) {
    ElMessage.error('请填写创意工坊 ID')
    return
  }

  ugcDeleting.value = true

  try {
    assertApiSuccess(await deleteUgcMod(workshopId))
    deleteUgcWorkshopId.value = ''
    ElMessage.success('本地 UGC 已删除')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '本地 UGC 删除失败'))
  } finally {
    ugcDeleting.value = false
  }
}

function getModId(mod: ModSummary): string {
  const candidates = [mod.modid, mod.id, mod.workshop_id, mod.workshopId, mod.publishedfileid]
  const value = candidates.find((candidate) => {
    if (candidate === undefined || candidate === null) {
      return false
    }

    return String(candidate).trim().length > 0
  })

  return value === undefined || value === null ? '' : formatWorkshopId(String(value))
}

function formatModName(mod: ModSummary): string {
  return mod.name || getModId(mod) || '未命名模组'
}

function formatModDescription(mod: ModSummary): string {
  return readText(mod.description) ?? readText(mod.desc) ?? '暂无简介'
}

function formatModAuthor(mod: ModSummary): string {
  return readText(mod.auth) ?? readText(mod.author) ?? '未知'
}

function formatModTime(mod: ModSummary): string {
  const lastTime = getModLastTime(mod)

  if (lastTime === undefined) {
    return '未知'
  }

  const timestamp = lastTime > 10_000_000_000 ? lastTime : lastTime * 1000
  return new Date(timestamp).toLocaleString('zh-CN', { hour12: false })
}

function createModPayload(mod: ModSummary): ModPayload {
  return {
    modid: getModId(mod),
    name: readText(mod.name) ?? '',
    description: formatModPayloadDescription(mod),
    img: readText(mod.img) ?? '',
    auth: formatModPayloadAuthor(mod),
    file_url: readText(mod.file_url) ?? '',
    last_time: getModLastTime(mod) ?? 0,
    mod_config: mod.mod_config ?? '',
    v: readText(mod.v) ?? '',
    update: mod.update ?? false,
    consumer_appid: mod.consumer_appid ?? 0,
    creator_appid: mod.creator_appid ?? 0,
  }
}

function formatModPayloadDescription(mod: ModSummary): string {
  return readText(mod.description) ?? readText(mod.desc) ?? ''
}

function formatModPayloadAuthor(mod: ModSummary): string {
  return readText(mod.auth) ?? readText(mod.author) ?? ''
}

function getModLastTime(mod: ModSummary): number | undefined {
  const searchTime = parseNumber(mod.time)

  if (searchTime !== undefined && searchTime !== 0) {
    return searchTime
  }

  return parseNumber(mod.last_time)
}

function readText(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

function parseNumber(value: unknown): number | undefined {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value
  }

  if (typeof value !== 'string' || value.trim().length === 0) {
    return undefined
  }

  const numericValue = Number(value)
  return Number.isFinite(numericValue) ? numericValue : undefined
}

function normalizePageData(
  result: PageResult<ModSummary> | ModSummary[] | undefined,
): ModSummary[] {
  if (Array.isArray(result)) {
    return result
  }

  return result?.data ?? result?.records ?? result?.list ?? []
}

function readApiData<T>(response: ApiEnvelope<T>, fallbackMessage: string): T {
  if (!isApiSuccess(response)) {
    throw new Error(response.msg || response.message || fallbackMessage)
  }

  return response.data
}

function assertApiSuccess(response: ApiEnvelope<unknown>): void {
  readApiData(response, '操作失败')
}

function getErrorMessage(error: unknown, fallbackMessage: string): string {
  if (error instanceof Error && error.message) {
    return error.message
  }

  return fallbackMessage
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.section-meta {
  color: #667085;
  font-size: 13px;
}

.search-form {
  display: grid;
  grid-template-columns: minmax(220px, 1fr) auto;
  gap: 12px;
}

.ugc-tools {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  align-items: center;
}

.delete-ugc-row {
  display: grid;
  grid-template-columns: minmax(180px, 1fr) auto;
  gap: 12px;
  align-items: center;
  min-width: min(100%, 420px);
}

.ugc-acf-table {
  margin-top: 16px;
}

.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
}

.mod-name {
  color: #111827;
  font-weight: 600;
  line-height: 1.4;
}

.mod-description {
  margin-top: 4px;
  color: #667085;
  font-size: 13px;
  line-height: 1.5;
}

:global(.dark) .section-meta,
:global(.dark) .mod-description {
  color: #9ca3af;
}

:global(.dark) .mod-name {
  color: #f8fafc;
}

@media (max-width: 640px) {
  .section-header,
  .search-form,
  .delete-ugc-row {
    align-items: stretch;
    grid-template-columns: 1fr;
  }

  .section-header {
    display: grid;
  }
}
</style>
