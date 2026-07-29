<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { getPolicy, updatePolicy, type PolicyConfig, getPairs } from '../api'

const form = ref<PolicyConfig | null>(null)
const loading = ref(false)
const saving = ref(false)
const hasAnyPair = ref(false)

async function load() {
  loading.value = true
  try {
    const [policyRes, pairsRes] = await Promise.all([
      getPolicy(),
      getPairs()
    ])
    form.value = policyRes.data
    hasAnyPair.value = pairsRes.data.length > 0
  } catch { ElMessage.error('加载失败') }
  loading.value = false
}

async function save() {
  if (!form.value) return
  // 安全检查：无配对时禁止禁用默认 Tile
  if (!form.value.default_tile_enabled && !hasAnyPair.value) {
    try {
      await ElMessageBox.confirm(
        '当前未配置任何配对规则，禁用默认登录 Tile 后将只能通过 WinSLA 双控登录。\n\n若 WinSLA 服务异常可能导致无法登录系统，是否确认禁用？',
        '安全风险提示',
        {
          confirmButtonText: '确认禁用',
          cancelButtonText: '保持启用',
          type: 'error'
        }
      )
      // 用户点击“确认禁用”，继续保存流程
    } catch {
      // 用户点击“保持启用”或关闭弹窗：恢复开关状态并放弃保存
      form.value.default_tile_enabled = true
      return
    }
  }
  
  saving.value = true
  try {
    await updatePolicy(form.value)
    ElMessage.success('保存成功')
    // 重新加载以刷新缓存
    await load()
  } catch (e: any) { ElMessage.error('保存失败：' + e.message) }
  saving.value = false
}

onMounted(load)
</script>

<template>
  <div class="page">
    <div class="page-toolbar">
      <span class="page-title">认证策略配置</span>
      <div class="toolbar-actions">
        <el-button size="small" type="primary" :loading="saving" @click="save">保存配置</el-button>
      </div>
    </div>

    <div class="settings-body" v-loading="loading" v-if="form">
      <div class="settings-group">
        <div class="group-title">认证参数</div>
        <div class="setting-row">
          <span class="setting-label">最大重试次数</span>
          <el-input-number v-model="form.max_retry_count" :min="1" :max="10" size="small" />
        </div>
        <div class="setting-row">
          <span class="setting-label">认证超时 (秒)</span>
          <el-input-number v-model="form.auth_timeout_secs" :min="5" :max="120" :step="5" size="small" />
        </div>
        <div class="setting-row">
          <span class="setting-label">锁定时长 (分钟)</span>
          <el-input-number v-model="form.lockout_duration_minutes" :min="1" :max="120" size="small" />
        </div>
        <div class="setting-row">
          <span class="setting-label">默认登录 Tile</span>
          <el-switch v-model="form.default_tile_enabled" size="small" />
        </div>
        <div class="setting-row">
          <span class="setting-label"></span>
          <el-alert title="建议保持关闭状态。开启后会显示 Windows 默认登录方式（单人密码）。" type="warning" :closable="false" />
        </div>
      </div>

      <div class="settings-group">
        <div class="group-title">应急覆盖</div>
        <div class="setting-row">
          <span class="setting-label">允许应急覆盖登录</span>
          <el-switch v-model="form.allow_emergency_override" size="small" />
        </div>
        <div class="setting-row">
          <span class="setting-label">应急登录需填写原因</span>
          <el-switch v-model="form.emergency_requires_reason" size="small" />
        </div>
      </div>

      <div class="settings-group">
        <div class="group-title">离线模式</div>
        <div class="setting-row">
          <span class="setting-label">启用离线缓存</span>
          <el-switch v-model="form.offline_cache_enabled" size="small" />
        </div>
      </div>
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
.toolbar-actions { margin-left: auto; }
.settings-body {
  flex: 1; background: #fff; border: 1px solid #e2e8f0;
  border-radius: 0 0 6px 6px; padding: 16px; overflow-y: auto;
}
.settings-group { margin-bottom: 16px; }
.group-title {
  font-size: 12px; font-weight: 600; color: #475569;
  padding-bottom: 6px; margin-bottom: 8px;
  border-bottom: 1px solid #f1f5f9;
}
.setting-row {
  display: flex; align-items: center; justify-content: space-between;
  padding: 6px 0;
}
.setting-label { font-size: 12.5px; color: #334155; }
</style>
