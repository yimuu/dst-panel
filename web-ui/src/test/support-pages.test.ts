import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus, { ElMessage } from 'element-plus'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import * as authApi from '@/features/auth/auth.api'
import * as settingsApi from '@/features/settings/settings.api'
import HelpPage from '@/pages/HelpPage.vue'
import LobbyPage from '@/pages/LobbyPage.vue'
import SettingsPage from '@/pages/SettingsPage.vue'
import UserProfilePage from '@/pages/UserProfilePage.vue'
import type { ApiEnvelope } from '@/shared/api/types'
import { useAuthStore } from '@/shared/stores/auth'
import type { DstConfig } from '@/features/settings/settings.api'

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

vi.mock('@/features/settings/settings.api', () => ({
  getDstConfig: vi.fn(),
  saveDstConfig: vi.fn(),
  getLobbyServerDetail: vi.fn(),
}))

vi.mock('@/features/auth/auth.api', () => ({
  getInitStatus: vi.fn(),
  initialize: vi.fn(),
  login: vi.fn(),
  logout: vi.fn(),
  getUser: vi.fn(),
  updateUser: vi.fn(),
  changePassword: vi.fn(),
}))

const getDstConfig = vi.mocked(settingsApi.getDstConfig)
const saveDstConfig = vi.mocked(settingsApi.saveDstConfig)
const getLobbyServerDetail = vi.mocked(settingsApi.getLobbyServerDetail)
const changePassword = vi.mocked(authApi.changePassword)

let wrapper: VueWrapper | undefined

const dstConfigFixture: DstConfig = {
  steamcmd: '/opt/steamcmd',
  force_install_dir: '/srv/dst',
  donot_starve_server_directory: '',
  cluster: 'Cluster_1',
  backup: '/srv/backup',
  mod_download_path: '/srv/mods',
  bin: 64,
  beta: 0,
  ugc_directory: '',
  persistent_storage_root: '/srv/klei',
  conf_dir: 'DoNotStarveTogether',
}

function success<T>(data: T): ApiEnvelope<T> {
  return {
    code: 0,
    data,
  }
}

function mountPage(component: Parameters<typeof mount>[0]): VueWrapper {
  const pinia = createPinia()
  setActivePinia(pinia)

  wrapper = mount(component, {
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

describe('support pages', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    getDstConfig.mockResolvedValue(success(dstConfigFixture))
    saveDstConfig.mockResolvedValue(success(null))
    getLobbyServerDetail.mockResolvedValue(
      success({
        name: '测试房间',
        connected: 2,
        maxconnections: 6,
        mode: 'survival',
        season: 'autumn',
        playerList: [{ name: 'Alice' }],
        dayData: { day: 12 },
      }),
    )
    changePassword.mockResolvedValue(success(null))
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
  })

  it('saves the complete DST config payload after editing one path field', async () => {
    mountPage(SettingsPage)
    await flushPromises()

    await wrapper
      ?.find<HTMLInputElement>('[data-test="force-install-dir-input"] input')
      .setValue('/srv/dst-new')
    await findButton('保存设置').trigger('click')
    await flushPromises()

    expect(saveDstConfig).toHaveBeenCalledWith({
      ...dstConfigFixture,
      force_install_dir: '/srv/dst-new',
    })
  })

  it('blocks settings save when required fields are empty', async () => {
    mountPage(SettingsPage)
    await flushPromises()

    const forceInstallInput = wrapper?.find<HTMLInputElement>(
      '[data-test="force-install-dir-input"] input',
    )

    await forceInstallInput?.setValue('')
    await findButton('保存设置').trigger('click')
    await flushPromises()

    expect(saveDstConfig).not.toHaveBeenCalled()
    expect(ElMessage.error).toHaveBeenCalledWith('请填写游戏安装目录')
  })

  it('saves trimmed settings payload after validation passes', async () => {
    mountPage(SettingsPage)
    await flushPromises()

    await wrapper?.find<HTMLInputElement>('[data-test="steamcmd-input"] input').setValue('  /opt/steamcmd  ')
    await wrapper
      ?.find<HTMLInputElement>('[data-test="force-install-dir-input"] input')
      .setValue('  /srv/dst  ')
    await wrapper?.find<HTMLInputElement>('[data-test="cluster-input"] input').setValue('  Cluster_1  ')
    await wrapper?.find<HTMLInputElement>('[data-test="backup-input"] input').setValue('  /srv/backup  ')
    await wrapper
      ?.find<HTMLInputElement>('[data-test="mod-download-path-input"] input')
      .setValue('  /srv/mods  ')

    await findButton('保存设置').trigger('click')
    await flushPromises()

    expect(saveDstConfig).toHaveBeenCalledWith({
      ...dstConfigFixture,
      steamcmd: '/opt/steamcmd',
      force_install_dir: '/srv/dst',
      cluster: 'Cluster_1',
      backup: '/srv/backup',
      mod_download_path: '/srv/mods',
    })
  })

  it('changes the current user password with the supported payload', async () => {
    mountPage(UserProfilePage)
    useAuthStore().user = {
      id: 1,
      username: 'admin',
      displayName: '管理员',
    }
    await flushPromises()

    await wrapper
      ?.find<HTMLInputElement>('[data-test="new-password-input"] input')
      .setValue('new-password-123')
    await findButton('保存密码').trigger('click')
    await flushPromises()

    expect(changePassword).toHaveBeenCalledWith({
      newPassword: 'new-password-123',
    })
  })

  it('renders real help links instead of disabled dead ends', () => {
    mountPage(HelpPage)

    const hrefs = wrapper?.findAll('a').map((link) => link.attributes('href'))

    expect(hrefs).toEqual(
      expect.arrayContaining([
        '/misc/Docker-compose.md',
        '/misc/DontStarveMultiWorldTotorial.md',
        '/misc/FQA.md',
      ]),
    )
    expect(wrapper?.findAll('button[disabled]').length).toBe(0)
  })

  it('queries lobby detail in read-only mode without publishing actions', async () => {
    mountPage(LobbyPage)

    expect(wrapper?.text()).not.toContain('发布大厅信息')

    await wrapper
      ?.find<HTMLInputElement>('[data-test="lobby-row-id-input"] input')
      .setValue('row-1')
    await findButton('查询').trigger('click')
    await flushPromises()

    expect(getLobbyServerDetail).toHaveBeenCalledWith({
      region: 'ap-southeast-1',
      rowId: 'row-1',
    })
    expect(wrapper?.text()).toContain('测试房间')
    expect(wrapper?.text()).toContain('Alice')
    expect(wrapper?.text()).toContain('第 12 天')
  })
})
