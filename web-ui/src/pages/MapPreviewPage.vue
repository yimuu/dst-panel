<template>
  <PageState title="地图预览" description="生成并查看世界地图、地形检查和 session 文件。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>地图生成</span>
          <el-button :icon="Refresh" :loading="loading" size="small" @click="loadLevelsAndMetadata">
            重新加载
          </el-button>
        </div>
      </template>

      <div class="control-row">
        <el-select v-model="selectedLevelName" class="level-select" @change="refreshMapMetadata">
          <el-option
            v-for="levelName in levelNames"
            :key="levelName"
            :label="levelName"
            :value="levelName"
          />
        </el-select>
        <el-button :icon="Picture" :loading="generating" type="primary" @click="handleGenerate">
          生成地图
        </el-button>
      </div>
    </el-card>

    <div class="map-layout">
      <el-card shadow="never">
        <template #header>
          <div class="section-header">
            <span>地图图片</span>
            <el-tag type="info">{{ selectedLevelName }}</el-tag>
          </div>
        </template>

        <div class="map-frame">
          <img :src="mapImageUrl" alt="地图预览" />
        </div>
      </el-card>

      <el-card shadow="never">
        <template #header>
          <span>地图状态</span>
        </template>

        <el-descriptions :column="1" border>
          <el-descriptions-item label="海象巢">
            {{ walrusText }}
          </el-descriptions-item>
        </el-descriptions>

        <el-collapse class="session-collapse">
          <el-collapse-item title="session 文件" name="session">
            <el-input
              :model-value="sessionFile"
              :autosize="{ minRows: 8, maxRows: 14 }"
              readonly
              spellcheck="false"
              type="textarea"
            />
          </el-collapse-item>
        </el-collapse>
      </el-card>
    </div>
  </PageState>
</template>

<script setup lang="ts">
import { Picture, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, onMounted, ref } from 'vue'

import { listLevels } from '@/features/levels/level.api'
import {
  checkWalrusHutPlains,
  generateMap,
  getMapImageUrl,
  getSessionFile,
} from '@/features/maps/map.api'
import { createMapCacheKey, normalizeMapLevelName } from '@/features/maps/map-state'
import { isApiSuccess } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import PageState from '@/shared/components/PageState.vue'
import type { LevelSummary } from '@/shared/types/domain'

const levelNames = ref<string[]>(['Master', 'Caves'])
const selectedLevelName = ref('Master')
const cacheKey = ref('')
const loading = ref(false)
const generating = ref(false)
const walrusHutPlains = ref<boolean | null>(null)
const sessionFile = ref('')

const mapImageUrl = computed(() => getMapImageUrl(selectedLevelName.value, cacheKey.value))
const walrusText = computed(() => {
  if (walrusHutPlains.value === null) {
    return '未检查'
  }

  return walrusHutPlains.value ? '存在海象巢' : '未发现海象巢'
})

onMounted(() => {
  void loadLevelsAndMetadata()
})

async function loadLevelsAndMetadata(): Promise<void> {
  loading.value = true

  try {
    const levels = readApiData(await listLevels(), '世界列表加载失败')
    const names = levels.map(formatLevelName).filter(Boolean)
    levelNames.value = names.length > 0 ? names : ['Master', 'Caves']
    selectedLevelName.value = normalizeMapLevelName(selectedLevelName.value || levelNames.value[0])
    if (!levelNames.value.includes(selectedLevelName.value)) {
      selectedLevelName.value = levelNames.value[0] ?? 'Master'
    }
    await refreshMapMetadata()
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '地图信息加载失败'))
  } finally {
    loading.value = false
  }
}

async function handleGenerate(): Promise<void> {
  generating.value = true

  try {
    const levelName = normalizeMapLevelName(selectedLevelName.value)
    assertApiSuccess(await generateMap(levelName))
    cacheKey.value = createMapCacheKey()
    await refreshMapMetadata()
    ElMessage.success('地图已生成')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '地图生成失败'))
  } finally {
    generating.value = false
  }
}

async function refreshMapMetadata(): Promise<void> {
  const levelName = normalizeMapLevelName(selectedLevelName.value)
  selectedLevelName.value = levelName

  const [walrusResponse, sessionResponse] = await Promise.all([
    checkWalrusHutPlains(levelName),
    getSessionFile(levelName),
  ])

  walrusHutPlains.value = readApiData(walrusResponse, '海象巢检查失败')
  sessionFile.value = readApiData(sessionResponse, 'session 文件加载失败')
}

function formatLevelName(level: LevelSummary): string {
  return normalizeMapLevelName(level.levelName || level.uuid || level.name)
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
  return error instanceof Error && error.message ? error.message : fallbackMessage
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.control-row {
  display: grid;
  grid-template-columns: minmax(220px, 320px) auto;
  gap: 12px;
  align-items: center;
}

.level-select {
  width: 100%;
}

.map-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.4fr) minmax(320px, 0.8fr);
  gap: 14px;
}

.map-frame {
  min-height: 360px;
  display: grid;
  place-items: center;
  overflow: auto;
  background: #f8fafc;
}

.map-frame img {
  max-width: 100%;
  height: auto;
  display: block;
}

.session-collapse {
  margin-top: 16px;
}

:global(.dark) .map-frame {
  background: #111827;
}

@media (max-width: 960px) {
  .control-row,
  .map-layout {
    grid-template-columns: 1fr;
  }
}
</style>
