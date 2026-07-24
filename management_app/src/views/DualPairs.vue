<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getPairs, addPair, deletePair, type DualPair } from '../api'

const pairs = ref<DualPair[]>([])
const loading = ref(false)
const dialogVisible = ref(false)
const form = ref({ user_a_name: '', user_b_name: '', user_a_sid: '', user_b_sid: '' })

async function load() {
  loading.value = true
  try {
    const { data } = await getPairs()
    pairs.value = data
  } catch (e: any) { ElMessage.error('加载失败: ' + e.message) }
  loading.value = false
}

async function handleAdd() {
  if (!form.value.user_a_name || !form.value.user_b_name) {
    ElMessage.warning('请填写用户名')
    return
  }
  try {
    await addPair(form.value)
    ElMessage.success('添加成功')
    dialogVisible.value = false
    form.value = { user_a_name: '', user_b_name: '', user_a_sid: '', user_b_sid: '' }
    load()
  } catch (e: any) { ElMessage.error('添加失败: ' + e.message) }
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
  <el-card>
    <template #header>
      <div style="display: flex; justify-content: space-between; align-items: center">
        <span>双账号配对规则</span>
        <el-button type="primary" size="small" @click="dialogVisible = true">
          <el-icon><Plus /></el-icon> 新增配对
        </el-button>
      </div>
    </template>

    <el-table :data="pairs" v-loading="loading" stripe border style="width: 100%">
      <el-table-column prop="user_a_name" label="用户 A" min-width="120" />
      <el-table-column prop="user_b_name" label="用户 B" min-width="120" />
      <el-table-column prop="user_a_sid" label="SID A" min-width="180" show-overflow-tooltip />
      <el-table-column prop="user_b_sid" label="SID B" min-width="180" show-overflow-tooltip />
      <el-table-column label="状态" width="80" align="center">
        <template #default="{ row }">
          <el-tag :type="row.enabled ? 'success' : 'info'" size="small">
            {{ row.enabled ? '启用' : '禁用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="180" />
      <el-table-column label="操作" width="80" align="center">
        <template #default="{ row }">
          <el-button type="danger" size="small" link @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-empty v-if="!loading && pairs.length === 0" description="暂无配对规则" />
  </el-card>

  <el-dialog v-model="dialogVisible" title="新增配对规则" width="480px">
    <el-form :model="form" label-width="90px">
      <el-form-item label="用户 A">
        <el-input v-model="form.user_a_name" placeholder="域账号名 (如 DOMAIN\alice)" />
      </el-form-item>
      <el-form-item label="用户 A SID">
        <el-input v-model="form.user_a_sid" placeholder="可选，如 S-1-5-21-..." />
      </el-form-item>
      <el-form-item label="用户 B">
        <el-input v-model="form.user_b_name" placeholder="域账号名 (如 DOMAIN\bob)" />
      </el-form-item>
      <el-form-item label="用户 B SID">
        <el-input v-model="form.user_b_sid" placeholder="可选，如 S-1-5-21-..." />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="dialogVisible = false">取消</el-button>
      <el-button type="primary" @click="handleAdd">确认添加</el-button>
    </template>
  </el-dialog>
</template>
