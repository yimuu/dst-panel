import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as modApi from '@/features/mods/mod.api'
import ModPage from '@/pages/ModPage.vue'
import type { ApiEnvelope, PageResult } from '@/shared/api/types'
import type { ModSummary } from '@/shared/types/domain'

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

vi.mock('@/features/mods/mod.api', () => ({
  listMods: vi.fn(),
  searchMods: vi.fn(),
  saveModInfo: vi.fn(),
}))

const listMods = vi.mocked(modApi.listMods)
const searchMods = vi.mocked(modApi.searchMods)
const saveModInfo = vi.mocked(modApi.saveModInfo)

let wrapper: VueWrapper | undefined

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function page(data: ModSummary[]): PageResult<ModSummary> {
  return {
    page: 1,
    size: 20,
    total: data.length,
    totalPage: 1,
    data,
  }
}

function mountModPage(): VueWrapper {
  wrapper = mount(ModPage, {
    attachTo: document.body,
    global: {
      plugins: [ElementPlus],
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

function searchInput(): DOMWrapper<HTMLInputElement> {
  const input = wrapper?.find<HTMLInputElement>('[data-test="mod-search-input"] input')

  if (!input?.exists()) {
    throw new Error('未找到模组搜索输入框')
  }

  return input
}

describe('mod page workflow', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    listMods.mockResolvedValue(success([]))
    searchMods.mockResolvedValue(success(page([])))
    saveModInfo.mockResolvedValue(success({}))
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('loads stored mods on mount', async () => {
    listMods.mockResolvedValue(
      success([
        {
          modid: '123',
          name: '几何布局',
          auth: '工坊作者',
        },
      ]),
    )

    mountModPage()
    await flushPromises()

    expect(listMods).toHaveBeenCalledTimes(1)
    expect(wrapper?.text()).toContain('几何布局')
    expect(wrapper?.text()).toContain('123')
  })

  it('searches with normalized workshop id', async () => {
    mountModPage()
    await flushPromises()

    await searchInput().setValue(' workshop-987654 ')
    await findButton('搜索').trigger('click')
    await flushPromises()

    expect(searchMods).toHaveBeenCalledWith({
      text: '987654',
      page: 1,
      size: 20,
      lang: 'zh',
    })
  })

  it('saves selected search results without duplicating stored mods', async () => {
    const storedMod: ModSummary = {
      modid: '1',
      name: '已保存模组',
    }
    const newMod: ModSummary = {
      id: '2',
      name: '新模组',
      desc: '新模组说明',
      img: '/mod.png',
      author: '作者',
      file_url: '/mod.zip',
      last_time: 0.0,
      time: 1710000000,
      mod_config: '{}',
      v: '1.0',
      update: true,
    }
    const duplicateNewMod: ModSummary = {
      ...newMod,
      name: '新模组重复',
    }
    const consumerOnlyMod: ModSummary = {
      consumer_id: 322330,
      name: '仅应用 ID',
    }

    listMods
      .mockResolvedValueOnce(success([storedMod]))
      .mockResolvedValueOnce(success([storedMod, newMod]))
    searchMods.mockResolvedValue(
      success(page([storedMod, newMod, duplicateNewMod, consumerOnlyMod])),
    )

    mountModPage()
    await flushPromises()
    await searchInput().setValue('几何')
    await findButton('搜索').trigger('click')
    await flushPromises()

    expect(wrapper?.text()).toContain('新模组说明')
    expect(wrapper?.text()).toContain('作者')
    expect(wrapper?.find('[data-test="mod-result-toggle-322330"]').exists()).toBe(false)

    await wrapper?.find('[data-test="mod-result-toggle-1"]').trigger('click')
    await wrapper?.find('[data-test="mod-result-toggle-2"]').trigger('click')
    await findButton('保存已选').trigger('click')
    await flushPromises()

    expect(saveModInfo).toHaveBeenCalledTimes(1)
    expect(saveModInfo).toHaveBeenCalledWith(
      expect.objectContaining({
        modid: '2',
        name: '新模组',
        description: '新模组说明',
        img: '/mod.png',
        auth: '作者',
        file_url: '/mod.zip',
        last_time: 1710000000,
        mod_config: '{}',
        v: '1.0',
        update: true,
      }),
    )
    expect(saveModInfo.mock.calls[0]?.[0]).not.toHaveProperty('id')
    expect(saveModInfo.mock.calls[0]?.[0]).not.toHaveProperty('ID')
    expect(listMods).toHaveBeenCalledTimes(2)
  })
})
