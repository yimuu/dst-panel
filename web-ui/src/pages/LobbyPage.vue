<template>
  <PageState title="大厅" description="按大厅 rowId 查询服务器详情，只读查看公开信息。">
    <el-card shadow="never">
      <template #header>查询条件</template>

      <el-form class="query-form" label-position="top" @submit.prevent>
        <el-form-item label="区域">
          <el-radio-group v-model="query.region" data-test="lobby-region-select">
            <el-radio-button
              v-for="region in regions"
              :key="region.value"
              :label="region.label"
              :value="region.value"
            />
          </el-radio-group>
        </el-form-item>

        <el-form-item label="rowId">
          <div class="field-control" data-test="lobby-row-id-input">
            <el-input v-model="query.rowId" placeholder="输入大厅 rowId" />
          </div>
        </el-form-item>

        <el-form-item class="query-actions">
          <el-button :icon="Search" :loading="loading" type="primary" @click="handleQuery">
            查询
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card class="result-card" shadow="never">
      <template #header>大厅详情</template>

      <el-empty v-if="!detail" description="输入 rowId 后查询大厅详情，当前页面仅提供只读查看。" />
      <template v-else>
        <el-descriptions :column="2" border>
          <el-descriptions-item label="房间名称">{{ textValue(detail.name) }}</el-descriptions-item>
          <el-descriptions-item label="房主">{{ textValue(detail.host) }}</el-descriptions-item>
          <el-descriptions-item label="模式">{{ textValue(detail.mode) }}</el-descriptions-item>
          <el-descriptions-item label="季节">{{ textValue(detail.season) }}</el-descriptions-item>
          <el-descriptions-item label="玩家数">{{ playerCountText }}</el-descriptions-item>
          <el-descriptions-item label="天数">{{ dayText }}</el-descriptions-item>
          <el-descriptions-item label="需要密码">
            {{ booleanText(detail.password) }}
          </el-descriptions-item>
          <el-descriptions-item label="启用模组">{{
            booleanText(detail.mods)
          }}</el-descriptions-item>
          <el-descriptions-item label="描述" :span="2">
            {{ textValue(detail.desc) }}
          </el-descriptions-item>
        </el-descriptions>

        <div class="players">
          <h2>在线玩家</h2>
          <el-empty v-if="players.length === 0" description="暂无玩家数据" />
          <el-table v-else :data="players" size="small">
            <el-table-column label="名称" min-width="160">
              <template #default="{ row }">
                {{ textValue(row.name) }}
              </template>
            </el-table-column>
            <el-table-column label="角色" min-width="140">
              <template #default="{ row }">
                {{ textValue(row.prefab) }}
              </template>
            </el-table-column>
          </el-table>
        </div>
      </template>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { Search } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, reactive, ref } from 'vue'

import {
  getLobbyServerDetail,
  type LobbyPlayer,
  type LobbyServerDetail,
} from '@/features/settings/settings.api'
import { getErrorMessage, readApiData } from '@/shared/api/envelope'
import PageState from '@/shared/components/PageState.vue'

const regions = [
  { label: '亚太', value: 'ap-southeast-1' },
  { label: '美国东部', value: 'us-east-1' },
  { label: '欧洲', value: 'eu-west-1' },
]

const query = reactive({
  region: 'ap-southeast-1',
  rowId: '',
})
const loading = ref(false)
const detail = ref<LobbyServerDetail | null>(null)

const players = computed<LobbyPlayer[]>(() =>
  Array.isArray(detail.value?.playerList) ? detail.value.playerList : [],
)
const playerCountText = computed(() => {
  const connected = readNumber(detail.value?.connected)
  const maxConnections = readNumber(detail.value?.maxconnections)

  if (connected === undefined && maxConnections === undefined) {
    return '未知'
  }

  return `${connected ?? 0}/${maxConnections ?? '?'}`
})
const dayText = computed(() => {
  const day = readNumber(detail.value?.dayData?.day)
  return day === undefined || day <= 0 ? '未知' : `第 ${day} 天`
})

async function handleQuery(): Promise<void> {
  const rowId = query.rowId.trim()

  if (!rowId) {
    ElMessage.error('请输入大厅 rowId')
    return
  }

  loading.value = true

  try {
    detail.value = readApiData(
      await getLobbyServerDetail({
        region: query.region,
        rowId,
      }),
      '大厅详情查询失败',
    )
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '大厅详情查询失败'))
  } finally {
    loading.value = false
  }
}

function textValue(value: unknown): string {
  return typeof value === 'string' && value.length > 0 ? value : '未知'
}

function booleanText(value: unknown): string {
  return value === true ? '是' : '否'
}

function readNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}
</script>

<style scoped>
.query-form {
  display: grid;
  grid-template-columns: minmax(180px, 220px) minmax(220px, 1fr) auto;
  gap: 12px;
  align-items: end;
}

.query-actions {
  margin-bottom: 18px;
}

.field-control {
  width: 100%;
}

.result-card {
  margin-top: 12px;
}

.players {
  margin-top: 16px;
}

.players h2 {
  margin: 0 0 10px;
  font-size: 16px;
}

@media (max-width: 760px) {
  .query-form {
    grid-template-columns: 1fr;
  }

  .query-actions {
    margin-bottom: 0;
  }
}
</style>
