import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as levelApi from '@/features/levels/level.api'
import * as mapApi from '@/features/maps/map.api'
import MapPreviewPage from '@/pages/MapPreviewPage.vue'
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
  }
})

vi.mock('@/features/levels/level.api', () => ({
  listLevels: vi.fn(),
}))

vi.mock('@/features/maps/map.api', async () => {
  const actual =
    await vi.importActual<typeof import('@/features/maps/map.api')>('@/features/maps/map.api')

  return {
    ...actual,
    generateMap: vi.fn(),
    checkWalrusHutPlains: vi.fn(),
    getSessionFile: vi.fn(),
  }
})

const listLevels = vi.mocked(levelApi.listLevels)
const generateMap = vi.mocked(mapApi.generateMap)
const checkWalrusHutPlains = vi.mocked(mapApi.checkWalrusHutPlains)
const getSessionFile = vi.mocked(mapApi.getSessionFile)

let wrapper: VueWrapper | undefined

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function mountMapPreviewPage(): VueWrapper {
  wrapper = mount(MapPreviewPage, {
    attachTo: document.body,
    global: {
      plugins: [ElementPlus],
      stubs: {
        transition: false,
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

describe('map preview page', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    listLevels.mockResolvedValue(
      success<LevelSummary[]>([
        {
          levelName: 'Master',
          uuid: 'Master',
        },
        {
          levelName: 'Caves',
          uuid: 'Caves',
        },
      ]),
    )
    generateMap.mockResolvedValue(success(null))
    checkWalrusHutPlains.mockResolvedValue(success(true))
    getSessionFile.mockResolvedValue(success('SESSION DATA'))
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('generates a map image and loads map metadata for the selected world', async () => {
    mountMapPreviewPage()
    await flushPromises()

    expect(listLevels).toHaveBeenCalled()

    await findButton('生成地图').trigger('click')
    await flushPromises()

    expect(generateMap).toHaveBeenCalledWith('Master')
    expect(wrapper?.find('img').attributes('src')).toContain('/api/dst/map/image?levelName=Master')
    expect(checkWalrusHutPlains).toHaveBeenCalledWith('Master')
    expect(getSessionFile).toHaveBeenCalledWith('Master')
  })
})
