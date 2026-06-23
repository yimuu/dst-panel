import { ref } from 'vue'
import { defineStore } from 'pinia'

export const useAppStore = defineStore('app', () => {
  const sidebarCollapsed = ref(false)
  const globalLoading = ref(false)

  function setSidebarCollapsed(value: boolean): void {
    sidebarCollapsed.value = value
  }

  function setGlobalLoading(value: boolean): void {
    globalLoading.value = value
  }

  return {
    sidebarCollapsed,
    globalLoading,
    setSidebarCollapsed,
    setGlobalLoading,
  }
})
