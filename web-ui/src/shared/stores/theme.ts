import { computed, ref } from 'vue'
import { defineStore } from 'pinia'

type ThemeMode = 'light' | 'dark'

const defaultMode: ThemeMode = 'light'
const defaultPrimaryColor = '#409eff'

function readThemeMode(): ThemeMode {
  const value = localStorage.getItem('theme')
  return value === 'dark' || value === 'light' ? value : defaultMode
}

export const useThemeStore = defineStore('theme', () => {
  const mode = ref<ThemeMode>(readThemeMode())
  const primaryColor = ref(localStorage.getItem('primaryColor') || defaultPrimaryColor)

  const isDark = computed(() => mode.value === 'dark')

  function setMode(value: ThemeMode): void {
    mode.value = value
    localStorage.setItem('theme', value)
  }

  function setPrimaryColor(value: string): void {
    primaryColor.value = value
    localStorage.setItem('primaryColor', value)
  }

  return {
    mode,
    primaryColor,
    isDark,
    setMode,
    setPrimaryColor,
  }
})
