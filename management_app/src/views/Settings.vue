<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { getPolicy, updatePolicy, type PolicyConfig } from '../api'

const form = ref<PolicyConfig>({
  max_retry_count: 3,
  auth_timeout_secs: 30,
  allow_emergency_override: true,
  emergency_requires_reason: true,
  offline_cache_enabled: true,
})
const loading = ref(false)
const saving = ref(false)

async function load() {
  loading.value = true
  try {
    const { data } = await getPolicy()
    form.value = data
  } catch (e: any) { ElMessage.error('加载失败') }
  loading.value = false
}

async function save() {
  saving.value = true
  try {
    await updatePolicy(form.value)
    ElMessage.success('保存成功')
  } catch (e: any) { ElMessage.error('保存失败: ' + e.message) }
  saving.value = false
}

onMounted(load)
</script>

<template>
  <el-card v-loading="loading" style="max-width: 600px">
    <template #header>认证策略配置</template>

    <el-form :model="form" label-width="160px" label-position="left">
      <el-form-item label="最大重试次数">
        <el-input-number v-model="form.max_retry_count" :min="1" :max="10" />
      </el-form-item>
      <el-form-item label="认证超时 (秒)">
        <el-input-number v-model="form.auth_timeout_secs" :min="5" :max="120" :step="5" />
      </el-form-item>
      <el-form-item label="允许应急覆盖">
        <el-switch v-model="form.allow_emergency_override" />
      </el-form-item>
      <el-form-item label="应急需填写原因">
        <el-switch v-model="form.emergency_requires_reason" />
      </el-form-item>
      <el-form-item label="离线缓存">
        <el-switch v-model="form.offline_cache_enabled" />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" :loading="saving" @click="save">保存配置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>
