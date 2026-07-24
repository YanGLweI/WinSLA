<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getPairs, addPair, deletePair, validateAccount, type DualPair } from '../api'

const pairs = ref<DualPair[]>([])
const loading = ref(false)
const dialogVisible = ref(false)

const form = ref({
  user_a_name: '', user_a_pass: '', user_a_sid: '',
  user_b_name: '', user_b_pass: '', user_b_sid: '',
})
const validatingA = ref(false)
const validatingB = ref(false)
const validatedA = ref(false)
const validatedB = ref(false)

async function load() {
  loading.value = true
  try {
    const { data } = await getPairs()
    pairs.value = data
  } catch (e: any) { ElMessage.error('加载失败: ' + e.message) }
  loading.value = false
}

async function validateA() {
  if (!form.value.user_a_name || !form.value.user_a_pass) {
    ElMessage.warning('请填写用户 A 的用户名和密码')
    return
  }
  validatingA.value = true
  validatedA.value = false
  try {
    const { data } = await validateAccount({ username: form.value.user_a_name, password: form.value.user_a_pass })
    if (data.success) {
      form.value.user_a_sid = data.sid
      form.value.user_a_name = data.display_name
      validatedA.value = true
      ElMessage.success(data.message)
    } else {
      ElMessage.error(data.message)
    }
  } catch (e: any) { ElMessage.error('验证失败: ' + e.message) }
  validatingA.value = false
}

async function validateB() {
  if (!form.value.user_b_name || !form.value.user_b_pass) {
    ElMessage.warning('请填写用户 B 的用户名和密码')
    return
  }
  validatingB.value = true
  validatedB.value = false
  try {
    const { data } = await validateAccount({ username: form.value.user_b_name, password: form.value.user_b_pass })
    if (data.success) {
      form.value.user_b_sid = data.sid
      form.value.user_b_name = data.display_name
      validatedB.value = true
      ElMessage.success(data.message)
    } else {
      ElMessage.error(data.message)
    }
  } catch (e: any) { ElMessage.error('验证失败: ' + e.message) }
  validatingB.value = false
}

async function handleAdd() {
  if (!validatedA.value || !validatedB.value) {
    ElMessage.warning('请先验证两个账号')
    return
  }
  try {
    await addPair({
      user_a_name: form.value.user_a_name,
      user_b_name: form.value.user_b_name,
      user_a_sid: form.value.user_a_sid,
      user_b_sid: form.value.user_b_sid,
    })
    ElMessage.success('配对添加成功')
    dialogVisible.value = false
    resetForm()
    load()
  } catch (e: any) { ElMessage.error('添加失败: ' + e.message) }
}

function resetForm() {
  form.value = { user_a_name: '', user_a_pass: '', user_a_sid: '', user_b_name: '', user_b_pass: '', user_b_sid: '' }
  validatedA.value = false
  validatedB.value = false
}

async function handleDelete(row: DualPair) {
  await ElMessageBox.confirm(`确定删除配对 "${row.user_a_name} <-> ${row.user_b_name}"？`, '确认删除', { type: 'warning' })
  try {
    await deletePair(row.id)
    ElMessage.success('已删除')
    load()
  } catch (e: any) { ElMessage.error('删除失败: ' + e.message) }
}

onMounted(load)
</script>

<template>
  <div class="page">
    <div class="page-toolbar">
      <span class="page-title">双账号配对规则</span>
      <span class="page-count">{{ pairs.length }} 条</span>
      <div class="toolbar-actions">
        <el-button size="small" @click="load">刷新</el-button>
        <el-button size="small" type="primary" @click="dialogVisible = true">+ 新增配对</el-button>
      </div>
    </div>

    <div class="table-wrap">
      <el-table :data="pairs" v-loading="loading" size="small" border stripe height="100%">
        <el-table-column prop="user_a_name" label="用户 A" min-width="120" />
        <el-table-column prop="user_b_name" label="用户 B" min-width="120" />
        <el-table-column prop="user_a_sid" label="SID A" min-width="160" show-overflow-tooltip />
        <el-table-column prop="user_b_sid" label="SID B" min-width="160" show-overflow-tooltip />
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

    <el-dialog v-model="dialogVisible" title="新增配对规则" width="460px" :close-on-click-modal="false" @closed="resetForm">
      <el-form label-width="70px" size="small">
        <el-divider content-position="left">用户 A</el-divider>
        <el-form-item label="用户名">
          <el-input v-model="form.user_a_name" placeholder="DOMAIN\alice 或 alice@domain.com" :disabled="validatedA" />
        </el-form-item>
        <el-form-item label="密码">
          <el-input v-model="form.user_a_pass" type="password" show-password placeholder="域账号密码" :disabled="validatedA" />
        </el-form-item>
        <el-form-item>
          <el-button v-if="!validatedA" type="primary" size="small" :loading="validatingA" @click="validateA">验证账号 A</el-button>
          <el-tag v-else type="success" size="small">已验证: {{ form.user_a_sid }}</el-tag>
        </el-form-item>

        <el-divider content-position="left">用户 B</el-divider>
        <el-form-item label="用户名">
          <el-input v-model="form.user_b_name" placeholder="DOMAIN\bob 或 bob@domain.com" :disabled="validatedB" />
        </el-form-item>
        <el-form-item label="密码">
          <el-input v-model="form.user_b_pass" type="password" show-password placeholder="域账号密码" :disabled="validatedB" />
        </el-form-item>
        <el-form-item>
          <el-button v-if="!validatedB" type="primary" size="small" :loading="validatingB" @click="validateB">验证账号 B</el-button>
          <el-tag v-else type="success" size="small">已验证: {{ form.user_b_sid }}</el-tag>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="dialogVisible = false">取消</el-button>
        <el-button size="small" type="primary" :disabled="!validatedA || !validatedB" @click="handleAdd">确认添加</el-button>
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
