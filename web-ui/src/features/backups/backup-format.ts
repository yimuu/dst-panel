export function formatBackupSize(bytes: number | undefined): string {
  const value = Number(bytes ?? 0)
  if (!Number.isFinite(value) || value <= 0) {
    return '0 B'
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let size = value
  let unitIndex = 0
  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024
    unitIndex += 1
  }

  return unitIndex === 0 ? `${Math.round(size)} B` : `${size.toFixed(2)} ${units[unitIndex]}`
}

export function formatBackupTime(timestamp: number | string | undefined): string {
  const value =
    typeof timestamp === 'string' ? Number.parseFloat(timestamp) : Number(timestamp ?? 0)
  if (!Number.isFinite(value) || value <= 0) {
    return '-'
  }

  const date = new Date(value * 1000)
  return `${date.getFullYear()}-${date.getMonth() + 1}-${date.getDate()} ${date.getHours()}:${pad(
    date.getMinutes(),
  )}:${pad(date.getSeconds())}`
}

function pad(value: number): string {
  return value.toString().padStart(2, '0')
}
