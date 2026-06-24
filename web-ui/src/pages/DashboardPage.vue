<template>
  <PageState title="仪表盘" description="查看服务运行状态、在线人数、备份任务和近期告警概览。">
    <el-row :gutter="12">
      <el-col v-for="card in summaryCards" :key="card.label" :xs="24" :sm="12" :lg="6">
        <el-card class="metric-card" shadow="never">
          <span>{{ card.label }}</span>
          <strong>{{ card.value }}</strong>
          <small>{{ card.hint }}</small>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="never">
      <template #header>近期动态</template>
      <el-empty description="详细监控数据将在后续任务中接入" />
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'

const summaryCards = [
  { label: '集群状态', value: '待接入', hint: '等待服务端状态接口' },
  { label: '在线玩家', value: '--', hint: '等待玩家统计接口' },
  { label: '世界数量', value: '--', hint: '等待世界列表接口' },
  { label: '备份任务', value: '--', hint: '等待备份接口' },
]
</script>

<style scoped>
.metric-card :deep(.el-card__body) {
  min-height: 118px;
  display: grid;
  align-content: center;
  gap: 8px;
}

.metric-card span {
  color: #667085;
  font-size: 13px;
}

.metric-card strong {
  color: #111827;
  font-size: 26px;
}

.metric-card small {
  color: #98a2b3;
}
</style>
