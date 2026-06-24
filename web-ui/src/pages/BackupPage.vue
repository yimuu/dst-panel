<template>
  <PageState title="备份" description="查看存档备份、创建备份并恢复指定时间点。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>备份文件</span>
          <el-button disabled type="primary">创建备份</el-button>
        </div>
      </template>

      <el-table :data="[]" empty-text="暂无备份数据">
        <el-table-column label="文件名" min-width="180" />
        <el-table-column label="大小" width="120" />
        <el-table-column label="创建时间" width="180" />
        <el-table-column label="操作" width="180">
          <el-button-group>
            <el-button disabled size="small">下载</el-button>
            <el-button disabled size="small">恢复</el-button>
          </el-button-group>
        </el-table-column>
      </el-table>
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
