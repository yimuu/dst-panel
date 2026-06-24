<template>
  <PageState title="面板" description="管理房间、集群和服务器运行状态。">
    <el-alert
      title="房间操作正在建设中"
      description="启动、停止、重启和配置写入将在后续功能页接入。"
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
          <el-button-group>
            <el-button disabled size="small">启动</el-button>
            <el-button disabled size="small">停止</el-button>
            <el-button disabled size="small">重启</el-button>
          </el-button-group>
        </el-table-column>
      </el-table>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue'

import PageState from '@/shared/components/PageState.vue'
import { useClusterStore } from '@/shared/stores/cluster'
import { useLevelStore } from '@/shared/stores/levels'
import type { LevelSummary } from '@/shared/types/domain'

const clusterStore = useClusterStore()
const levelStore = useLevelStore()

const selectedClusterLabel = computed(() => clusterStore.selectedCluster || '未选择集群')
const emptyText = computed(() => (levelStore.loading ? '正在加载世界列表' : '暂无世界数据'))

onMounted(() => {
  refreshLevels()
})

function refreshLevels(): void {
  void levelStore.refreshLevels(clusterStore.selectedCluster).catch(() => undefined)
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
