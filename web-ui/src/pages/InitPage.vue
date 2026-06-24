<template>
  <PageState title="初始化" description="初始化流程将用于创建首个管理员账号并检查服务端环境。">
    <el-alert
      title="初始化接口尚未接入"
      description="后续将对接 /api/init，在这里完成首次管理员创建与环境检查。"
      type="info"
      show-icon
      :closable="false"
    />
  </PageState>
</template>

<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>
