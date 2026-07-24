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
  <div class="dashboard" v-loading="loading">
    <div class="panel-row">
      <!-- Service status panel -->
      <section class="panel">
        <div class="panel-header">服务状态</div>
        <div class="panel-body status-panel">
          <div class="status-indicator" :class="status?.running ? 'online' : 'offline'">
            <span class="dot"></span>
            {{ status?.running ? '运行中' : '已停止' }}
          </div>
          <div class="status-meta">版本 {{ status?.version || '-' }}</div>
        </div>
      </section>

      <!-- Stats panel -->
      <section class="panel">
        <div class="panel-header">认证统计</div>
        <div class="panel-body stats-grid">
          <div class="stat-cell">
            <span class="stat-value">{{ status?.connections_accepted ?? 0 }}</span>
            <span class="stat-label">连接数</span>
          </div>
          <div class="stat-cell">
            <span class="stat-value success">{{ status?.successful_auths ?? 0 }}</span>
            <span class="stat-label">成功</span>
          </div>
          <div class="stat-cell">
            <span class="stat-value danger">{{ status?.failed_auths ?? 0 }}</span>
            <span class="stat-label">失败</span>
          </div>
        </div>
      </section>

      <!-- System info panel -->
      <section class="panel">
        <div class="panel-header">系统信息</div>
        <div class="panel-body info-list">
          <div class="info-row"><span class="info-key">服务名</span><span class="info-val">WinSLA Service</span></div>
          <div class="info-row"><span class="info-key">管道</span><span class="info-val mono">\\.\pipe\winsla-auth-pipe</span></div>
          <div class="info-row"><span class="info-key">数据库</span><span class="info-val">winsla.db (SQLite)</span></div>
        </div>
      </section>
    </div>

    <!-- Quick actions -->
    <section class="panel">
      <div class="panel-header">快速操作</div>
      <div class="panel-body actions-row">
        <el-button size="small" type="primary" @click="$router.push('/pairs')">管理配对规则</el-button>
        <el-button size="small" type="warning" @click="$router.push('/emergency')">应急账号</el-button>
        <el-button size="small" @click="$router.push('/audit')">审计日志</el-button>
        <el-button size="small" @click="load">刷新状态</el-button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.dashboard { display: flex; flex-direction: column; gap: 12px; }
.panel-row { display: grid; grid-template-columns: 1fr 1fr 1.2fr; gap: 12px; }
.panel {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  overflow: hidden;
}
.panel-header {
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 600;
  color: #475569;
  background: #f8fafc;
  border-bottom: 1px solid #e2e8f0;
}
.panel-body { padding: 12px; }

/* Status */
.status-panel { display: flex; flex-direction: column; gap: 8px; align-items: center; padding: 16px 12px; }
.status-indicator {
  display: flex; align-items: center; gap: 8px;
  font-size: 14px; font-weight: 600;
}
.status-indicator .dot {
  width: 10px; height: 10px; border-radius: 50%;
}
.status-indicator.online .dot { background: #22c55e; box-shadow: 0 0 6px #22c55e88; }
.status-indicator.offline .dot { background: #ef4444; box-shadow: 0 0 6px #ef444488; }
.status-meta { font-size: 11px; color: #94a3b8; }

/* Stats */
.stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; text-align: center; }
.stat-cell { display: flex; flex-direction: column; gap: 2px; }
.stat-value { font-size: 20px; font-weight: 700; color: #1e293b; }
.stat-value.success { color: #16a34a; }
.stat-value.danger { color: #dc2626; }
.stat-label { font-size: 11px; color: #94a3b8; }

/* Info */
.info-list { display: flex; flex-direction: column; gap: 6px; }
.info-row { display: flex; justify-content: space-between; align-items: center; }
.info-key { font-size: 12px; color: #64748b; }
.info-val { font-size: 12px; color: #1e293b; }
.info-val.mono { font-family: "Cascadia Code", "Consolas", monospace; font-size: 11px; }

/* Actions */
.actions-row { display: flex; gap: 8px; flex-wrap: wrap; }
</style>
