<template>
  <PageState title="帮助" description="查看常见操作说明、服务端检查项和问题排查入口。">
    <el-row :gutter="12">
      <el-col v-for="item in helpItems" :key="item.title" :xs="24" :md="8">
        <el-card class="help-card" shadow="never">
          <h2>{{ item.title }}</h2>
          <p>{{ item.description }}</p>
          <a class="help-link" :href="item.href" rel="noreferrer" target="_blank">
            <el-button :icon="Link" size="small" type="primary">打开说明</el-button>
          </a>
        </el-card>
      </el-col>
    </el-row>
  </PageState>
</template>

<script setup lang="ts">
import { Link } from '@element-plus/icons-vue'

import PageState from '@/shared/components/PageState.vue'

const helpItems = [
  {
    title: '部署文档',
    description: '查看项目部署、运行和打包说明。',
    href: 'https://github.com/yimuu/dst-panel/blob/main/docs/install.md',
  },
  {
    title: '多世界教程',
    description: '查看本地多世界配置教程和示例。',
    href: '/misc/DontStarveMultiWorldTotorial.md',
  },
  {
    title: '常见问题',
    description: '查看本地 FAQ、Docker Compose 和排障材料。',
    href: '/misc/FQA.md',
  },
  {
    title: 'Docker Compose',
    description: '查看本地容器部署参考。',
    href: '/misc/Docker-compose.md',
  },
  {
    title: '世界参数',
    description: '打开本地世界设置 JSON 模板。',
    href: '/misc/dst_world_setting.json',
  },
  {
    title: '预设模板',
    description: '打开本地预设模板配置。',
    href: '/misc/preinstall.json',
  },
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

.help-link {
  width: fit-content;
  text-decoration: none;
}
</style>
