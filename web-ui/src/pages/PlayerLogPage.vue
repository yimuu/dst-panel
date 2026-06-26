<template>
  <PageState title="玩家日志" description="查看世界运行日志，辅助排查玩家进入、离开和聊天记录。">
    <el-card shadow="never">
      <template #header>
        <div class="section-header">
          <div class="section-title">
            <span>日志流</span>
            <el-tag :type="isConnected ? 'success' : 'info'" size="small">
              {{ connectionLabel }}
            </el-tag>
          </div>

          <el-button
            :disabled="!canConnect"
            :type="isConnected ? 'default' : 'primary'"
            @click="toggleStream"
          >
            {{ isConnected ? '断开连接' : '连接' }}
          </el-button>
        </div>
      </template>

      <div class="stream-panel">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="当前集群">
            {{ selectedClusterLabel }}
          </el-descriptions-item>
          <el-descriptions-item label="连接状态">
            {{ connectionLabel }}
          </el-descriptions-item>
        </el-descriptions>

        <el-form class="filters" inline @submit.prevent>
          <el-form-item label="世界">
            <el-input
              v-model.trim="levelName"
              :disabled="isConnected"
              placeholder="Master"
              data-test="level-name-input"
            />
          </el-form-item>

          <el-form-item label="级别">
            <el-button-group>
              <el-button
                v-for="filter in severityFilters"
                :key="filter.value"
                :data-test="filter.testId"
                :type="levelFilter === filter.value ? 'primary' : 'default'"
                @click="levelFilter = filter.value"
              >
                {{ filter.label }}
              </el-button>
            </el-button-group>
          </el-form-item>

          <el-form-item label="滚动">
            <el-switch v-model="autoScroll" active-text="自动" inactive-text="关闭" />
          </el-form-item>
        </el-form>

        <el-alert
          v-if="streamError"
          :title="streamError"
          type="warning"
          show-icon
          :closable="false"
        />

        <div ref="logContainer" class="log-console" role="log" aria-live="polite">
          <div v-if="visibleLogs.length === 0" class="empty-state">暂无日志数据</div>

          <div
            v-for="row in visibleLogs"
            :key="row.id"
            class="log-row"
            :class="`log-row--${row.levelKey}`"
            data-test="log-row"
          >
            <time class="log-time">{{ row.timestamp }}</time>
            <el-tag :type="getLevelTagType(row.level)" size="small" effect="plain">
              {{ row.level }}
            </el-tag>
            <span class="log-content">{{ row.content }}</span>
          </div>
        </div>
      </div>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from 'vue'

import { buildGameLogStreamPath } from '@/features/game/game.api'
import PageState from '@/shared/components/PageState.vue'
import { useClusterStore } from '@/shared/stores/cluster'

const maxLogRows = 1000

type LogLevel = '信息' | '警告' | '错误'
type LogLevelKey = 'info' | 'warn' | 'error'
type LevelFilter = '全部' | LogLevel

interface LogRow {
  id: number
  timestamp: string
  level: LogLevel
  levelKey: LogLevelKey
  content: string
}

const severityFilters: Array<{ value: LevelFilter; label: string; testId: string }> = [
  { value: '全部', label: '全部', testId: 'severity-all' },
  { value: '信息', label: '信息', testId: 'severity-info' },
  { value: '警告', label: '警告', testId: 'severity-warning' },
  { value: '错误', label: '错误', testId: 'severity-error' },
]

const clusterStore = useClusterStore()
const levelName = ref('Master')
const levelFilter = ref<LevelFilter>('全部')
const autoScroll = ref(true)
const logRows = ref<LogRow[]>([])
const logContainer = ref<HTMLElement>()
const stream = ref<EventSource>()
const streamError = ref('')

let nextLogId = 0
let removeStreamListeners: (() => void) | undefined

const selectedClusterLabel = computed(() => clusterStore.selectedCluster || '未选择集群')
const normalizedLevelName = computed(() => levelName.value.trim())
const canConnect = computed(() => isConnected.value || normalizedLevelName.value.length > 0)
const isConnected = computed(() => stream.value !== undefined)
const connectionLabel = computed(() => (isConnected.value ? '已连接' : '未连接'))
const visibleLogs = computed(() => {
  if (levelFilter.value === '全部') {
    return logRows.value
  }

  return logRows.value.filter((row) => row.level === levelFilter.value)
})

