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
  <el-card>
    <template #header>
      <div style="display: flex; justify-content: space-between; align-items: center">
        <span>应急覆盖账号</span>
        <el-button type="warning" size="small" @click="dialogVisible = true">
          <el-icon><Plus /></el-icon> 新增
        </el-button>
      </div>
    </template>

    <el-alert type="warning" :closable="false" style="margin-bottom: 16px">
      应急账号可在双人验证不可用时单独登录，所有应急登录均记录审计日志。
    </el-alert>

    <el-table :data="accounts" v-loading="loading" stripe border>
      <el-table-column prop="username" label="用户名" min-width="120" />
      <el-table-column prop="sid" label="SID" min-width="200" show-overflow-tooltip />
      <el-table-column prop="reason" label="原因" min-width="150" show-overflow-tooltip />
      <el-table-column prop="approved_by" label="审批人" width="100" />
      <el-table-column prop="activated_at" label="激活时间" width="180" />
      <el-table-column label="操作" width="80" align="center">
        <template #default="{ row }">
          <el-button type="danger" size="small" link @click="handleDelete(row)">移除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-empty v-if="!loading && accounts.length === 0" description="暂无应急账号" />
  </el-card>

  <el-dialog v-model="dialogVisible" title="新增应急账号" width="480px">
    <el-form :model="form" label-width="80px">
      <el-form-item label="用户名">
        <el-input v-model="form.username" placeholder="管理员账号名" />
      </el-form-item>
      <el-form-item label="SID">
        <el-input v-model="form.sid" placeholder="S-1-5-21-..." />
      </el-form-item>
      <el-form-item label="原因">
        <el-input v-model="form.reason" type="textarea" placeholder="授权原因说明" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="dialogVisible = false">取消</el-button>
      <el-button type="primary" @click="handleAdd">确认</el-button>
    </template>
  </el-dialog>
</template>
