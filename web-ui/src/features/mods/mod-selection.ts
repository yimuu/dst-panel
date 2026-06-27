export function toggleSelectedMod(selectedIds: string[], modId: string): string[] {
  const normalizedModId = modId.trim()
  if (!normalizedModId) {
    return selectedIds
  }

  return selectedIds.includes(normalizedModId)
    ? selectedIds.filter((selectedId) => selectedId !== normalizedModId)
    : [...selectedIds, normalizedModId]
}
