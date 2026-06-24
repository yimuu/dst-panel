<template>
  <el-container class="admin-layout">
    <el-aside class="admin-sidebar" width="232px">
      <div class="admin-brand">
        <span class="admin-brand__mark">饥</span>
        <span class="admin-brand__name">饥荒管理控制台</span>
      </div>

      <el-menu class="admin-menu" :default-active="activeMenuPath" @select="handleMenuSelect">
        <el-menu-item v-for="item in adminMenuItems" :key="item.path" :index="item.path">
          <el-icon>
            <component :is="item.icon" />
          </el-icon>
          <span>{{ item.label }}</span>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <el-container>
      <el-header class="admin-header">
        <div class="admin-header__title">{{ currentMenuLabel }}</div>
        <div class="admin-header__actions">
          <el-button :icon="themeIcon" circle :title="themeTitle" @click="toggleTheme" />
          <el-dropdown trigger="click" @command="handleUserCommand">
            <button class="admin-user" type="button">
              <el-icon><User /></el-icon>
              <span>{{ userName }}</span>
            </button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="profile">用户资料</el-dropdown-item>
                <el-dropdown-item divided command="logout">退出登录</el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </el-header>

      <el-main class="admin-main">
        <RouterView />
      </el-main>
    </el-container>
  </el-container>
</template>

<script setup lang="ts">
import { computed, watchEffect } from 'vue'
import { Moon, Sunny, User } from '@element-plus/icons-vue'
import { useRoute, useRouter } from 'vue-router'

import { adminMenuItems } from '@/layouts/menu'
import { routes } from '@/shared/config/routes'
import { useAuthStore } from '@/shared/stores/auth'
import { useThemeStore } from '@/shared/stores/theme'

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()
const theme = useThemeStore()

const activeMenuPath = computed(() => route.path)
const currentMenuLabel = computed(
  () => adminMenuItems.find((item) => item.path === route.path)?.label || '控制台',
)
const userName = computed(() => {
  const displayName = auth.user?.displayName
  return (typeof displayName === 'string' && displayName) || auth.user?.username || 'admin'
})
const themeIcon = computed(() => (theme.isDark ? Sunny : Moon))
const themeTitle = computed(() => (theme.isDark ? '切换浅色模式' : '切换深色模式'))

watchEffect(() => {
  document.documentElement.classList.toggle('dark', theme.isDark)
  document.documentElement.dataset.theme = theme.mode
})

function handleMenuSelect(path: string): void {
  if (path !== route.path) {
    void router.push(path)
  }
}

function toggleTheme(): void {
  theme.setMode(theme.isDark ? 'light' : 'dark')
}

async function handleUserCommand(command: string | number | object): Promise<void> {
  if (command === 'profile') {
    await router.push(routes.userProfile)
    return
  }

  if (command === 'logout') {
    await auth.logoutUser()
    await router.replace(routes.login)
  }
}
</script>
