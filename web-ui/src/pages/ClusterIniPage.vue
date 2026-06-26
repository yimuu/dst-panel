<template>
  <PageState title="集群设置" description="编辑当前房间的 cluster.ini 和集群令牌。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>cluster.ini</span>
          <el-button :icon="Refresh" :loading="loading" size="small" @click="loadClusterIni">
            重新加载
          </el-button>
        </div>
      </template>

      <el-form v-loading="loading" label-position="top" @submit.prevent>
        <section class="settings-section">
          <h2>基础信息</h2>
          <el-row :gutter="12">
            <el-col :xs="24" :lg="8">
              <el-form-item label="世界名称">
                <div class="field-control" data-test="cluster-name-input">
                  <el-input v-model="form.cluster.cluster_name" />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="游戏模式">
                <el-select v-model="form.cluster.game_mode" class="field-control">
                  <el-option label="生存" value="survival" />
                  <el-option label="无尽" value="endless" />
                  <el-option label="荒野" value="wilderness" />
                </el-select>
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="最大人数">
                <div class="field-control" data-test="max-players-input">
                  <el-input
                    :model-value="String(form.cluster.max_players)"
                    min="1"
                    type="number"
                    @update:model-value="updateClusterNumber('max_players', $event)"
                  />
                </div>
              </el-form-item>
            </el-col>

            <el-col :xs="24">
              <el-form-item label="世界描述">
                <el-input
                  v-model="form.cluster.cluster_description"
                  :autosize="{ minRows: 3, maxRows: 6 }"
                  type="textarea"
                />
              </el-form-item>
            </el-col>
          </el-row>
        </section>

        <section class="settings-section">
          <h2>规则</h2>
          <el-row :gutter="12">
            <el-col :xs="24" :lg="8">
              <el-form-item label="是否 PVP">
                <el-switch v-model="form.cluster.pvp" active-text="开启" inactive-text="关闭" />
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="无人暂停">
                <el-switch
                  v-model="form.cluster.pause_when_nobody"
                  active-text="开启"
                  inactive-text="关闭"
                />
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="投票">
                <el-switch
                  v-model="form.cluster.vote_enabled"
                  active-text="开启"
                  inactive-text="关闭"
                />
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="最大快照数">
                <el-input
                  :model-value="String(form.cluster.max_snapshots)"
                  min="1"
                  type="number"
                  @update:model-value="updateClusterNumber('max_snapshots', $event)"
                />
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="Tick 速率">
                <el-input
                  :model-value="String(form.cluster.tick_rate)"
                  min="1"
                  type="number"
                  @update:model-value="updateClusterNumber('tick_rate', $event)"
                />
              </el-form-item>
            </el-col>
          </el-row>
        </section>

        <section class="settings-section">
          <h2>网络与令牌</h2>
          <el-row :gutter="12">
            <el-col :xs="24" :lg="8">
              <el-form-item label="集群密码">
                <el-input v-model="form.cluster.cluster_password" show-password />
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="令牌">
                <el-input v-model="form.token" show-password />
              </el-form-item>
            </el-col>

            <el-col :xs="24" :lg="8">
              <el-form-item label="主节点端口">
                <el-input
                  :model-value="String(form.cluster.master_port)"
                  min="1"
                  type="number"
                  @update:model-value="updateClusterNumber('master_port', $event)"
                />
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

import { getClusterIni, saveClusterIni } from '@/features/room/room.api'
import { isApiSuccess } from '@/shared/api/http'
import type { ApiEnvelope } from '@/shared/api/types'
import PageState from '@/shared/components/PageState.vue'
import type { ClusterIni, ClusterIniEnvelope } from '@/shared/types/domain'

type ClusterNumberField = 'max_players' | 'max_snapshots' | 'tick_rate' | 'master_port'

const form = reactive<ClusterIniEnvelope>(createEmptyClusterIniEnvelope())
const loading = ref(false)
const saving = ref(false)

onMounted(() => {
  void loadClusterIni()
})

async function loadClusterIni(): Promise<void> {
  loading.value = true

  try {
    applyClusterIniEnvelope(readApiData(await getClusterIni(), '集群设置加载失败'))
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '集群设置加载失败'))
  } finally {
    loading.value = false
  }
}

