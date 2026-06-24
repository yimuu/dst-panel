import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

type ThemeMode = 'light' | 'dark'

const defaultMode: ThemeMode = 'light'
const defaultPrimaryColor = '#409eff'

function readStorage(key: string): string | null {
  try {
    return localStorage.getItem(key)
  } catch {
    return null
  }
}

function writeStorage(key: string, value: string): void {
  try {
    localStorage.setItem(key, value)
  } catch {
    // State still updates in memory when browser storage is unavailable.
  }
}

function readThemeMode(): ThemeMode {
  const value = readStorage('theme')
  return value === 'dark' || value === 'light' ? value : defaultMode
}

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>(readThemeMode())
  const primaryColor = ref(readStorage('primaryColor') || defaultPrimaryColor)

  const isDark = computed(() => mode.value === 'dark')

  function setMode(value: ThemeMode): void {
    mode.value = value
    writeStorage('theme', value)
  }

  function setPrimaryColor(value: string): void {
    primaryColor.value = value
    writeStorage('primaryColor', value)
  }

  return {
    mode,
    primaryColor,
    isDark,
    setMode,
    setPrimaryColor,
  }
})
