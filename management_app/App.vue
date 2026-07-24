<script setup lang="ts">
import { ref, onMounted } from 'vue'
import ElementPlus from 'element-plus'
import zhCn from 'element-plus/es/locale/lang/zh-cn'
import 'element-plus/dist/index.css'

const serviceRunning = ref(false)
const errorMessage = ref('')

async function checkServiceStatus() {
  try {
    // In production, this calls the Tauri command
    const status = await window.__TAURI_CORE__tauri.invoke('get_service_status')
    serviceRunning.value = status.running
  } catch (error) {
    errorMessage.value = `Failed to check service status: ${error}`
    console.error(error)
  }
}

async function startService() {
  try {
    await window.__TAURI_CORE__tauri.invoke('start_service')
    serviceRunning.value = true
    errorMessage.value = ''
  } catch (error) {
    errorMessage.value = `Failed to start service: ${error}`
    console.error(error)
  }
}

async function stopService() {
  try {
    await window.__TAURI_CORE__tauri.invoke('stop_service')
    serviceRunning.value = false
    errorMessage.value = ''
  } catch (error) {
    errorMessage.value = `Failed to stop service: ${error}`
    console.error(error)
  }
}

onMounted(() => {
  checkServiceStatus()
})
</script>

<template>
  <div id="app" style="padding: 20px; font-family: Arial, sans-serif;">
    <h1 style="color: #409EFF;">WinSLA Dual-Account Authentication</h1>
    
    <el-card style="margin-bottom: 20px;">
      <template #header>
        <span>Service Status</span>
      </template>
      <div style="text-align: center;">
        <el-tag :type="serviceRunning ? 'success' : 'danger'" size="large">
          {{ serviceRunning ? 'Running' : 'Stopped' }}
        </el-tag>
      </div>
      <div style="margin-top: 20px; text-align: center;">
        <el-button 
          type="primary" 
          @click="startService" 
          :disabled="serviceRunning"
          style="margin-right: 10px;"
        >
          Start Service
        </el-button>
        <el-button 
          type="danger" 
          @click="stopService" 
          :disabled="!serviceRunning"
        >
          Stop Service
        </el-button>
      </div>
    </el-card>

    <el-card v-if="errorMessage">
      <template #header>
        <span>Error</span>
      </template>
      <p style="color: red;">{{ errorMessage }}</p>
    </el-card>
  </div>
</template>

<style scoped>
#app {
  max-width: 800px;
  margin: 0 auto;
}
</style>
