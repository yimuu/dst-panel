<template>
  <PageState title="预设模板" description="应用 static/preinstall 目录中的世界模板。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>预设模板</span>
        </div>
      </template>

      <el-alert
        class="warning-alert"
        :closable="false"
        show-icon
        title="应用预设会停止服务器并创建备份，然后替换当前集群文件。"
        type="warning"
      />

      <el-form class="template-form" label-position="top" @submit.prevent>
        <el-form-item label="模板名称">
          <div class="field-control" data-test="preinstall-template-input">
            <el-input v-model="templateName" placeholder="default" />
          </div>
        </el-form-item>

        <el-button :icon="Upload" :loading="saving" type="primary" @click="handleApply">
          应用模板
        </el-button>
      </el-form>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { Upload } from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { ref } from 'vue'

import { applyPreinstallTemplate } from '@/features/game/game.api'
import { isApiSuccess } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import PageState from '@/shared/components/PageState.vue'

const templateName = ref('default')
const saving = ref(false)

async function handleApply(): Promise<void> {
  const name = templateName.value.trim() || 'default'

  try {
    await ElMessageBox.confirm(
      '应用预设会停止服务器、保存世界、创建备份并替换当前集群文件。确定继续？',
      '应用预设模板',
      {
        cancelButtonText: '取消',
        confirmButtonText: '应用模板',
        type: 'warning',
      },
    )
  } catch {
    return
  }

  saving.value = true

  try {
    assertApiSuccess(await applyPreinstallTemplate(name))
    ElMessage.success('预设模板已应用')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '预设模板应用失败'))
  } finally {
    saving.value = false
  }
}

function assertApiSuccess(response: ApiEnvelope<unknown>): void {
  if (!isApiSuccess(response)) {
    throw new Error(response.msg || response.message || '操作失败')
  }
}

function getErrorMessage(error: unknown, fallbackMessage: string): string {
  return error instanceof Error && error.message ? error.message : fallbackMessage
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.warning-alert {
  margin-bottom: 16px;
}

.template-form {
  max-width: 520px;
}

.field-control {
  width: 100%;
}
</style>
