import { ElMessageBox } from 'element-plus'

type ConfirmActionOptions = {
  cancelButtonText?: string
  confirmButtonText?: string
  type?: 'success' | 'warning' | 'info' | 'error'
}

export async function confirmAction(
  message: string,
  title: string,
  options: ConfirmActionOptions = {},
): Promise<boolean> {
  try {
    await ElMessageBox.confirm(message, title, {
      cancelButtonText: options.cancelButtonText ?? '取消',
      confirmButtonText: options.confirmButtonText ?? '确定',
      type: options.type ?? 'warning',
    })

    return true
  } catch {
    return false
  }
}
