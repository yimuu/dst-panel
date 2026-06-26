import { flushPromises, mount, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as authApi from '@/features/auth/auth.api'
import InitPage from '@/pages/InitPage.vue'

const replace = vi.fn()

vi.mock('vue-router', async () => {
  const actual = await vi.importActual<typeof import('vue-router')>('vue-router')

  return {
    ...actual,
    useRouter: () => ({
      replace,
    }),
  }
})

vi.mock('@/features/auth/auth.api', () => ({
  getInitStatus: vi.fn(),
  initialize: vi.fn(),
}))

const getInitStatus = vi.mocked(authApi.getInitStatus)
const initialize = vi.mocked(authApi.initialize)

let wrapper: VueWrapper | undefined

function mountInitPage(): VueWrapper {
  wrapper = mount(InitPage, {
    global: {
      plugins: [ElementPlus],
    },
  })

  return wrapper
}

function textInput(index: number): HTMLInputElement {
  const input = wrapper?.findAll<HTMLInputElement>('.el-input__inner')[index]?.element

  if (!input) {
    throw new Error(`未找到输入框：${index}`)
  }

  return input
}

describe('init page', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    getInitStatus.mockResolvedValue({ code: 200, msg: 'is first', data: null })
    initialize.mockResolvedValue({ code: 0, data: null })
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('loads first-run status and renders the initialization form', async () => {
    mountInitPage()
    await flushPromises()

    expect(getInitStatus).toHaveBeenCalled()
    expect(wrapper?.text()).toContain('创建管理员账号')
    expect(wrapper?.text()).toContain('初始化')
  })

  it('submits the backend userInfo payload and redirects to login', async () => {
    mountInitPage()
    await flushPromises()

    await wrapper?.findAll('.el-input__inner')[0]?.setValue('admin')
    await wrapper?.findAll('.el-input__inner')[1]?.setValue('管理员')
    await wrapper?.findAll('.el-input__inner')[2]?.setValue('https://example.test/avatar.png')
    await wrapper?.findAll('.el-input__inner')[3]?.setValue('secret')
    await wrapper?.findAll('.el-input__inner')[4]?.setValue('secret')
    await wrapper?.find('form').trigger('submit.prevent')
    await flushPromises()

    expect(initialize).toHaveBeenCalledWith({
      userInfo: {
        username: 'admin',
        displayName: '管理员',
        photoURL: 'https://example.test/avatar.png',
        password: 'secret',
      },
    })
    expect(replace).toHaveBeenCalledWith('/login')
  })

  it('redirects to login when the server reports setup is complete', async () => {
    getInitStatus.mockResolvedValue({ code: 400, msg: 'is not first', data: null })

    mountInitPage()
    await flushPromises()

    expect(replace).toHaveBeenCalledWith('/login')
  })

  it('blocks submission when passwords do not match', async () => {
    mountInitPage()
    await flushPromises()

    textInput(0).value = 'admin'
    await wrapper?.findAll('.el-input__inner')[0]?.trigger('input')
    await wrapper?.findAll('.el-input__inner')[3]?.setValue('secret')
    await wrapper?.findAll('.el-input__inner')[4]?.setValue('different')
    await wrapper?.find('form').trigger('submit.prevent')
    await flushPromises()

    expect(wrapper?.text()).toContain('两次输入的密码不一致')
    expect(initialize).not.toHaveBeenCalled()
  })
})
