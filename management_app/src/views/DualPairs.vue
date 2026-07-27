<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getPairs, addPair, deletePair, validateAccount as validateAccountApi, type DualPair } from '../api'

const pairs = ref<DualPair[]>([])
const loading = ref(false)
const dialogVisible = ref(false)

const form = ref({
  account_username: '', 
  account_password: '', 
  account_sid: '',
  approver_username: '', 
  approver_password: '', 
  approver_sid: '',
})
const validatingAccount = ref(false)
const validatingApprover = ref(false)
const validatedAccount = ref(false)
const validatedApprover = ref(false)

async function load() {
  loading.value = true
  try {
    const { data } = await getPairs()
    pairs.value = data
  } catch (e: any) { ElMessage.error('加载失败: ' + e.message) }
  loading.value = false
}

async function handleValidateAccount() {
  if (!form.value.account_username || !form.value.account_password) {
    ElMessage.warning('请填写主账号的用户名和密码')
    return
  }
  validatingAccount.value = true
  validatedAccount.value = false
  try {
    const { data } = await validateAccountApi({ username: form.value.account_username, password: form.value.account_password })
    if (data.success) {
      form.value.account_sid = data.sid
      form.value.account_username = data.display_name
      validatedAccount.value = true
      ElMessage.success(data.message)
    } else {
      ElMessage.error(data.message)
    }
  } catch (e: any) { ElMessage.error('验证失败：' + e.message) }
  validatingAccount.value = false
}

async function handleValidateApprover() {
  if (!form.value.approver_username || !form.value.approver_password) {
    ElMessage.warning('请填写审批人的用户名和密码')
    return
  }
  validatingApprover.value = true
  validatedApprover.value = false
  try {
    const { data } = await validateAccountApi({ username: form.value.approver_username, password: form.value.approver_password })
    if (data.success) {
      form.value.approver_sid = data.sid
      form.value.approver_username = data.display_name
      validatedApprover.value = true
      ElMessage.success(data.message)
    } else {
      ElMessage.error(data.message)
    }
  } catch (e: any) { ElMessage.error('验证失败：' + e.message) }
  validatingApprover.value = false
}

async function handleAdd() {
  if (!validatedAccount.value || !validatedApprover.value) {
    ElMessage.warning('请先验证两个账号')
    return
  }
  try {
    await addPair({
      account_sid: form.value.account_sid,
      approver_sid: form.value.approver_sid,
      account_username: form.value.account_username,
      approver_username: form.value.approver_username,
    })
    ElMessage.success('主账号与审批人关联已建立')
    dialogVisible.value = false
    resetForm()
    load()
  } catch (e: any) { ElMessage.error('添加失败：' + e.message) }
}

function resetForm() {
  form.value = { 
    account_username: '', 
    account_password: '', 
    account_sid: '',
    approver_username: '', 
    approver_password: '', 
    approver_sid: '' 
  }
  validatedAccount.value = false
  validatedApprover.value = false
}

async function handleDelete(row: DualPair) {
  await ElMessageBox.confirm(`确定删除主账号 "${row.account_username}" 与审批人 "${row.approver_username}" 的关联关系？`, '确认删除', { type: 'warning' })
  try {
    await deletePair(row.id)
    ElMessage.success('已删除')
    load()
  } catch (e: any) { ElMessage.error('删除失败：' + e.message) }
}

onMounted(load)
</script>

<template>
  <div class="page">
    <div class="page-toolbar">
      <span class="page-title">主账号与审批人配置</span>
      <span class="page-count">{{ pairs.length }}条</span>
      <div class="toolbar-actions">
        <el-button size="small" @click="load">刷新</el-button>
        <el-button size="small" type="primary" @click="dialogVisible = true">+ 新增主账号与审批人</el-button>
      </div>
    </div>
  
    <div class="table-wrap">
      <el-table :data="pairs" v-loading="loading" size="small" border stripe height="100%">
        <el-table-column prop="account_username" label="主账号" min-width="120" />
        <el-table-column prop="approver_username" label="审批人" min-width="120" />
        <el-table-column prop="account_sid" label="主账号 SID" min-width="160" show-overflow-tooltip />
        <el-table-column prop="approver_sid" label="审批人 SID" min-width="160" show-overflow-tooltip />
        <el-table-column label="状态" width="64" align="center">
          <template #default="{ row }">
            <el-tag :type="row.enabled ? 'success' : 'info'" size="small" effect="plain">
              {{ row.enabled ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="60" align="center">
          <template #default="{ row }">
            <el-button type="danger" size="small" link @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>
  
    <el-dialog v-model="dialogVisible" title="新增主账号与审批人" width="460px" :close-on-click-modal="false" @closed="resetForm">
      <el-form label-width="80px" size="small">
        <el-divider content-position="left">主账号（实际登录者）</el-divider>
        <el-form-item label="用户名">
          <el-input 
            v-model="form.account_username" 
            placeholder="alice 或 DOMAIN\alice 或 alice@domain.com" 
            :disabled="validatedAccount"
            @keydown.enter="handleValidateAccount"
          />
        </el-form-item>
        <el-form-item label="密码">
          <el-input 
            v-model="form.account_password" 
            type="password" 
            show-password 
            placeholder="域账号密码" 
            :disabled="validatedAccount"
            @keydown.enter="handleValidateAccount"
          />
        </el-form-item>
        <el-form-item>
          <el-button 
            v-if="!validatedAccount" 
            type="primary" 
            size="small" 
            :loading="validatingAccount" 
            @click="handleValidateAccount"
          >
            验证主账号
          </el-button>
          <el-tag v-else type="success" size="small">
            已验证：{{ form.account_sid }}
          </el-tag>
        </el-form-item>
  
        <el-divider content-position="left">审批人（审核批准者）</el-divider>
        <el-form-item label="用户名">
          <el-input 
            v-model="form.approver_username" 
            placeholder="bob 或 DOMAIN\bob 或 bob@domain.com" 
            :disabled="validatedApprover"
            @keydown.enter="handleValidateApprover"
          />
        </el-form-item>
        <el-form-item label="密码">
          <el-input 
            v-model="form.approver_password" 
            type="password" 
            show-password 
            placeholder="域账号密码" 
            :disabled="validatedApprover"
            @keydown.enter="handleValidateApprover"
          />
        </el-form-item>
        <el-form-item>
          <el-button 
            v-if="!validatedApprover" 
            type="primary" 
            size="small" 
            :loading="validatingApprover" 
            @click="handleValidateApprover"
          >
            验证审批人
          </el-button>
          <el-tag v-else type="success" size="small">
            已验证：{{ form.approver_sid }}
          </el-tag>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="dialogVisible = false">取消</el-button>
        <el-button 
          size="small" 
          type="primary" 
          :disabled="!validatedAccount || !validatedApprover" 
          @click="handleAdd"
        >
          确认添加
        </el-button>
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
.table-wrap { flex: 1; border: 1px solid #e2e8f0; border-radius: 0 0 6px 6px; overflow: hidden; background: #fff; }
</style>
