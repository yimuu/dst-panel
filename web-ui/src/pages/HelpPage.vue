<template>
  <PageState title="帮助" description="查看常见操作说明、服务端检查项和问题排查入口。">
    <el-row :gutter="12">
      <el-col v-for="item in helpItems" :key="item.title" :xs="24" :md="8">
        <el-card class="help-card" shadow="never">
          <h2>{{ item.title }}</h2>
          <p>{{ item.description }}</p>
          <el-button disabled size="small">查看说明</el-button>
        </el-card>
      </el-col>
    </el-row>
  </PageState>
</template>

<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'

const helpItems = [
  { title: '快速开始', description: '创建集群、添加世界并启动服务器。' },
  { title: '配置检查', description: '检查路径、权限、SteamCMD 和存档目录。' },
  { title: '问题排查', description: '定位启动失败、模组下载失败和连接异常。' },
]
</script>

<style scoped>
.help-card :deep(.el-card__body) {
  min-height: 150px;
  display: grid;
  align-content: start;
  gap: 10px;
}

.help-card h2,
.help-card p {
  margin: 0;
}

.help-card h2 {
  font-size: 16px;
}

.help-card p {
  color: #667085;
  font-size: 14px;
  line-height: 1.6;
}
</style>
