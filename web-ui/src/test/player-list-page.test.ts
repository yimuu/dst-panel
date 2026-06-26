import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as roomApi from '@/features/room/room.api'
import type { PlayerListKind } from '@/features/room/player-lists'
import PlayerListPage from '@/pages/PlayerListPage.vue'
import type { ApiEnvelope } from '@/shared/api/types'

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
  getPlayerList: vi.fn(),
  savePlayerList: vi.fn(),
}))

const getPlayerList = vi.mocked(roomApi.getPlayerList)
const savePlayerList = vi.mocked(roomApi.savePlayerList)

let wrapper: VueWrapper | undefined

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function mountPlayerListPage(kind: PlayerListKind, title: string): VueWrapper {
  wrapper = mount(PlayerListPage, {
    attachTo: document.body,
    props: {
      kind,
      title,
      description: `${title}维护`,
    },
    global: {
      plugins: [ElementPlus],
    },
  })

  return wrapper
}

function textarea(): DOMWrapper<HTMLTextAreaElement> {
  const input = wrapper?.find<HTMLTextAreaElement>('[data-test="player-list-textarea"] textarea')

  if (!input?.exists()) {
    throw new Error('未找到玩家列表文本框')
  }

  return input
}

function addInput(): DOMWrapper<HTMLInputElement> {
  const input = wrapper?.find<HTMLInputElement>('[data-test="new-player-input"] input')

  if (!input?.exists()) {
    throw new Error('未找到新增玩家输入框')
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

describe('player list page', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    getPlayerList.mockResolvedValue(success([]))
    savePlayerList.mockResolvedValue(success(null))
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it.each([
    ['adminlist', '管理员列表', 'KU_admin'],
    ['whitelist', '白名单', 'KU_friend'],
    ['blacklist', '黑名单', 'KU_blocked'],
  ] as const)('loads and saves %s values', async (kind, title, value) => {
    mountPlayerListPage(kind, title)
    await flushPromises()

    expect(getPlayerList).toHaveBeenCalledWith(kind)
    expect(wrapper?.text()).toContain(title)

    await textarea().setValue(`${value}\n${value}\n\n`)
    await findButton('保存列表').trigger('click')
    await flushPromises()

    expect(savePlayerList).toHaveBeenCalledWith(kind, [value])
  })

  it('adds trimmed player ids and removes rows before saving', async () => {
    getPlayerList.mockResolvedValue(success(['KU_existing']))

    mountPlayerListPage('adminlist', '管理员列表')
    await flushPromises()

    await addInput().setValue('  KU_admin  ')
    await findButton('添加').trigger('click')
    await flushPromises()
    await findButton('删除').trigger('click')
    await flushPromises()
    await findButton('保存列表').trigger('click')
    await flushPromises()

    expect(savePlayerList).toHaveBeenCalledWith('adminlist', ['KU_admin'])
  })
})
