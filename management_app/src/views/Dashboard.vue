<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { getStatus, type ServiceStatus } from '../api'

const status = ref<ServiceStatus | null>(null)
const loading = ref(true)

async function load() {
  loading.value = true
  try {
    const { data } = await getStatus()
    status.value = data
  } catch { status.value = null }
  loading.value = false
}

onMounted(load)
</script>

<template>
  <div v-loading="loading">
    <el-row :gutter="16">
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>服务状态</template>
          <div style="text-align: center">
            <el-tag :type="status?.running ? 'success' : 'danger'" size="large" effect="dark">
              {{ status?.running ? '运行中' : '已停止' }}
            </el-tag>
            <p style="margin-top: 12px; color: #909399; font-size: 13px">
              版本: {{ status?.version || '-' }}
            </p>
          </div>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>认证统计</template>
          <el-descriptions :column="1" border size="small">
            <el-descriptions-item label="连接数">{{ status?.connections_accepted ?? 0 }}</el-descriptions-item>
            <el-descriptions-item label="认证成功">{{ status?.successful_auths ?? 0 }}</el-descriptions-item>
            <el-descriptions-item label="认证失败">{{ status?.failed_auths ?? 0 }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
      <el-col :span="8">
        <el-card shadow="hover">
          <template #header>系统信息</template>
          <el-descriptions :column="1" border size="small">
            <el-descriptions-item label="管道">\\.\pipe\winsla-auth-pipe</el-descriptions-item>
            <el-descriptions-item label="服务名">WinSLA Service</el-descriptions-item>
            <el-descriptions-item label="数据库">winsla.db (SQLite)</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>
    </el-row>

    <el-card style="margin-top: 16px">
      <template #header>快速操作</template>
      <el-space wrap>
        <el-button type="primary" @click="$router.push('/pairs')">
          <el-icon><User /></el-icon> 管理配对规则
        </el-button>
        <el-button type="warning" @click="$router.push('/emergency')">
          <el-icon><Warning /></el-icon> 应急账号
        </el-button>
        <el-button @click="$router.push('/audit')">
          <el-icon><Document /></el-icon> 查看审计日志
        </el-button>
        <el-button @click="load">
          <el-icon><Refresh /></el-icon> 刷新状态
        </el-button>
      </el-space>
    </el-card>
  </div>
</template>
