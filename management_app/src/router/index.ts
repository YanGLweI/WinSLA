import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: '/', redirect: '/dashboard' },
    { path: '/dashboard', name: 'Dashboard', component: () => import('../views/Dashboard.vue'), meta: { title: '仪表盘' } },
    { path: '/pairs', name: 'DualPairs', component: () => import('../views/DualPairs.vue'), meta: { title: '配对规则' } },
    { path: '/emergency', name: 'Emergency', component: () => import('../views/Emergency.vue'), meta: { title: '应急账号' } },
    { path: '/audit', name: 'AuditLog', component: () => import('../views/AuditLog.vue'), meta: { title: '审计日志' } },
    { path: '/settings', name: 'Settings', component: () => import('../views/Settings.vue'), meta: { title: '策略配置' } },
  ],
})

export default router
