<template>
  <PageState :title="title" :description="description">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <span>{{ title }}</span>
          <el-button :icon="Refresh" :loading="loading" size="small" @click="loadPlayerList">
            重新加载
          </el-button>
        </div>
      </template>

      <el-form v-loading="loading" label-position="top" @submit.prevent>
        <el-form-item label="玩家 KU ID">
          <div class="field-control" data-test="player-list-textarea">
            <el-input
              v-model="textareaValue"
              :autosize="{ minRows: 8, maxRows: 14 }"
              spellcheck="false"
              type="textarea"
            />
          </div>
        </el-form-item>

        <div class="action-row">
          <div class="field-control" data-test="new-player-input">
            <el-input v-model="newPlayerId" placeholder="KU_xxxxx" @keyup.enter="addPlayer" />
          </div>
          <el-button :icon="Plus" @click="addPlayer">添加</el-button>
          <el-button :icon="Check" :loading="saving" type="primary" @click="handleSave">
            保存列表
          </el-button>
        </div>
      </el-form>

      <el-table :data="playerRows" :empty-text="emptyText" row-key="value">
        <el-table-column label="KU ID" min-width="220" prop="value" />
        <el-table-column fixed="right" label="操作" width="120">
          <template #default="{ row }">
            <el-button :icon="Delete" size="small" type="danger" @click="removePlayer(row.value)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { Check, Delete, Plus, Refresh } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, onMounted, ref, watch } from 'vue'

import { getPlayerList, savePlayerList } from '@/features/room/room.api'
import type { PlayerListKind } from '@/features/room/player-lists'
import { assertApiSuccess, getErrorMessage, readApiData } from '@/shared/api/envelope'
import PageState from '@/shared/components/PageState.vue'

interface PlayerListPageProps {
  kind: PlayerListKind
  title: string
  description: string
}

const props = defineProps<PlayerListPageProps>()

const loading = ref(false)
const saving = ref(false)
const textareaValue = ref('')
const newPlayerId = ref('')
const emptyText = computed(() => (loading.value ? '正在加载玩家列表' : '暂无玩家数据'))
const playerRows = computed(() => parsePlayerList(textareaValue.value).map((value) => ({ value })))

watch(
  () => props.kind,
  () => {
    void loadPlayerList()
  },
)

onMounted(() => {
  void loadPlayerList()
})

async function loadPlayerList(): Promise<void> {
  loading.value = true

  try {
    textareaValue.value = parsePlayerList(
      readApiData(await getPlayerList(props.kind), '玩家列表加载失败').join('\n'),
    ).join('\n')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '玩家列表加载失败'))
  } finally {
    loading.value = false
  }
}

function addPlayer(): void {
  const value = newPlayerId.value.trim()

  if (!value) {
    return
  }

  const values = parsePlayerList(textareaValue.value)
  if (!values.includes(value)) {
    textareaValue.value = [...values, value].join('\n')
  }
  newPlayerId.value = ''
}

function removePlayer(value: string): void {
  textareaValue.value = parsePlayerList(textareaValue.value)
    .filter((candidate) => candidate !== value)
    .join('\n')
}

async function handleSave(): Promise<void> {
  saving.value = true

  try {
    const values = parsePlayerList(textareaValue.value)
    assertApiSuccess(await savePlayerList(props.kind, values))
    textareaValue.value = values.join('\n')
    ElMessage.success('玩家列表已保存')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '玩家列表保存失败'))
  } finally {
    saving.value = false
  }
}

function parsePlayerList(value: string): string[] {
  const uniqueValues = new Set<string>()

  for (const line of value.split(/\r?\n/)) {
    const trimmed = line.trim()
    if (trimmed) {
      uniqueValues.add(trimmed)
    }
  }

  return [...uniqueValues]
}
</script>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.field-control {
  width: 100%;
}

.action-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  gap: 12px;
  align-items: center;
  margin-bottom: 16px;
}

@media (max-width: 720px) {
  .action-row {
    grid-template-columns: 1fr;
  }
}
</style>
