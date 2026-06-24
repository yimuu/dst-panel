<template>
  <PageState title="模组" description="管理服务器模组、版本状态和配置内容。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>模组列表</span>
          <el-button disabled type="primary">添加模组</el-button>
        </div>
      </template>

      <el-empty description="模组列表和配置编辑将在后续任务中接入" />
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
</style>