async function handleSave(): Promise<void> {
  saving.value = true

  try {
    const payload = normalizeClusterIniEnvelope(form)
    assertApiSuccess(await saveClusterIni(payload))
    await loadClusterIni()
    ElMessage.success('集群设置已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '集群设置保存失败'))
  } finally {
    saving.value = false
  }
}

function updateClusterNumber(field: ClusterNumberField, value: string | number): void {
  form.cluster[field] = readNumber(value, form.cluster[field])
}

function applyClusterIniEnvelope(payload: ClusterIniEnvelope): void {
  const normalized = normalizeClusterIniEnvelope(payload)

  Object.assign(form.cluster, normalized.cluster)
  form.token = normalized.token
}

function createEmptyClusterIniEnvelope(): ClusterIniEnvelope {
  return {
    cluster: {
      game_mode: 'survival',
      max_players: 8,
      pvp: false,
      pause_when_nobody: true,
      vote_enabled: true,
      vote_kick_enabled: true,
      lan_only_cluster: false,
      cluster_intention: '',
      cluster_description: '',
      cluster_password: '',
      cluster_name: '',
      offline_cluster: false,
      cluster_language: 'zh',
      whitelist_slots: 0,
      tick_rate: 15,
      console_enabled: true,
      max_snapshots: 6,
      shard_enabled: true,
      bind_ip: '0.0.0.0',
      master_ip: '127.0.0.1',
      master_port: 10888,
      cluster_key: '',
      steam_group_id: '',
      steam_group_only: false,
      steam_group_admins: false,
    },
    token: '',
  }
}

function normalizeClusterIniEnvelope(payload: Partial<ClusterIniEnvelope>): ClusterIniEnvelope {
  const defaults = createEmptyClusterIniEnvelope()
  const cluster = payload.cluster ?? ({} as Partial<ClusterIni>)

  return {
    cluster: {
      game_mode: readString(cluster.game_mode, defaults.cluster.game_mode).trim(),
      max_players: readNumber(cluster.max_players, defaults.cluster.max_players),
      pvp: readBoolean(cluster.pvp, defaults.cluster.pvp),
      pause_when_nobody: readBoolean(cluster.pause_when_nobody, defaults.cluster.pause_when_nobody),
      vote_enabled: readBoolean(cluster.vote_enabled, defaults.cluster.vote_enabled),
      vote_kick_enabled: readBoolean(cluster.vote_kick_enabled, defaults.cluster.vote_kick_enabled),
      lan_only_cluster: readBoolean(cluster.lan_only_cluster, defaults.cluster.lan_only_cluster),
      cluster_intention: readString(
        cluster.cluster_intention,
        defaults.cluster.cluster_intention,
      ).trim(),
      cluster_description: readString(
        cluster.cluster_description,
        defaults.cluster.cluster_description,
      ).trim(),
      cluster_password: readString(cluster.cluster_password, defaults.cluster.cluster_password),
      cluster_name: readString(cluster.cluster_name, defaults.cluster.cluster_name).trim(),
      offline_cluster: readBoolean(cluster.offline_cluster, defaults.cluster.offline_cluster),
      cluster_language: readString(cluster.cluster_language, defaults.cluster.cluster_language),
      whitelist_slots: readNumber(cluster.whitelist_slots, defaults.cluster.whitelist_slots),
      tick_rate: readNumber(cluster.tick_rate, defaults.cluster.tick_rate),
      console_enabled: readBoolean(cluster.console_enabled, defaults.cluster.console_enabled),
      max_snapshots: readNumber(cluster.max_snapshots, defaults.cluster.max_snapshots),
      shard_enabled: readBoolean(cluster.shard_enabled, defaults.cluster.shard_enabled),
      bind_ip: readString(cluster.bind_ip, defaults.cluster.bind_ip),
      master_ip: readString(cluster.master_ip, defaults.cluster.master_ip),
      master_port: readNumber(cluster.master_port, defaults.cluster.master_port),
      cluster_key: readString(cluster.cluster_key, defaults.cluster.cluster_key),
      steam_group_id: readString(cluster.steam_group_id, defaults.cluster.steam_group_id),
      steam_group_only: readBoolean(cluster.steam_group_only, defaults.cluster.steam_group_only),
      steam_group_admins: readBoolean(
        cluster.steam_group_admins,
        defaults.cluster.steam_group_admins,
      ),
    },
    token: readString(payload.token, defaults.token).trim(),
  }
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

function readNumber(value: unknown, fallback: number): number {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value
  }

  if (typeof value === 'string' && value.trim()) {
    const parsed = Number(value)

    return Number.isFinite(parsed) ? parsed : fallback
  }

  return fallback
}

function readBoolean(value: unknown, fallback: boolean): boolean {
  return typeof value === 'boolean' ? value : fallback
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
