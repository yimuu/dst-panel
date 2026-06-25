export function formatWorkshopId(value: string): string {
  return value.trim().replace(/^workshop-/i, '')
}

export function toggleModId(selectedIds: string[], modId: string): string[] {
  return selectedIds.includes(modId)
    ? selectedIds.filter((id) => id !== modId)
    : [...selectedIds, modId]
}
