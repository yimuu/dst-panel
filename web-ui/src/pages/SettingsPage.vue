<template>
  <PageState title="设置" description="管理面板标识、注册开关和 Steam API Key。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>基础设置</span>
          <el-button :icon="Refresh" :loading="loading" size="small" @click="loadSettings">
            重新加载
          </el-button>
        </div>
      </template>

      <el-form v-loading="loading" label-position="top" @submit.prevent>
        <el-form-item label="面板标识">
          <div class="field-control" data-test="panel-name-input">
            <el-input
              v-model="form.panelName"
              maxlength="40"
              placeholder="例如：DST 管理面板"
              show-word-limit
            />
          </div>
          <p class="field-hint">用于区分当前管理面板实例。</p>
        </el-form-item>

        <el-form-item label="注册开关">
          <div class="switch-row">
            <el-switch
              v-model="form.enableRegister"
              active-text="允许注册"
              inactive-text="关闭注册"
            />
          </div>
          <p class="field-hint">控制是否开放新账号注册入口。</p>
        </el-form-item>

        <el-form-item label="Steam API Key">
          <div class="field-control" data-test="steam-api-key-input">
            <el-input
              v-model="form.steamApiKey"
              autocomplete="off"
              placeholder="用于 Steam 相关接口的 API Key"
              show-password
              type="password"
            />
          </div>
          <p class="field-hint">仅保存此字段，不会配置 SteamCMD 路径或存档目录。</p>
        </el-form-item>

        <el-form-item>
          <el-button :icon="Check" :loading="saving" type="primary" @click="handleSave">
            保存设置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { Check, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { onMounted, reactive, ref } from 'vue'

import { normalizePanelSettings, type PanelSettingsForm } from '@/features/settings/settings-form'
import {
  getDstConfig,
  saveDstConfig,
  type DstConfigResponse,
} from '@/features/settings/settings.api'
import { isApiSuccess } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import PageState from '@/shared/components/PageState.vue'

const form = reactive<PanelSettingsForm>({
  panelName: 'DST 管理面板',
  enableRegister: false,
  steamApiKey: '',
})
const loading = ref(false)
const saving = ref(false)

onMounted(() => {
  void loadSettings()
})

async function loadSettings(): Promise<void> {
  loading.value = true

  try {
    applyConfig(readApiData(await getDstConfig(), '设置加载失败'))
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '设置加载失败'))
  } finally {
    loading.value = false
  }
}

async function handleSave(): Promise<void> {
  saving.value = true

  try {
    const payload = normalizePanelSettings(form)
    assertApiSuccess(await saveDstConfig(payload))
    Object.assign(form, payload)
    ElMessage.success('设置已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '设置保存失败'))
  } finally {
    saving.value = false
  }
}

function applyConfig(config: DstConfigResponse): void {
  form.panelName = readString(config.panelName, form.panelName)
  form.enableRegister = readBoolean(config.enableRegister, form.enableRegister)
  form.steamApiKey = readString(config.steamApiKey, form.steamApiKey)
}

function readApiData<T>(response: ApiEnvelope<T>, fallbackMessage: string): T {
  if (!isApiSuccess(response)) {
    throw new Error(response.msg || response.message || fallbackMessage)
  }

  return response.data
}

function assertApiSuccess(response: ApiEnvelope<unknown>): void {
  readApiData(response, '操作失败')
}

function readString(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback
}

function readBoolean(value: unknown, fallback: boolean): boolean {
  if (typeof value === 'boolean') {
    return value
  }

  if (typeof value === 'number') {
    return value !== 0
  }

  if (typeof value === 'string') {
    return value === 'true' || value === '1'
  }

  return fallback
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

.field-hint {
  width: 100%;
  margin: 6px 0 0;
  color: #667085;
  font-size: 13px;
  line-height: 1.5;
}

.field-control {
  width: 100%;
}

.switch-row {
  min-height: 32px;
  display: flex;
  align-items: center;
}
</style>
