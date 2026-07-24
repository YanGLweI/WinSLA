<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getEmergency, addEmergency, deleteEmergency, type EmergencyAccount } from '../api'

const accounts = ref<EmergencyAccount[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const form = ref({ sid: '', username: '', reason: '' })

async function load() {
  loading.value = true
  try {
    const { data } = await getEmergency()
    accounts.value = data
  } catch (e: any) { ElMessage.error('加载失败: ' + e.message) }
  loading.value = false
}

async function handleAdd() {
  if (!form.value.username || !form.value.sid) {
    ElMessage.warning('请填写用户名和 SID')
    return
  }
  try {
    await addEmergency(form.value)
    ElMessage.success('添加成功')
    dialogVisible.value = false
    form.value = { sid: '', username: '', reason: '' }
    load()
  } catch (e: any) { ElMessage.error('添加失败: ' + e.message) }
}

async function handleDelete(row: EmergencyAccount) {
  await ElMessageBox.confirm(`确定移除应急账号 "${row.username}"？`, '确认', { type: 'warning' })
  try {
    await deleteEmergency(row.id)
    ElMessage.success('已移除')
    load()
  } catch (e: any) { ElMessage.error('操作失败: ' + e.message) }
}

onMounted(load)
</script>

<template>
  <div class="page">
    <div class="page-toolbar">
      <span class="page-title">应急覆盖账号</span>
      <span class="page-count">{{ accounts.length }} 个</span>
      <div class="toolbar-actions">
        <el-button size="small" @click="load">刷新</el-button>
        <el-button size="small" type="warning" @click="dialogVisible = true">+ 新增</el-button>
      </div>
    </div>

    <div class="notice-bar">
      应急账号可在双人验证不可用时单独登录，所有应急登录均记录审计日志。
    </div>

    <div class="table-wrap">
      <el-table :data="accounts" v-loading="loading" size="small" border stripe height="100%">
        <el-table-column prop="username" label="用户名" min-width="100" />
        <el-table-column prop="sid" label="SID" min-width="180" show-overflow-tooltip />
        <el-table-column prop="reason" label="原因" min-width="140" show-overflow-tooltip />
        <el-table-column prop="approved_by" label="审批人" width="80" />
        <el-table-column prop="activated_at" label="激活时间" width="160" />
        <el-table-column label="操作" width="60" align="center">
          <template #default="{ row }">
            <el-button type="danger" size="small" link @click="handleDelete(row)">移除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <el-dialog v-model="dialogVisible" title="新增应急账号" width="400px" :close-on-click-modal="false">
      <el-form :model="form" label-width="60px" size="small">
        <el-form-item label="用户名">
          <el-input v-model="form.username" placeholder="管理员账号" />
        </el-form-item>
        <el-form-item label="SID">
          <el-input v-model="form.sid" placeholder="S-1-5-21-..." />
        </el-form-item>
        <el-form-item label="原因">
          <el-input v-model="form.reason" type="textarea" :rows="2" placeholder="授权原因" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="dialogVisible = false">取消</el-button>
        <el-button size="small" type="primary" @click="handleAdd">确认</el-button>
      </template>
    </el-dialog>
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
.notice-bar {
  padding: 6px 12px; font-size: 11px; color: #92400e;
  background: #fef3c7; border: 1px solid #fde68a; border-top: none;
}
.table-wrap { flex: 1; border: 1px solid #e2e8f0; border-top: none; border-radius: 0 0 6px 6px; overflow: hidden; background: #fff; }
</style>
