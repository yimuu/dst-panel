import { describe, expect, it } from 'vitest'

import { assertApiSuccess, getErrorMessage, isApiSuccess, readApiData } from '@/shared/api/envelope'

describe('api envelope helpers', () => {
  it('reads data from successful envelopes', () => {
    const envelope = { code: 200, data: { name: '森林' } }

    expect(isApiSuccess(envelope)).toBe(true)
    expect(readApiData(envelope)).toEqual({ name: '森林' })
    expect(assertApiSuccess(envelope)).toEqual({ name: '森林' })
  })

  it('uses message fields for failed envelopes', () => {
    expect(getErrorMessage({ code: 500, msg: '保存失败' })).toBe('保存失败')
    expect(getErrorMessage({ code: 500, message: '登录失败' })).toBe('登录失败')
  })
})
