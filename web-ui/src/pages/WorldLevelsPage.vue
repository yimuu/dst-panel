<template>
  <PageState title="世界" description="查看世界分片、配置文件和地图相关功能入口。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>世界列表</span>
          <el-button
            :loading="levelStore.loading"
            size="small"
            type="primary"
            @click="refreshLevels"
          >
            刷新
          </el-button>
        </div>
      </template>

      <el-table :data="levelStore.levels" :empty-text="emptyText" row-key="uuid">
        <el-table-column label="世界名称" min-width="180">
          <template #default="{ row }">
            {{ formatLevelName(row) }}
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
        <el-table-column label="操作" width="220">
          <el-button-group>
            <el-button disabled size="small">编辑</el-button>
            <el-button disabled size="small">复制</el-button>
            <el-button disabled size="small">删除</el-button>
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
