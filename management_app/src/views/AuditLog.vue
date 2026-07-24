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
  <div class="page">
    <div class="page-toolbar">
      <span class="page-title">认证审计日志</span>
      <span class="page-count">{{ entries.length }} 条</span>
      <div class="toolbar-actions">
        <el-select v-model="limit" size="small" style="width: 90px" @change="load">
          <el-option :value="20" label="20 条" />
          <el-option :value="50" label="50 条" />
          <el-option :value="100" label="100 条" />
        </el-select>
        <el-button size="small" @click="load">刷新</el-button>
      </div>
    </div>

    <div class="table-wrap">
      <el-table :data="entries" v-loading="loading" size="small" border stripe height="100%">
        <el-table-column prop="timestamp" label="时间" width="155" />
        <el-table-column prop="user_a_sid" label="用户 A" min-width="130" show-overflow-tooltip />
        <el-table-column prop="user_b_sid" label="用户 B" min-width="130" show-overflow-tooltip />
        <el-table-column label="结果" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="resultTag(row.result)" size="small" effect="plain">{{ row.result }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="error_message" label="错误信息" min-width="140" show-overflow-tooltip />
        <el-table-column prop="client_hostname" label="主机" width="100" />
      </el-table>
    </div>
  </div>
</template>

<style scoped>
.page { display: flex; flex-direction: column; height: 100%; }
.page-toolbar {
  display: flex; align-items: center; gap: 10px;
  padding: 8px 12px;
  background: #fff; border: 1px solid #e2e8f0; border-radius: 6px 6px 0 0;
  border-bottom: none;
}
.page-title { font-size: 13px; font-weight: 600; color: #1e293b; }
.page-count { font-size: 11px; color: #94a3b8; }
.toolbar-actions { margin-left: auto; display: flex; gap: 6px; }
.table-wrap { flex: 1; border: 1px solid #e2e8f0; border-radius: 0 0 6px 6px; overflow: hidden; background: #fff; }
</style>
