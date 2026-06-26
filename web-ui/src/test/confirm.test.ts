import { ElMessageBox } from 'element-plus'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { confirmAction } from '@/shared/ui/confirm'

vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')

  return {
    ...actual,
    ElMessageBox: {
      confirm: vi.fn(),
    },
  }
})

const confirmMessageBox = vi.mocked(ElMessageBox.confirm)

describe('confirmAction', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('returns true after user confirmation with default button text', async () => {
    confirmMessageBox.mockResolvedValue({} as never)

    await expect(confirmAction('确定执行吗？', '确认操作')).resolves.toBe(true)

    expect(confirmMessageBox).toHaveBeenCalledWith('确定执行吗？', '确认操作', {
      cancelButtonText: '取消',
      confirmButtonText: '确定',
      type: 'warning',
    })
  })

  it('returns false when user cancels', async () => {
    confirmMessageBox.mockRejectedValue(new Error('cancel'))

    await expect(confirmAction('确定删除吗？', '删除确认')).resolves.toBe(false)
  })

  it('allows custom confirm text', async () => {
    confirmMessageBox.mockResolvedValue({} as never)

    await expect(
      confirmAction('确定恢复吗？', '恢复备份', {
        confirmButtonText: '恢复',
      }),
    ).resolves.toBe(true)

    expect(confirmMessageBox).toHaveBeenCalledWith('确定恢复吗？', '恢复备份', {
      cancelButtonText: '取消',
      confirmButtonText: '恢复',
      type: 'warning',
    })
  })
})
