<template>
  <PageState title="初始化" description="首次运行时创建管理员账号并初始化默认世界。">
    <el-alert v-if="errorMessage" :title="errorMessage" type="error" show-icon :closable="false" />

    <el-card class="init-card" shadow="never">
      <template #header>
        <div class="init-card__header">
          <h1>创建管理员账号</h1>
          <p>初始化完成后即可返回登录页进入控制台。</p>
        </div>
      </template>

      <el-form
        v-loading="checking"
        class="init-form"
        label-position="top"
        @submit.prevent="handleSubmit"
      >
        <el-form-item label="用户名" required>
          <el-input v-model.trim="form.username" autocomplete="username" placeholder="admin" />
        </el-form-item>

        <el-form-item label="显示名称">
          <el-input v-model.trim="form.displayName" placeholder="管理员" />
        </el-form-item>

        <el-form-item label="头像地址">
          <el-input v-model.trim="form.photoURL" placeholder="https://example.com/avatar.png" />
        </el-form-item>

        <el-form-item label="密码" required>
          <el-input
            v-model="form.password"
            autocomplete="new-password"
            placeholder="请输入密码"
            show-password
            type="password"
          />
        </el-form-item>

        <el-form-item label="确认密码" required>
          <el-input
            v-model="confirmPassword"
            autocomplete="new-password"
            placeholder="请再次输入密码"
            show-password
            type="password"
          />
        </el-form-item>

        <el-button
          class="init-form__submit"
          :disabled="checking"
          :loading="submitting"
          native-type="submit"
          type="primary"
        >
          初始化
        </el-button>
      </el-form>
    </el-card>
  </PageState>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue'
import { useRouter } from 'vue-router'

import { getInitStatus, initialize } from '@/features/auth/auth.api'
import { isApiSuccess, normalizeApiError } from '@/shared/api/http'
import PageState from '@/shared/components/PageState.vue'

const router = useRouter()
const checking = ref(true)
const submitting = ref(false)
const errorMessage = ref('')
const confirmPassword = ref('')
const form = reactive({
  username: '',
  displayName: '',
  photoURL: '',
  password: '',
})

onMounted(() => {
  void loadInitStatus()
})

async function loadInitStatus(): Promise<void> {
  checking.value = true

  try {
    const response = await getInitStatus()
    if (!isApiSuccess(response)) {
      await router.replace('/login')
    }
  } catch (error) {
    errorMessage.value = normalizeApiError(error).message
  } finally {
    checking.value = false
  }
}

async function handleSubmit(): Promise<void> {
  errorMessage.value = ''

  if (!form.username.trim()) {
    errorMessage.value = '请填写用户名'
    return
  }

  if (!form.password) {
    errorMessage.value = '请填写密码'
    return
  }

  if (form.password !== confirmPassword.value) {
    errorMessage.value = '两次输入的密码不一致'
    return
  }

  submitting.value = true

  try {
    const response = await initialize({
      userInfo: {
        username: form.username,
        displayName: form.displayName,
        photoURL: form.photoURL,
        password: form.password,
      },
    })

    if (!isApiSuccess(response)) {
      throw new Error(response.msg || response.message || '初始化失败')
    }

    await router.replace('/login')
  } catch (error) {
    errorMessage.value = normalizeApiError(error).message
  } finally {
    submitting.value = false
  }
}
</script>

<style scoped>
.init-card {
  max-width: 520px;
  border: 1px solid #d7dee9;
  border-radius: 8px;
}

.init-card__header {
  display: grid;
  gap: 6px;
}

.init-card__header h1,
.init-card__header p {
  margin: 0;
}

.init-card__header h1 {
  font-size: 22px;
  line-height: 1.25;
}

.init-card__header p {
  color: #667085;
  font-size: 14px;
}

.init-form {
  display: grid;
  gap: 2px;
}

.init-form__submit {
  width: 100%;
}
</style>