onBeforeUnmount(() => {
  disconnectStream()
})

function toggleStream(): void {
  if (isConnected.value) {
    disconnectStream()
    return
  }

  connectStream()
}

function connectStream(): void {
  if (!normalizedLevelName.value) {
    return
  }

  disconnectStream()
  streamError.value = ''

  const source = new EventSource(buildGameLogStreamPath(normalizedLevelName.value))
  const logListener = ((event: MessageEvent<string>) => {
    streamError.value = ''
    appendLogRow(event.data)
  }) as EventListener
  const openListener = () => {
    streamError.value = ''
  }

  source.addEventListener('log', logListener)
  source.addEventListener('message', logListener)
  source.addEventListener('open', openListener)
  source.onerror = () => {
    streamError.value = '日志流连接异常，正在等待重试'
  }

  removeStreamListeners = () => {
    source.removeEventListener('log', logListener)
    source.removeEventListener('message', logListener)
    source.removeEventListener('open', openListener)
  }
  stream.value = source
}

function disconnectStream(): void {
  const currentStream = stream.value

  if (!currentStream) {
    return
  }

  removeStreamListeners?.()
  removeStreamListeners = undefined
  currentStream.close()
  stream.value = undefined
  streamError.value = ''
}

function appendLogRow(content: string): void {
  const { level, levelKey } = inferLogLevel(content)
  const nextRows = [
    ...logRows.value,
    {
      id: (nextLogId += 1),
      timestamp: formatTime(new Date()),
      level,
      levelKey,
      content,
    },
  ]

  logRows.value = nextRows.length > maxLogRows ? nextRows.slice(-maxLogRows) : nextRows

  if (autoScroll.value) {
    void nextTick(scrollToLatestLog)
  }
}

function inferLogLevel(content: string): { level: LogLevel; levelKey: LogLevelKey } {
  const normalizedContent = content.toLowerCase()

  if (
    normalizedContent.includes('error') ||
    normalizedContent.includes('failed') ||
    normalizedContent.includes('panic') ||
    content.includes('错误')
  ) {
    return { level: '错误', levelKey: 'error' }
  }

  if (normalizedContent.includes('warn') || content.includes('警告')) {
    return { level: '警告', levelKey: 'warn' }
  }

  return { level: '信息', levelKey: 'info' }
}

function getLevelTagType(level: LogLevel): 'success' | 'warning' | 'danger' {
  if (level === '错误') {
    return 'danger'
  }

  if (level === '警告') {
    return 'warning'
  }

  return 'success'
}

function scrollToLatestLog(): void {
  const container = logContainer.value

  if (!container) {
    return
  }

  container.scrollTop = container.scrollHeight
}

function formatTime(date: Date): string {
  return [date.getHours(), date.getMinutes(), date.getSeconds()]
    .map((part) => String(part).padStart(2, '0'))
    .join(':')
}
</script>

<style scoped>
.section-header,
.section-title,
.filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}

.section-header {
  justify-content: space-between;
}

.section-title {
  gap: 8px;
  font-weight: 600;
}

.stream-panel {
  display: grid;
  gap: 14px;
}

.filters {
  margin-bottom: -18px;
}

.log-console {
  min-height: 380px;
  max-height: 58vh;
  overflow-y: auto;
  border: 1px solid #d0d5dd;
  border-radius: 8px;
  background: #101828;
  color: #f8fafc;
  font-family:
    ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New',
    monospace;
  font-size: 12px;
  line-height: 1.6;
}

.empty-state {
  min-height: 380px;
  display: grid;
  place-items: center;
  color: #98a2b3;
}

.log-row {
  display: grid;
  grid-template-columns: 72px 54px minmax(0, 1fr);
  align-items: start;
  gap: 10px;
  padding: 6px 10px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.log-row--error {
  background: rgba(180, 35, 24, 0.16);
}

.log-row--warn {
  background: rgba(181, 71, 8, 0.14);
}

.log-time {
  color: #98a2b3;
  white-space: nowrap;
}

.log-content {
  min-width: 0;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

:global(.dark) .log-console {
  border-color: #344054;
  background: #0b1220;
}
</style>
