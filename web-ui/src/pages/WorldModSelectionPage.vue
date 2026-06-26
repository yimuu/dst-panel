<template>
  <PageState title="选择模组" description="为当前世界生成并保存 modoverrides.lua。">
    <div class="mod-layout">
      <el-card shadow="never">
        <template #header>
          <div class="section-header">
            <span>已安装模组</span>
            <el-button :icon="Refresh" :loading="loading" size="small" @click="loadModSelection">
              重新加载
            </el-button>
          </div>
        </template>

        <div class="search-row" data-test="mod-filter-input">
          <el-input v-model="keyword" clearable placeholder="筛选模组名称或创意工坊 ID" />
        </div>

        <el-table
          v-loading="loading"
          :data="filteredMods"
          :empty-text="modEmptyText"
          :row-key="getModId"
        >
          <el-table-column label="选择" width="84">
            <template #default="{ row }">
              <el-checkbox
                :data-test="`mod-toggle-${getModId(row)}`"
                :disabled="!getModId(row)"
                :model-value="isSelected(row)"
                @change="() => toggleMod(row)"
              />
            </template>
          </el-table-column>
          <el-table-column label="模组" min-width="220">
            <template #default="{ row }">
              <div class="mod-name">{{ formatModName(row) }}</div>
              <div class="mod-description">{{ formatModDescription(row) }}</div>
            </template>
          </el-table-column>
          <el-table-column label="创意工坊 ID" min-width="140">
            <template #default="{ row }">
              {{ formatWorkshopKey(getModId(row)) || '未知' }}
            </template>
          </el-table-column>
        </el-table>
      </el-card>

      <el-card shadow="never">
        <template #header>
          <div class="section-header">
            <span>已选择</span>
            <el-tag type="info">{{ selectedModIds.length }} 个</el-tag>
          </div>
        </template>

        <div class="selected-list">
          <el-tag v-for="modId in selectedModIds" :key="modId" closable @close="removeMod(modId)">
            {{ formatWorkshopKey(modId) }}
          </el-tag>
          <span v-if="selectedModIds.length === 0" class="empty-text">暂无选择</span>
        </div>

        <el-form-item class="preview-field" label="modoverrides.lua 预览">
          <el-input
            :model-value="modDataPreview"
            :autosize="{ minRows: 8, maxRows: 14 }"
            readonly
            spellcheck="false"
            type="textarea"
          />
        </el-form-item>

        <el-button :icon="Check" :loading="saving" type="primary" @click="handleSave">
          保存选择
        </el-button>
      </el-card>
    </div>
  </PageState>
</template>

<script setup lang="ts">
import { Check, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, onMounted, ref } from 'vue'

import { listMods } from '@/features/mods/mod.api'
import { formatWorkshopId, toggleModId } from '@/features/mods/mod-selection'
import { getGameConfig, saveGameConfig } from '@/features/settings/settings.api'
import { assertApiSuccess, getErrorMessage, readApiData } from '@/shared/api/envelope'
import PageState from '@/shared/components/PageState.vue'
import type { GameConfig, ModSummary } from '@/shared/types/domain'

const keyword = ref('')
const installedMods = ref<ModSummary[]>([])
const selectedModIds = ref<string[]>([])
const gameConfig = ref<GameConfig | null>(null)
const loading = ref(false)
const saving = ref(false)

const filteredMods = computed(() => {
  const text = keyword.value.trim().toLowerCase()

  if (!text) {
    return installedMods.value
  }

  return installedMods.value.filter((mod) => {
    const haystack = [formatModName(mod), formatModDescription(mod), getModId(mod)]
      .join(' ')
      .toLowerCase()

    return haystack.includes(text)
  })
})
const modDataPreview = computed(() => renderModOverrides(selectedModIds.value))
const modEmptyText = computed(() => (loading.value ? '正在加载模组列表' : '暂无已安装模组'))

onMounted(() => {
  void loadModSelection()
})

async function loadModSelection(): Promise<void> {
  loading.value = true

  try {
    const [modsResponse, configResponse] = await Promise.all([listMods(), getGameConfig()])
    installedMods.value = readApiData(modsResponse, '模组列表加载失败')
    gameConfig.value = readApiData(configResponse, '游戏配置加载失败')
    selectedModIds.value = parseSelectedModIds(gameConfig.value.modData)
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '模组选择加载失败'))
  } finally {
    loading.value = false
  }
}

function toggleMod(mod: ModSummary): void {
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

function removeMod(modId: string): void {
  selectedModIds.value = selectedModIds.value.filter((candidate) => candidate !== modId)
}

async function handleSave(): Promise<void> {
  if (!gameConfig.value) {
    ElMessage.error('游戏配置尚未加载')
    return
  }

  saving.value = true

  try {
    assertApiSuccess(
      await saveGameConfig({
        ...gameConfig.value,
        modData: modDataPreview.value,
      }),
    )
    ElMessage.success('模组选择已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '模组选择保存失败'))
  } finally {
    saving.value = false
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

function formatWorkshopKey(modId: string): string {
  return modId ? `workshop-${formatWorkshopId(modId)}` : ''
}

function formatModName(mod: ModSummary): string {
  return mod.name || getModId(mod) || '未命名模组'
}

function formatModDescription(mod: ModSummary): string {
  return readText(mod.description) ?? readText(mod.desc) ?? '暂无简介'
}

function parseSelectedModIds(modData: string): string[] {
  const ids = new Set<string>()
  const matcher = /\["workshop-([^"]+)"\]/g
  let match = matcher.exec(modData)

  while (match) {
    ids.add(formatWorkshopId(match[1] ?? ''))
    match = matcher.exec(modData)
  }

  return [...ids].filter(Boolean)
}

function renderModOverrides(modIds: string[]): string {
  const lines = modIds.map((modId) => `  ["${formatWorkshopKey(modId)}"] = { enabled = true },`)

  return ['return {', ...lines, '}'].join('\n')
}

function readText(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined
}
</script>

<style scoped>
.mod-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.5fr) minmax(320px, 0.8fr);
  gap: 14px;
}

.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.search-row {
  margin-bottom: 12px;
}

.mod-name {
  color: #111827;
  font-weight: 600;
  line-height: 1.4;
}

.mod-description,
.empty-text {
  color: #667085;
  font-size: 13px;
  line-height: 1.5;
}

.selected-list {
  min-height: 40px;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: flex-start;
  margin-bottom: 16px;
}

.preview-field {
  display: block;
}

:global(.dark) .mod-name {
  color: #f8fafc;
}

:global(.dark) .mod-description,
:global(.dark) .empty-text {
  color: #9ca3af;
}

@media (max-width: 960px) {
  .mod-layout {
    grid-template-columns: 1fr;
  }
}
</style>
