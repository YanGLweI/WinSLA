<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  getStatus, startService, stopService, restartService,
  getServiceConfig, setServiceConfig, getComputerInfo, getDailyStats,
  type ServiceStatus, type ServiceConfig
} from '../api'
import * as echarts from 'echarts'

const status = ref<ServiceStatus | null>(null)
const loading = ref(true)
const svcActionLoading = ref(false)
const autoStart = ref(false)
const dailyStats = ref([])
const chartRef1 = ref<HTMLElement | null>(null)
const chartRef2 = ref<HTMLElement | null>(null)
let chartsInstance: any[] = []
const computerInfo = ref({ hostname: '-', domain: '-' })

async function load() {
  loading.value = true
  try {
    const { data } = await getStatus()
    status.value = data
  } catch { status.value = null }
  
  // Load computer info
  try {
    const { data } = await getComputerInfo()
    computerInfo.value = data || { hostname: '-', domain: '-' }
  } catch (e) {
    console.error('Failed to load computer info:', e)
    computerInfo.value = { hostname: '-', domain: '-' }
  }
  
  loading.value = false
  updateCharts()
}

async function loadDailyStats() {
  try {
    const { data } = await getDailyStats()
    dailyStats.value = data || []
    updateCharts()
  } catch (e) {
    console.error('Failed to load daily stats:', e)
    dailyStats.value = []
    updateCharts()
  }
}

async function loadConfig() {
  try {
    const { data } = await getServiceConfig()
    autoStart.value = data.auto_start
  } catch { /* ignore */ }
}

// Refresh both status and daily stats (used by refresh button and auto-refresh timer)
function refreshAll() {
  load()
  loadDailyStats()
}

async function doStart() {
  svcActionLoading.value = true
  try {
    const { data } = await startService()
    if (data.success) ElMessage.success(data.message || '服务已启动')
    else ElMessage.error(data.message || '启动失败')
  } catch (e: any) { ElMessage.error('启动失败: ' + (e.message || '')) }
  svcActionLoading.value = false
  setTimeout(load, 1500)
}

async function doStop() {
  try {
    await ElMessageBox.confirm('确认停止 WinSLA 服务？停止后双账号认证将不可用。', '停止服务', { type: 'warning' })
  } catch { return }
  svcActionLoading.value = true
  try {
    const { data } = await stopService()
    if (data.success) ElMessage.success(data.message || '服务已停止')
    else ElMessage.error(data.message || '停止失败')
  } catch (e: any) { ElMessage.error('停止失败: ' + (e.message || '')) }
  svcActionLoading.value = false
  setTimeout(load, 1500)
}

async function doRestart() {
  svcActionLoading.value = true
  try {
    const { data } = await restartService()
    if (data.success) ElMessage.success('服务已重启')
    else ElMessage.error(data.message || '重启失败')
  } catch (e: any) { ElMessage.error('重启失败: ' + (e.message || '')) }
  svcActionLoading.value = false
  setTimeout(load, 1500)
}

async function toggleAutoStart(val: boolean) {
  try {
    const { data } = await setServiceConfig({ auto_start: val })
    if (data.success) ElMessage.success(data.message)
    else { ElMessage.error(data.message); autoStart.value = !val }
  } catch {
    ElMessage.error('设置失败')
    autoStart.value = !val
  }
}

function initCharts() {
  if (!chartRef1.value || !chartRef2.value) return

  const chart1 = echarts.init(chartRef1.value)
  const chart2 = echarts.init(chartRef2.value)
  chartsInstance = [chart1, chart2]

  updateCharts()
}

