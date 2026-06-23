import { describe, expect, it } from 'vitest'

import type { ApiEnvelope } from '@/shared/api/types'
import { isApiSuccess, normalizeApiError } from '@/shared/api/http'

describe('API HTTP helpers', () => {
  it('treats backend success codes as successful envelopes', () => {
    const zeroCode: ApiEnvelope<null> = { code: 0, data: null }
    const okCode: ApiEnvelope<null> = { code: 200, data: null }

    expect(isApiSuccess(zeroCode)).toBe(true)
    expect(isApiSuccess(okCode)).toBe(true)
  })

  it('normalizes Axios-like response errors', () => {
    const error = {
      response: {
        status: 401,
        data: {
          code: 401,
          msg: 'unauthorized',
        },
      },
    }

    expect(normalizeApiError(error)).toEqual({
      status: 401,
      message: 'unauthorized',
    })
  })
})
