import { flushPromises, mount, type DOMWrapper, type VueWrapper } from '@vue/test-utils'
import ElementPlus from 'element-plus'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick } from 'vue'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'

import PlayerLogPage from '@/pages/PlayerLogPage.vue'

class MockEventSource {
  static instances: MockEventSource[] = []

  readonly url: string
  readonly close = vi.fn()
  onerror: ((event: Event) => void) | null = null
  private readonly listeners: Partial<Record<string, EventListener[]>> = {}

  constructor(url: string | URL) {
    this.url = String(url)
    MockEventSource.instances.push(this)
  }

  addEventListener(type: string, listener: EventListener): void {
    this.listeners[type] = [...(this.listeners[type] ?? []), listener]
  }

  removeEventListener(type: string, listener: EventListener): void {
    this.listeners[type] = (this.listeners[type] ?? []).filter(
      (candidate) => candidate !== listener,
    )
  }

  emit(type: string, data: string): void {
    for (const listener of this.listeners[type] ?? []) {
      listener(new MessageEvent(type, { data }))
    }
  }

  emitError(): void {
    this.onerror?.(new Event('error'))
  }
}

let wrapper: VueWrapper | undefined

function mountPlayerLogPage(): VueWrapper {
  const pinia = createPinia()
  setActivePinia(pinia)

  wrapper = mount(PlayerLogPage, {
    attachTo: document.body,
    global: {
      plugins: [pinia, ElementPlus],
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

describe('player log page stream', () => {
  beforeEach(() => {
    MockEventSource.instances = []
    vi.stubGlobal('EventSource', MockEventSource)
  })

  afterEach(() => {
    wrapper?.unmount()
    wrapper = undefined
    document.body.innerHTML = ''
    vi.unstubAllGlobals()
  })

  it('shows the exact empty state text without cluster-target UI', () => {
    mountPlayerLogPage()

    expect(wrapper?.text()).not.toContain('当前集群')
    expect(wrapper?.text()).toContain('暂无日志数据')
  })

  it('opens the backend SSE log stream and closes it on disconnect', async () => {
    mountPlayerLogPage()

    await findButton('连接').trigger('click')
    await flushPromises()

    const stream = MockEventSource.instances[0]

    expect(stream?.url).toBe('/api/game/log/stream?levelName=Master')
    expect(wrapper?.text()).toContain('已连接')

    await findButton('断开').trigger('click')

    expect(stream?.close).toHaveBeenCalledTimes(1)
  })

  it('closes an active stream when unmounted', async () => {
    mountPlayerLogPage()

    await findButton('连接').trigger('click')
    await flushPromises()

    const stream = MockEventSource.instances[0]

    wrapper?.unmount()

    expect(stream?.close).toHaveBeenCalledTimes(1)
  })

  it('keeps the latest 1000 log rows and filters by severity', async () => {
    mountPlayerLogPage()

    await findButton('连接').trigger('click')
    await flushPromises()

    const stream = MockEventSource.instances[0]

    for (let index = 0; index < 1005; index += 1) {
      stream?.emit('log', index === 1004 ? '[ERROR] line-1004' : `line-${index}`)
    }
    await nextTick()

    expect(wrapper?.findAll('[data-test="log-row"]')).toHaveLength(1000)
    expect(wrapper?.text()).not.toContain('line-0')
    expect(wrapper?.text()).toContain('line-1004')

    await wrapper?.find('[data-test="severity-error"]').trigger('click')
    await nextTick()

    expect(wrapper?.findAll('[data-test="log-row"]')).toHaveLength(1)
    expect(wrapper?.text()).toContain('line-1004')
    expect(wrapper?.text()).not.toContain('line-1003')
  })

  it('clears stale stream warnings when logs resume after reconnect', async () => {
    mountPlayerLogPage()

    await findButton('连接').trigger('click')
    await flushPromises()

    const stream = MockEventSource.instances[0]

    stream?.emitError()
    await nextTick()

    expect(wrapper?.text()).toContain('日志流连接异常，正在等待重试')

    stream?.emit('log', 'player joined')
    await nextTick()

    expect(wrapper?.text()).not.toContain('日志流连接异常，正在等待重试')
    expect(wrapper?.text()).toContain('player joined')
  })
})
