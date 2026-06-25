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
        <el-button
          :icon="SearchIcon"
          :loading="searchLoading"
          type="primary"
          @click="searchWorkshopMods"
        >
          搜索
        </el-button>
      </el-form>
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
            <div class="mod-description">{{ row.description || '暂无简介' }}</div>
          </template>
        </el-table-column>
        <el-table-column label="创意工坊 ID" min-width="140">
          <template #default="{ row }">
            {{ getModId(row) || '未知' }}
          </template>
        </el-table-column>
        <el-table-column label="作者" min-width="140">
          <template #default="{ row }">
            {{ row.auth || '未知' }}
          </template>
        </el-table-column>
        <el-table-column label="版本" width="120">
          <template #default="{ row }">
            {{ row.v || '未标记' }}
          </template>
        </el-table-column>
        <el-table-column label="更新时间" min-width="160">
          <template #default="{ row }">
            {{ formatLastTime(row.last_time) }}
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
            <div class="mod-description">{{ row.description || '暂无简介' }}</div>
          </template>
        </el-table-column>
        <el-table-column label="创意工坊 ID" min-width="140">
          <template #default="{ row }">
            {{ getModId(row) || '未知' }}
          </template>
        </el-table-column>
        <el-table-column label="作者" min-width="140">
          <template #default="{ row }">
            {{ row.auth || '未知' }}
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
import { Check, Refresh, Search as SearchIcon } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, onMounted, ref } from 'vue'

import { formatWorkshopId, toggleModId } from '@/features/mods/mod-selection'
import { listMods, saveModInfo, searchMods, type ModPayload } from '@/features/mods/mod.api'
import { isApiSuccess } from '@/shared/api/http'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import PageState from '@/shared/components/PageState.vue'
import type { ModSummary } from '@/shared/types/domain'

const keyword = ref('')
const storedMods = ref<ModSummary[]>([])
const searchResults = ref<ModSummary[]>([])
const selectedModIds = ref<string[]>([])
const storedLoading = ref(false)
const searchLoading = ref(false)
const saving = ref(false)

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
  const modsToSave = searchResults.value.filter((mod) => {
    const modId = getModId(mod)
    return modId && selectedModIds.value.includes(modId) && !storedIds.has(modId)
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

function getModId(mod: ModSummary): string {
  const candidates = [
    mod.modid,
    mod.id,
    mod.ID,
    mod.workshop_id,
    mod.workshopId,
    mod.publishedfileid,
    mod.consumer_id,
  ]
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

function formatLastTime(value: unknown): string {
  if (typeof value !== 'number' || Number.isNaN(value)) {
    return '未知'
  }

  const timestamp = value > 10_000_000_000 ? value : value * 1000
  return new Date(timestamp).toLocaleString('zh-CN', { hour12: false })
}

function createModPayload(mod: ModSummary): ModPayload {
  return {
    ...mod,
    modid: getModId(mod),
    name: mod.name,
    description: mod.description,
    img: mod.img,
    auth: mod.auth,
    file_url: mod.file_url,
    last_time: mod.last_time,
    mod_config: mod.mod_config,
    v: mod.v,
    update: mod.update,
  }
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
  .search-form {
    align-items: stretch;
    grid-template-columns: 1fr;
  }

  .section-header {
    display: grid;
  }
}
</style>
