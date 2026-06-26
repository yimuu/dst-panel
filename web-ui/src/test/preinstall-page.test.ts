import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus, { ElMessageBox } from 'element-plus'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as gameApi from '@/features/game/game.api'
import PreinstallPage from '@/pages/PreinstallPage.vue'

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

vi.mock('@/features/game/game.api', () => ({
  applyPreinstallTemplate: vi.fn(),
}))

const applyPreinstallTemplate = vi.mocked(gameApi.applyPreinstallTemplate)
const confirmMessageBox = vi.mocked(ElMessageBox.confirm)

let wrapper: VueWrapper | undefined

function mountPreinstallPage(): VueWrapper {
  wrapper = mount(PreinstallPage, {
    attachTo: document.body,
    global: {
      plugins: [ElementPlus],
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

describe('preinstall template page', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    confirmMessageBox.mockResolvedValue({} as never)
    applyPreinstallTemplate.mockResolvedValue({
      code: 0,
      data: null,
    })
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('guards the default preinstall template operation with confirmation', async () => {
    mountPreinstallPage()
    await flushPromises()

    expect(wrapper?.text()).toContain('预设模板')
    expect(wrapper?.text()).toContain('会停止服务器并创建备份')

    await findButton('应用模板').trigger('click')
    await flushPromises()

    expect(confirmMessageBox).toHaveBeenCalled()
    expect(applyPreinstallTemplate).toHaveBeenCalledWith('default')
  })
})
