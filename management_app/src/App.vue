<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed } from 'vue'

const route = useRoute()
const title = computed(() => (route.meta.title as string) || 'WinSLA')
</script>

<template>
  <div class="app-shell">
    <!-- Top toolbar -->
    <header class="toolbar">
      <div class="toolbar-brand">
        <span class="brand-icon">W</span>
        <span class="brand-text">WinSLA</span>
      </div>
      <nav class="toolbar-tabs">
        <router-link to="/dashboard" class="tab" active-class="tab-active">仪表盘</router-link>
        <router-link to="/pairs" class="tab" active-class="tab-active">配对规则</router-link>
        <router-link to="/emergency" class="tab" active-class="tab-active">应急账号</router-link>
        <router-link to="/audit" class="tab" active-class="tab-active">审计日志</router-link>
        <router-link to="/settings" class="tab" active-class="tab-active">策略配置</router-link>
      </nav>
    </header>

    <!-- Content area -->
    <main class="content">
      <router-view />
    </main>

    <!-- Status bar -->
    <footer class="statusbar">
      <span class="status-item">{{ title }}</span>
      <span class="status-sep">|</span>
      <span class="status-item">管道: \\.\pipe\winsla-auth-pipe</span>
      <span class="status-right">WinSLA v0.1.0</span>
    </footer>
  </div>
</template>

<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body, #app { height: 100%; overflow: hidden; }
body {
  font-family: "Segoe UI", -apple-system, sans-serif;
  font-size: 13px;
  color: #1a1a2e;
  background: #f0f2f5;
}

.app-shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* Toolbar */
.toolbar {
  display: flex;
  align-items: center;
  height: 40px;
  background: #1b2838;
  padding: 0 12px;
  gap: 16px;
  flex-shrink: 0;
  -webkit-app-region: drag;
}
.toolbar-brand {
  display: flex;
  align-items: center;
  gap: 6px;
}
.brand-icon {
  width: 20px;
  height: 20px;
  background: #409eff;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 11px;
  font-weight: 700;
}
.brand-text {
  color: #e0e6ed;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: 0.5px;
}
.toolbar-tabs {
  display: flex;
  gap: 2px;
  -webkit-app-region: no-drag;
}
.tab {
  padding: 6px 14px;
  color: #8899aa;
  text-decoration: none;
  font-size: 12.5px;
  border-radius: 4px;
  transition: all 0.15s;
}
.tab:hover {
  color: #cfd8e3;
  background: rgba(255,255,255,0.06);
}
.tab-active {
  color: #fff;
  background: rgba(64,158,255,0.2);
}

/* Content */
.content {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
}

/* Status bar */
.statusbar {
  display: flex;
  align-items: center;
  height: 24px;
  background: #e8ecf0;
  border-top: 1px solid #d0d7de;
  padding: 0 10px;
  font-size: 11px;
  color: #5a6a7a;
  flex-shrink: 0;
  gap: 8px;
}
.status-sep { color: #c0c8d0; }
.status-right { margin-left: auto; }
</style>
