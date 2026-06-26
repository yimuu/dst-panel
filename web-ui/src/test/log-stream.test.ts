import { describe, expect, it } from 'vitest'

import { buildLogStreamUrl } from '@/features/logs/log-stream'

describe('log stream', () => {
  it('builds same-origin websocket urls for browser usage', () => {
    expect(buildLogStreamUrl('http://127.0.0.1:5173', 'Cluster_1')).toBe(
      'ws://127.0.0.1:5173/ws/log?cluster=Cluster_1',
    )
  })

  it('uses secure websocket urls on https origins', () => {
    expect(buildLogStreamUrl('https://panel.example.com', 'Cluster 1')).toBe(
      'wss://panel.example.com/ws/log?cluster=Cluster+1',
    )
  })
})