function updateCharts() {
  if (chartsInstance.length < 2 || !chartsInstance[0] || !chartsInstance[1]) {
    console.warn('[updateCharts] Charts not initialized')
    return
  }
  const [chart1, chart2] = chartsInstance

  // 柱状图：近 7 天登录次数
  const barOption = {
    title: {
      text: '近 7 天登录趋势',
      left: 'center',
      top: 10,  // 增加上边距
      textStyle: { fontSize: 13, fontWeight: 'normal' }
    },
    tooltip: {
      trigger: 'axis',
      formatter: (params: any) => {
        const p = params[0]
        return `${p.axisValue}<br/>登录次数：${p.data}`
      }
    },
    grid: {
      top: '20%',     // 减小上边距让图表更紧凑
      bottom: '18%',  // 增加下边距确保日期显示
      left: '8%',
      right: '8%'
    },
    xAxis: {
      type: 'category',
      data: dailyStats.value.map((d: any) => {
        // 只显示月 - 日格式，不显示年份
        const date = new Date(d.date)
        return `${date.getMonth() + 1}/${date.getDate()}`
      }),
      axisLabel: {
        fontSize: 10,
        interval: 0,
        rotate: 0  // 固定不旋转，始终横向显示
      }
    },
    yAxis: {
      type: 'value'
    },
    series: [{
      data: dailyStats.value.map((d: any) => d.total),
      type: 'bar',
      itemStyle: { color: '#409eff' },
      showBackground: true,
      backgroundStyle: { color: 'rgba(64, 158, 255, 0.1)' },
      barWidth: '60%'
    }]
  }

  // 饼图：登录状态占比
  const successCount = status.value?.successful_auths || 0
  const failedCount = status.value?.failed_auths || 0
  const pieData = [
    { value: successCount, name: '成功' },
    { value: failedCount, name: '失败' }
  ]

  const pieOption = {
    title: {
      text: '登录状态趋势',
      left: 'center',
      top: 5,
      textStyle: { fontSize: 12, fontWeight: 'normal', color: '#475569' }
    },
    tooltip: {
      trigger: 'item',
      formatter: '{b}: {c} ({d}%)'
    },
    legend: {
      orient: 'horizontal',
      bottom: '0'
    },
    series: [{
      name: '登录状态',
      type: 'pie',
      radius: ['35%', '65%'],  // 细环：内径 35%，外径 65%
      center: ['50%', '50%'],
      avoidLabelOverlap: false,
      label: {
        show: false  // 不显示外部标签
      },
      labelLine: {
        show: false  // 不显示标签引导线
      },
      data: pieData.filter((d: any) => d.value > 0),
      color: ['#5dade2', '#ec7063']  // 柔和色系：浅蓝 + 浅红
    }]
  }

  chart1.setOption(barOption)
  chart2.setOption(pieOption)
}

function resizeCharts() {
  chartsInstance.forEach(chart => chart && chart.resize())
}

onMounted(async () => { 
  await nextTick()
  initCharts()
  load()
  loadConfig()
  loadDailyStats()
  window.addEventListener('resize', resizeCharts)
})

onUnmounted(() => {
  window.removeEventListener('resize', resizeCharts)
  chartsInstance.forEach((chart: any) => chart && chart.dispose())
})
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
          <div class="svc-controls">
            <el-button size="small" type="success" :loading="svcActionLoading" @click="doStart" :disabled="status?.running">启动</el-button>
            <el-button size="small" type="danger" :loading="svcActionLoading" @click="doStop" :disabled="!status?.running">停止</el-button>
            <el-button size="small" type="warning" :loading="svcActionLoading" @click="doRestart">重启</el-button>
          </div>
          <div class="autostart-row">
            <span class="autostart-label">开机自动启动</span>
            <el-switch v-model="autoStart" size="small" @change="toggleAutoStart" />
          </div>
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
          <div class="info-row"><span class="info-key">计算机名</span><span class="info-val">{{ computerInfo.hostname }}</span></div>
          <div class="info-row"><span class="info-key">域名</span><span class="info-val">{{ computerInfo.domain }}</span></div>
        </div>
      </section>
    </div>

    <!-- Charts row -->
    <section class="panel chart-panel">
      <div class="panel-row chart-row">
        <div class="chart-container" ref="chartRef1"></div>
        <div class="chart-container" ref="chartRef2"></div>
      </div>
    </section>

    <section class="panel">
      <div class="panel-header">快速操作</div>
      <div class="panel-body actions-row">
        <el-button size="small" type="primary" @click="$router.push('/pairs')">管理配对规则</el-button>
        <el-button size="small" type="warning" @click="$router.push('/emergency')">应急账号</el-button>
        <el-button size="small" @click="$router.push('/audit')">审计日志</el-button>
        <el-button size="small" @click="refreshAll">刷新状态</el-button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.dashboard { display: flex; flex-direction: column; gap: 12px; }
.panel-row { display: grid; grid-template-columns: 1fr 1fr 1.2fr; gap: 12px; }
.chart-row { display: grid; grid-template-columns: 2fr 1fr; gap: 12px; }
.panel {
  background: #fff;
  border: 1px solid #e2e8f0;
  border-radius: 6px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.panel-header {
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 600;
  color: #475569;
  background: #f8fafc;
  border-bottom: 1px solid #e2e8f0;
}
.panel-body { padding: 12px; flex: 1; }

/* Charts */
.chart-container {
  height: 240px;
  width: 100%;
}

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
.svc-controls { display: flex; gap: 6px; margin-top: 4px; }
.autostart-row { display: flex; align-items: center; gap: 8px; margin-top: 8px; }
.autostart-label { font-size: 12px; color: #64748b; }

/* Stats */
.stats-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; text-align: center; align-items: stretch; height: 100%; }
.stat-cell {
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 2px;
  height: 100%;
}
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
