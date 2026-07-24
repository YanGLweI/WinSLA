<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getAudit, type AuditEntry } from '../api'

const entries = ref<AuditEntry[]>([])
const loading = ref(false)
const limit = ref(50)

async function load() {
  loading.value = true
  try {
    const { data } = await getAudit(limit.value)
    entries.value = data
  } catch { entries.value = [] }
  loading.value = false
}

function resultTag(result: string) {
  if (result === 'success') return 'success'
  if (result.includes('fail')) return 'danger'
  if (result === 'timeout') return 'warning'
  return 'info'
}

onMounted(load)
</script>

<template>
  <el-card>
    <template #header>
      <div style="display: flex; justify-content: space-between; align-items: center">
        <span>认证审计日志</span>
        <div>
          <el-select v-model="limit" size="small" style="width: 100px; margin-right: 8px" @change="load">
            <el-option :value="20" label="最近 20" />
            <el-option :value="50" label="最近 50" />
            <el-option :value="100" label="最近 100" />
          </el-select>
          <el-button size="small" @click="load">
            <el-icon><Refresh /></el-icon> 刷新
          </el-button>
        </div>
      </div>
    </template>

    <el-table :data="entries" v-loading="loading" stripe border size="small">
      <el-table-column prop="timestamp" label="时间" width="170" />
      <el-table-column prop="user_a_sid" label="用户 A" min-width="140" show-overflow-tooltip />
      <el-table-column prop="user_b_sid" label="用户 B" min-width="140" show-overflow-tooltip />
      <el-table-column label="结果" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="resultTag(row.result)" size="small">{{ row.result }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="error_message" label="错误信息" min-width="160" show-overflow-tooltip />
      <el-table-column prop="client_hostname" label="主机" width="120" />
    </el-table>

    <el-empty v-if="!loading && entries.length === 0" description="暂无审计记录" />
  </el-card>
</template>
