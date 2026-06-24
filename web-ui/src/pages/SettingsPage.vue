<template>
  <PageState title="设置" description="管理面板偏好、服务端路径和全局参数。">
    <el-card shadow="never">
      <template #header>基础设置</template>

      <el-form label-position="top">
        <el-form-item label="SteamCMD 路径">
          <el-input disabled placeholder="等待服务端配置接口" />
        </el-form-item>
        <el-form-item label="存档目录">
          <el-input disabled placeholder="等待服务端配置接口" />
        </el-form-item>
        <el-form-item>
          <el-button disabled type="primary">保存设置</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import PageState from '@/shared/components/PageState.vue'
</script>
