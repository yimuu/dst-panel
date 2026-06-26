import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus, { ElMessage, ElMessageBox } from 'element-plus'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as levelApi from '@/features/levels/level.api'
import WorldLevelsPage from '@/pages/WorldLevelsPage.vue'
import type { ApiEnvelope } from '@/shared/api/types'
import type { LevelSummary } from '@/shared/types/domain'

vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')

  return {
    ...actual,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
    },
    ElMessageBox: {
      confirm: vi.fn(),
    },
  }
})

vi.mock('@/features/levels/level.api', () => ({
  listLevels: vi.fn(),
  createLevel: vi.fn(),
  saveLevels: vi.fn(),
  deleteLevel: vi.fn(),
}))

const listLevels = vi.mocked(levelApi.listLevels)
const createLevel = vi.mocked(levelApi.createLevel)
const saveLevels = vi.mocked(levelApi.saveLevels)
const deleteLevel = vi.mocked(levelApi.deleteLevel)
const confirmMessageBox = vi.mocked(ElMessageBox.confirm)
const errorMessage = vi.mocked(ElMessage.error)

let wrapper: VueWrapper | undefined

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function mountWorldLevelsPage(levels: LevelSummary[]): VueWrapper {
  const pinia = createPinia()
  setActivePinia(pinia)
  listLevels.mockResolvedValue(success(levels))

  wrapper = mount(WorldLevelsPage, {
    attachTo: document.body,
    global: {
      plugins: [pinia, ElementPlus],
      stubs: {
        teleport: true,
      },
    },
  })

  return wrapper
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

function dialogTextInput(index: number): DOMWrapper<HTMLInputElement> {
  const input = wrapper?.findAll<HTMLInputElement>('.el-input__inner')[index]

  if (!input) {
    throw new Error(`未找到输入框：${index}`)
  }

  return input
}

function dialogTextarea(index: number): DOMWrapper<HTMLTextAreaElement> {
  const textarea = wrapper?.findAll<HTMLTextAreaElement>('.el-textarea__inner')[index]

  if (!textarea) {
    throw new Error(`未找到文本框：${index}`)
  }

  return textarea
}

async function openCreateDialog(): Promise<void> {
  await findButton('新建世界').trigger('click')
  await flushPromises()
}

async function openEditDialog(): Promise<void> {
  await findButton('编辑').trigger('click')
  await flushPromises()
}

async function saveDialog(): Promise<void> {
  await findButton('保存').trigger('click')
  await flushPromises()
}

describe('world levels page editing workflow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    createLevel.mockResolvedValue(success({}))
    saveLevels.mockResolvedValue(success(null))
    deleteLevel.mockResolvedValue(success(null))
    confirmMessageBox.mockResolvedValue({} as never)
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('creates a world with object-shaped server_ini', async () => {
    mountWorldLevelsPage([])
    await flushPromises()
    await openCreateDialog()

    await dialogTextarea(0).setValue('[NETWORK]\nserver_port = 11001')
    await saveDialog()

    expect(createLevel).toHaveBeenCalledWith(
      expect.objectContaining({
        levelName: 'Master',
        uuid: 'Master',
        server_ini: {
          server_port: 11001,
          is_master: true,
          name: 'Master',
          id: 10000,
          encode_user_path: true,
          authentication_port: 8766,
          master_server_port: 27016,
        },
      }),
    )
    expect(typeof createLevel.mock.calls[0]?.[0].server_ini).toBe('object')
  })

  it('keeps the original uuid when saving edits', async () => {
    mountWorldLevelsPage([
      {
        levelName: '森林',
        uuid: 'Master',
        is_master: true,
        server_ini: {
          server_port: 10999,
          is_master: true,
          name: 'Master',
          id: 10000,
          encode_user_path: true,
          authentication_port: 8766,
          master_server_port: 27016,
        },
      },
    ])
    await flushPromises()
    await openEditDialog()

    const levelNameInput = dialogTextInput(0)
    const uuidInput = dialogTextInput(1)

    expect(uuidInput.element.disabled).toBe(true)

    await levelNameInput.setValue('森林更新')
    await uuidInput.setValue('RenamedShard')
    await saveDialog()

    expect(saveLevels).toHaveBeenCalledWith([
      expect.objectContaining({
        levelName: '森林更新',
        uuid: 'Master',
        server_ini: expect.objectContaining({
          name: 'Master',
        }),
      }),
    ])
  })

  it('deletes by shard uuid instead of display name', async () => {
    mountWorldLevelsPage([
      {
        levelName: '森林',
        uuid: 'Master',
        is_master: true,
      },
    ])
    await flushPromises()

    await findButton('删除').trigger('click')
    await flushPromises()

    expect(deleteLevel).toHaveBeenCalledWith('Master')
  })

  it('rejects duplicate create uuid before calling the API', async () => {
    mountWorldLevelsPage([
      {
        levelName: '森林',
        uuid: 'Master',
        is_master: true,
      },
    ])
    await flushPromises()
    await openCreateDialog()
    await saveDialog()

    expect(errorMessage).toHaveBeenCalledWith('分片标识已存在')
    expect(createLevel).not.toHaveBeenCalled()
  })
})
