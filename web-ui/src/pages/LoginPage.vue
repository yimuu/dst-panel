<template>
  <section class="login-page">
    <el-card class="login-card" shadow="never">
      <template #header>
        <div class="login-card__header">
          <h1>登录控制台</h1>
          <p>使用管理员账号进入饥荒管理控制台</p>
        </div>
      </template>

      <el-alert
        v-if="errorMessage"
        :title="errorMessage"
        type="error"
        show-icon
        :closable="false"
      />

      <el-form class="login-form" label-position="top" @submit.prevent="handleSubmit">
        <el-form-item label="用户名">
          <el-input v-model="form.username" autocomplete="username" placeholder="请输入用户名" />
        </el-form-item>

        <el-form-item label="密码">
          <el-input
            v-model="form.password"
            autocomplete="current-password"
            placeholder="请输入密码"
            show-password
            type="password"
          />
        </el-form-item>

        <el-button
          class="login-form__submit"
          :loading="auth.loading || submitting"
          native-type="submit"
          type="primary"
        >
          登录
        </el-button>
      </el-form>
    </el-card>
  </section>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { resolveLoginRedirect } from '@/pages/login-redirect'
import { normalizeApiError } from '@/shared/api/http'
import { useAuthStore } from '@/shared/stores/auth'
import type { LoginRequest } from '@/shared/types/domain'

const auth = useAuthStore()
const route = useRoute()
const router = useRouter()

const form = reactive<LoginRequest>({
  username: '',
  password: '',
})
const submitting = ref(false)
const errorMessage = ref('')
const redirectTarget = computed(() => resolveLoginRedirect(route.query.redirect))

async function handleSubmit(): Promise<void> {
  errorMessage.value = ''
  submitting.value = true

  try {
    await auth.loginWithPassword(form)
  } catch (error) {
    errorMessage.value = normalizeApiError(error).message
    submitting.value = false
    return
  }

  try {
    await router.replace(redirectTarget.value)
  } catch {
    // Authentication already succeeded; navigation failures should not become login errors.
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.login-page {
  width: min(420px, 100%);
}

.login-card {
  border: 1px solid #d7dee9;
  border-radius: 8px;
}

.login-card__header {
  display: grid;
  gap: 6px;
}

.login-card__header h1,
.login-card__header p {
  margin: 0;
}

.login-card__header h1 {
  font-size: 22px;
  line-height: 1.25;
}

.login-card__header p {
  color: #667085;
  font-size: 14px;
}

.login-form {
  display: grid;
  gap: 2px;
  margin-top: 14px;
}

.login-form__submit {
  width: 100%;
  margin-top: 4px;
}
</style>
