import { describe, expect, it } from 'vitest'

import { assertApiSuccess, getErrorMessage, readApiData } from '@/shared/api/envelope'
import type { ApiEnvelope } from '@/shared/api/types'

describe('API envelope helpers', () => {
  it('reads data from backend success envelopes', () => {
    expect(readApiData({ code: 0, data: 'ok' }, '加载失败')).toBe('ok')
    expect(readApiData({ code: 200, data: { saved: true } }, '加载失败')).toEqual({ saved: true })
  })

  it('throws backend message fields before using fallback text', () => {
    const msgEnvelope: ApiEnvelope<null> = { code: 500, data: null, msg: '后端错误' }
    const messageEnvelope: ApiEnvelope<null> = { code: 500, data: null, message: '业务错误' }
    const emptyEnvelope: ApiEnvelope<null> = { code: 500, data: null }

    expect(() => readApiData(msgEnvelope, '加载失败')).toThrow('后端错误')
    expect(() => readApiData(messageEnvelope, '加载失败')).toThrow('业务错误')
    expect(() => readApiData(emptyEnvelope, '加载失败')).toThrow('加载失败')
  })

  it('asserts API success with a configurable fallback', () => {
    expect(() => assertApiSuccess({ code: 0, data: null })).not.toThrow()
    expect(() => assertApiSuccess({ code: 500, data: null }, '保存失败')).toThrow('保存失败')
  })

  it('normalizes thrown values for Chinese UI messages', () => {
    expect(getErrorMessage(new Error('网络异常'), '请求失败')).toBe('网络异常')
    expect(getErrorMessage('字符串错误', '请求失败')).toBe('字符串错误')
    expect(getErrorMessage({}, '请求失败')).toBe('请求失败')
  })
})
