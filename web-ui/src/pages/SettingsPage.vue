<template>
  <PageState title="设置" description="管理 DST 服务端路径、集群存储和运行参数。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>DST 配置</span>
          <el-button :icon="Refresh" :loading="loading" size="small" @click="loadSettings">
            重新加载
          </el-button>
        </div>
      </template>

      <el-alert
        class="settings-alert"
        :closable="false"
        show-icon
        title="此页保存的是后端 dst_config 文件；注册开关和 Steam API Key 暂无持久化接口。"
        type="info"
      />

      <el-form v-loading="loading" label-position="top" @submit.prevent>
        <section class="settings-section">
          <h2>服务端路径</h2>
          <el-row :gutter="12">
            <el-col :xs="24" :lg="8">
              <el-form-item label="SteamCMD 目录">
                <div class="field-control" data-test="steamcmd-input">
                  <el-input v-model="form.steamcmd" placeholder="/opt/steamcmd" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="游戏安装目录">
                <div class="field-control" data-test="force-install-dir-input">
                  <el-input v-model="form.force_install_dir" placeholder="/opt/dst-server" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="饥荒服务端目录">
                <div class="field-control" data-test="donot-starve-server-directory-input">
                  <el-input v-model="form.donot_starve_server_directory" placeholder="可留空" />
                </div>
              </el-form-item>
            </el-col>
          </el-row>
        </section>

        <section class="settings-section">
          <h2>集群与存储</h2>
          <el-row :gutter="12">
            <el-col :xs="24" :lg="8">
              <el-form-item label="集群名称">
                <div class="field-control" data-test="cluster-input">
                  <el-input v-model="form.cluster" placeholder="Cluster_1" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="备份目录">
                <div class="field-control" data-test="backup-input">
                  <el-input v-model="form.backup" placeholder="/opt/dst-backup" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="模组下载目录">
                <div class="field-control" data-test="mod-download-path-input">
                  <el-input v-model="form.mod_download_path" placeholder="/opt/dst-mods" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="持久化根目录">
                <div class="field-control" data-test="persistent-storage-root-input">
                  <el-input v-model="form.persistent_storage_root" placeholder="可留空" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="配置目录名">
                <div class="field-control" data-test="conf-dir-input">
                  <el-input v-model="form.conf_dir" placeholder="DoNotStarveTogether" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="UGC 目录">
                <div class="field-control" data-test="ugc-directory-input">
                  <el-input v-model="form.ugc_directory" placeholder="可留空" />
                </div>
              </el-form-item>
            </el-col>
          </el-row>
        </section>

        <section class="settings-section">
          <h2>运行参数</h2>
          <el-row :gutter="12">
            <el-col :xs="24" :lg="8">
              <el-form-item label="运行位数">
                <el-radio-group v-model="form.bin" data-test="bin-radio-group">
                  <el-radio-button :value="32">32 位</el-radio-button>
                  <el-radio-button :value="64">64 位</el-radio-button>
                </el-radio-group>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="测试分支">
                <el-radio-group v-model="form.beta" data-test="beta-radio-group">
                  <el-radio-button :value="0">关闭</el-radio-button>
                  <el-radio-button :value="1">开启</el-radio-button>
                </el-radio-group>
              </el-form-item>
            </el-col>
          </el-row>
        </section>

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

import { getDstConfig, saveDstConfig, type DstConfig } from '@/features/settings/settings.api'
import { assertApiSuccess, getErrorMessage, readApiData } from '@/shared/api/envelope'
import PageState from '@/shared/components/PageState.vue'

const form = reactive<DstConfig>(createEmptyDstConfig())
const loading = ref(false)
const saving = ref(false)

onMounted(() => {
  void loadSettings()
})

async function loadSettings(): Promise<void> {
  loading.value = true

  try {
    Object.assign(form, normalizeDstConfig(readApiData(await getDstConfig(), '设置加载失败')))
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '设置加载失败'))
  } finally {
    loading.value = false
  }
}

async function handleSave(): Promise<void> {
  saving.value = true

  try {
    const payload = normalizeDstConfig(form)
    assertApiSuccess(await saveDstConfig(payload))
    await loadSettings()
    ElMessage.success('设置已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '设置保存失败'))
  } finally {
    saving.value = false
  }
}

function createEmptyDstConfig(): DstConfig {
  return {
    steamcmd: '',
    force_install_dir: '',
    donot_starve_server_directory: '',
    cluster: '',
    backup: '',
    mod_download_path: '',
    bin: 32,
    beta: 0,
    ugc_directory: '',
    persistent_storage_root: '',
    conf_dir: '',
  }
}

function normalizeDstConfig(config: Partial<DstConfig>): DstConfig {
  const defaults = createEmptyDstConfig()

  return {
    steamcmd: readString(config.steamcmd, defaults.steamcmd).trim(),
    force_install_dir: readString(config.force_install_dir, defaults.force_install_dir).trim(),
    donot_starve_server_directory: readString(
      config.donot_starve_server_directory,
      defaults.donot_starve_server_directory,
    ).trim(),
    cluster: readString(config.cluster, defaults.cluster).trim(),
    backup: readString(config.backup, defaults.backup).trim(),
    mod_download_path: readString(config.mod_download_path, defaults.mod_download_path).trim(),
    bin: readNumber(config.bin, defaults.bin) === 64 ? 64 : 32,
    beta: readNumber(config.beta, defaults.beta) === 1 ? 1 : 0,
    ugc_directory: readString(config.ugc_directory, defaults.ugc_directory).trim(),
    persistent_storage_root: readString(
      config.persistent_storage_root,
      defaults.persistent_storage_root,
    ).trim(),
    conf_dir: readString(config.conf_dir, defaults.conf_dir).trim(),
  }
}

function readString(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback
}

function readNumber(value: unknown, fallback: number): number {
  return typeof value === 'number' && Number.isFinite(value) ? value : fallback
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.settings-alert {
  margin-bottom: 16px;
}

.settings-section {
  margin-bottom: 8px;
}

.settings-section h2 {
  margin: 0 0 12px;
  font-size: 16px;
  font-weight: 600;
}

.field-control {
  width: 100%;
}
</style>
