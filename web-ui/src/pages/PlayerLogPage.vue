<template>
  <PageState title="玩家日志" description="查看玩家进入、离开、聊天和管理操作记录。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>日志查询</span>
          <el-button disabled type="primary">导出日志</el-button>
        </div>
      </template>

      <el-form class="filters" inline>
        <el-form-item label="玩家">
          <el-input disabled placeholder="玩家名或 KU ID" />
        </el-form-item>
        <el-form-item label="时间">
          <el-date-picker
            disabled
            type="daterange"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
          />
        </el-form-item>
      </el-form>

      <el-table :data="[]" empty-text="暂无玩家日志">
        <el-table-column label="时间" width="180" />
        <el-table-column label="玩家" width="160" />
        <el-table-column label="事件" min-width="180" />
        <el-table-column label="详情" min-width="220" />
      </el-table>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<style scoped>
.section-header,
.filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}

.section-header {
  justify-content: space-between;
}
</style>
