import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus, { ElMessage } from 'element-plus'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as roomApi from '@/features/room/room.api'
import ClusterIniPage from '@/pages/ClusterIniPage.vue'
import type { ApiEnvelope } from '@/shared/api/types'
import type { ClusterIniEnvelope } from '@/shared/types/domain'

vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')

  return {
    ...actual,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
    },
  }
})

vi.mock('@/features/room/room.api', () => ({
  getClusterIni: vi.fn(),
  saveClusterIni: vi.fn(),
}))

const getClusterIni = vi.mocked(roomApi.getClusterIni)
const saveClusterIni = vi.mocked(roomApi.saveClusterIni)

let wrapper: VueWrapper | undefined

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function createClusterIniPayload(): ClusterIniEnvelope {
  return {
    cluster: {
      game_mode: 'survival',
      max_players: 8,
      pvp: false,
      pause_when_nobody: true,
      vote_enabled: true,
      vote_kick_enabled: true,
      lan_only_cluster: false,
      cluster_intention: 'cooperative',
      cluster_description: '测试描述',
      cluster_password: '',
      cluster_name: '旧世界',
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
    token: 'server-token',
  }
}

function mountClusterIniPage(): VueWrapper {
  wrapper = mount(ClusterIniPage, {
    attachTo: document.body,
    global: {
      plugins: [ElementPlus],
    },
  })

  return wrapper
}

function findInput(testId: string): DOMWrapper<HTMLInputElement> {
  const input = wrapper?.find<HTMLInputElement>(`[data-test="${testId}"] input`)

  if (!input?.exists()) {
    throw new Error(`未找到输入框：${testId}`)
  }

  return input
}

function findButton(label: string): DOMWrapper<HTMLButtonElement> {
  const button = wrapper
    ?.findAll<HTMLButtonElement>('button')
    .find((candidate) => candidate.text().includes(label))

  if (!button) {
    throw new Error(`未找到按钮：${label}`)
  }

  return button
}

describe('cluster.ini settings page', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    getClusterIni.mockResolvedValue(success(createClusterIniPayload()))
    saveClusterIni.mockResolvedValue(success(createClusterIniPayload()))
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('loads cluster.ini and saves edited cluster settings', async () => {
    mountClusterIniPage()
    await flushPromises()

    expect(getClusterIni).toHaveBeenCalled()
    expect(wrapper?.text()).toContain('集群设置')
    expect(wrapper?.text()).toContain('世界名称')

    await findInput('cluster-name-input').setValue('测试世界')
    await findInput('max-players-input').setValue('12')
    await findButton('保存设置').trigger('click')
    await flushPromises()

    expect(saveClusterIni).toHaveBeenCalledWith({
      cluster: expect.objectContaining({
        cluster_name: '测试世界',
        max_players: 12,
        pvp: false,
      }),
      token: 'server-token',
    })
  })

  it('shows a Chinese error message when loading fails', async () => {
    getClusterIni.mockResolvedValue({ code: 500, data: createClusterIniPayload(), msg: '读取失败' })

    mountClusterIniPage()
    await flushPromises()

    expect(ElMessage.error).toHaveBeenCalledWith('读取失败')
  })
})
