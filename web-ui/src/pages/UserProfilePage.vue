<template>
  <PageState title="个人信息" description="查看当前登录账号信息和后续账号安全入口。">
    <el-card shadow="never">
      <template #header>账号资料</template>

      <el-descriptions :column="2" border>
        <el-descriptions-item label="用户名">{{ displayName }}</el-descriptions-item>
        <el-descriptions-item label="角色">{{ roleName }}</el-descriptions-item>
        <el-descriptions-item label="账号 ID">{{ accountId }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ createdAt }}</el-descriptions-item>
      </el-descriptions>

      <div class="actions">
        <el-button disabled type="primary">修改资料</el-button>
        <el-button disabled>修改密码</el-button>
      </div>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { computed } from 'vue'

import PageState from '@/shared/components/PageState.vue'
import { useAuthStore } from '@/shared/stores/auth'

const auth = useAuthStore()

const displayName = computed(
  () => auth.user?.displayName || auth.user?.username || auth.user?.name || '未登录',
)
const roleName = computed(() => auth.user?.role || '管理员')
const accountId = computed(() => auth.user?.id || auth.user?.ID || '待接入')
const createdAt = computed(() => auth.user?.createdAt || '待接入')
</script>

<style scoped>
.actions {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  margin-top: 14px;
}
</style>
