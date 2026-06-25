<template>
  <PageState title="面板" description="管理房间、集群和服务器运行状态。">
    <el-alert
      title="世界操作已接入"
      description="提交启动、停止或重启后会自动刷新世界状态。"
      type="info"
      show-icon
      :closable="false"
    />

    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>当前集群</span>
          <el-button :loading="levelStore.loading" size="small" @click="refreshLevels"
            >刷新世界</el-button
          >
        </div>
      </template>

      <el-descriptions :column="2" border>
        <el-descriptions-item label="集群">
          {{ selectedClusterLabel }}
        </el-descriptions-item>
        <el-descriptions-item label="世界数量">
          {{ levelStore.levels.length }}
        </el-descriptions-item>
      </el-descriptions>
    </el-card>

    <el-card shadow="never">
      <template #header>世界状态</template>
      <el-table :data="levelStore.levels" :empty-text="emptyText" row-key="uuid">
        <el-table-column label="世界" min-width="160">
          <template #default="{ row }">
            {{ formatLevelName(row) }}
          </template>
        </el-table-column>
        <el-table-column label="角色" width="120">
          <template #default="{ row }">
            {{ row.is_master ? '主世界' : '洞穴/分片' }}
          </template>
        </el-table-column>
        <el-table-column label="状态" width="120">
          <template #default="{ row }">
            <el-tag :type="row.status ? 'success' : 'info'">
              {{ row.status ? '运行中' : '未运行' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="220">
          <template #default="{ row }">
            <el-button-group>
              <el-button
                v-for="action in panelActions"
                :key="action"
                :disabled="isActionDisabled(row, action)"
                :loading="isActionLoading(row, action)"
                size="small"
                @click="runLevelAction(row, action)"
              >
                {{ getPanelActionLabel(action) }}
              </el-button>
            </el-button-group>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'

import { startLevel, stopLevel } from '@/features/game/game.api'
import {
  getPanelActionLabel,
  isLevelActionDisabled,
  type PanelAction,
} from '@/features/panel/panel-actions'
import { isApiSuccess } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import PageState from '@/shared/components/PageState.vue'
import { useClusterStore } from '@/shared/stores/cluster'
import { useLevelStore } from '@/shared/stores/levels'
import type { LevelSummary } from '@/shared/types/domain'

const clusterStore = useClusterStore()
const levelStore = useLevelStore()
const panelActions: PanelAction[] = ['start', 'stop', 'restart']
const loadingActions = ref<Record<string, PanelAction | undefined>>({})

const selectedClusterLabel = computed(() => clusterStore.selectedCluster || '未选择集群')
const emptyText = computed(() => (levelStore.loading ? '正在加载世界列表' : '暂无世界数据'))

onMounted(() => {
  refreshLevels()
})

function refreshLevels(): void {
  void levelStore.refreshLevels(clusterStore.selectedCluster).catch(() => undefined)
}

async function runLevelAction(level: LevelSummary, action: PanelAction): Promise<void> {
  const levelName = getActionLevelName(level)

  if (!levelName) {
    ElMessage.error('缺少世界名称，无法执行操作')
    return
  }

  const levelKey = getLevelKey(level)
  loadingActions.value = {
    ...loadingActions.value,
    [levelKey]: action,
  }

  try {
    await submitLevelAction(levelName, action)
    ElMessage.success('操作已提交')
  } catch {
    ElMessage.error('操作失败')
  } finally {
    try {
      await levelStore.refreshLevels(clusterStore.selectedCluster).catch(() => undefined)
    } finally {
      const nextLoadingActions = { ...loadingActions.value }
      delete nextLoadingActions[levelKey]
      loadingActions.value = nextLoadingActions
    }
  }
}

async function submitLevelAction(levelName: string, action: PanelAction): Promise<void> {
  const cluster = clusterStore.selectedCluster

  if (action === 'start') {
    assertApiSuccess(await startLevel(levelName, cluster))
    return
  }

  if (action === 'stop') {
    assertApiSuccess(await stopLevel(levelName, cluster))
    return
  }

  assertApiSuccess(await stopLevel(levelName, cluster))
  assertApiSuccess(await startLevel(levelName, cluster))
}

function assertApiSuccess(response: ApiEnvelope<unknown>): void {
  if (!isApiSuccess(response)) {
    throw new Error(response.msg || response.message || '操作失败')
  }
}

function isActionDisabled(level: LevelSummary, action: PanelAction): boolean {
  return Boolean(loadingActions.value[getLevelKey(level)]) || isLevelActionDisabled(level, action)
}

function isActionLoading(level: LevelSummary, action: PanelAction): boolean {
  return loadingActions.value[getLevelKey(level)] === action
}

function getActionLevelName(level: LevelSummary): string {
  return typeof level.levelName === 'string' ? level.levelName.trim() : ''
}

function getLevelKey(level: LevelSummary): string {
  return level.uuid || level.levelName || level.name || '未命名世界'
}

function formatLevelName(level: LevelSummary): string {
  return level.levelName || level.name || level.uuid || '未命名世界'
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
</style>
