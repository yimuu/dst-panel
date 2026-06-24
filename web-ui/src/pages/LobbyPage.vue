<template>
  <PageState title="大厅" description="管理大厅展示信息、公开状态和玩家入口。">
    <el-card shadow="never">
      <template #header>大厅信息</template>

      <el-descriptions :column="2" border>
        <el-descriptions-item label="公开状态">待接入</el-descriptions-item>
        <el-descriptions-item label="房间名称">待接入</el-descriptions-item>
        <el-descriptions-item label="房间描述">待接入</el-descriptions-item>
        <el-descriptions-item label="连接信息">待接入</el-descriptions-item>
      </el-descriptions>

      <div class="actions">
        <el-button disabled type="primary">发布大厅信息</el-button>
        <el-button disabled>刷新状态</el-button>
      </div>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<style scoped>
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 14px;
}
</style>
