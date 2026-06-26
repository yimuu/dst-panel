<template>
  <PageState title="个人信息" description="查看当前登录账号信息并修改登录密码。">
    <el-row :gutter="12">
      <el-col :xs="24" :lg="14">
        <el-card shadow="never">
          <template #header>账号资料</template>

          <el-descriptions :column="2" border>
            <el-descriptions-item label="用户名">{{ displayName }}</el-descriptions-item>
            <el-descriptions-item label="角色">{{ roleName }}</el-descriptions-item>
            <el-descriptions-item label="账号 ID">{{ accountId }}</el-descriptions-item>
            <el-descriptions-item label="创建时间">{{ createdAt }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>

      <el-col :xs="24" :lg="10">
        <el-card shadow="never">
          <template #header>账号安全</template>

          <el-form label-position="top" @submit.prevent>
            <el-form-item label="新密码">
              <div class="field-control" data-test="new-password-input">
                <el-input
                  v-model="passwordForm.newPassword"
                  autocomplete="new-password"
                  placeholder="请输入新密码"
                  show-password
                  type="password"
                />
              </div>
              <p class="field-hint">保存后，下次登录将使用新密码。</p>
            </el-form-item>

            <el-form-item>
              <el-button
                :disabled="passwordForm.newPassword.trim().length === 0"
                :icon="Lock"
                :loading="savingPassword"
                type="primary"
                @click="handleChangePassword"
              >
                保存密码
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
    </el-row>
  </PageState>
</template>

<script setup lang="ts">
import { Lock } from '@element-plus/icons-vue'
import { ElMessage } from 'element-plus'
import { computed, reactive, ref } from 'vue'

import { changePassword } from '@/features/auth/auth.api'
import { assertApiSuccess, getErrorMessage } from '@/shared/api/envelope'
import PageState from '@/shared/components/PageState.vue'
import { useAuthStore } from '@/shared/stores/auth'

const auth = useAuthStore()
const passwordForm = reactive({
  newPassword: '',
})
const savingPassword = ref(false)

const displayName = computed(
  () => auth.user?.displayName || auth.user?.username || auth.user?.name || '未登录',
)
const roleName = computed(() => auth.user?.role || '管理员')
const accountId = computed(() => auth.user?.id || auth.user?.ID || '待接入')
const createdAt = computed(() => auth.user?.createdAt || '待接入')

async function handleChangePassword(): Promise<void> {
  if (passwordForm.newPassword.trim().length === 0) {
    ElMessage.error('请输入新密码')
    return
  }

  savingPassword.value = true

  try {
    assertApiSuccess(await changePassword({ newPassword: passwordForm.newPassword }))
    passwordForm.newPassword = ''
    ElMessage.success('密码已更新')
  } catch (error) {
    ElMessage.error(getErrorMessage(error, '密码更新失败'))
  } finally {
    savingPassword.value = false
  }
}
</script>

<style scoped>
.field-hint {
  width: 100%;
  margin: 6px 0 0;
  color: #667085;
  font-size: 13px;
  line-height: 1.5;
}

.field-control {
  width: 100%;
}
</style>
