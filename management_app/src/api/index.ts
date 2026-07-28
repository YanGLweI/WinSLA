import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  timeout: 10000,
})

export interface ServiceStatus {
  running: boolean
  version: string
  connections_accepted: number
  successful_auths: number
  failed_auths: number
}

export interface DualPair {
  id: string
  account_sid: string        // 主账号 SID
  approver_sid: string       // 审批人 SID
  account_username: string   // 主账号用户名
  approver_username: string  // 审批人用户名
  enabled: boolean
  created_at: string
  updated_at: string
}

export interface AddPairResponse {
  pair: DualPair
  auto_disabled_default_tile: boolean   // 是否自动禁用了默认 Tile
  has_emergency_accounts: boolean       // 是否已有应急账号
  should_configure_emergency: boolean   // 是否应提示配置应急账号
}

export interface EmergencyAccount {
  id: string
  sid: string
  username: string
  reason: string
  approved_by: string
  activated_at: string
  expires_at: string | null
}

export interface AuditEntry {
  id: number
  timestamp: string
  account_sid: string
  approver_sid: string
  result: string
  error_message: string | null
  client_hostname: string | null
}

export interface PolicyConfig {
  max_retry_count: number
  auth_timeout_secs: number
  allow_emergency_override: boolean
  emergency_requires_reason: boolean
  offline_cache_enabled: boolean
  lockout_duration_minutes: number
  default_tile_enabled: boolean  // 是否启用 Windows 默认登录 Tile（默认 false = WinSLA 独占）
}

export interface ServiceActionResult {
  success: boolean
  message: string
}

export interface ServiceConfig {
  auto_start: boolean
}

export interface ValidateAccountResult {
  success: boolean
  sid: string
  display_name: string
  message: string
}

export const getStatus = () => api.get<ServiceStatus>('/status')
export const getPairs = () => api.get<DualPair[]>('/pairs')
export const addPair = (data: Partial<DualPair>) => api.post('/pairs', data)
export const deletePair = (id: string) => api.delete(`/pairs/${id}`)
export const getEmergency = () => api.get<EmergencyAccount[]>('/emergency')
export const addEmergency = (data: Partial<EmergencyAccount>) => api.post('/emergency', data)
export const deleteEmergency = (id: string) => api.delete(`/emergency/${id}`)
export const getAudit = (limit = 50) => api.get<AuditEntry[]>('/audit', { params: { limit } })
export const getPolicy = () => api.get<PolicyConfig>('/policy')
export const updatePolicy = (data: PolicyConfig) => api.put('/policy', data)
export const startService = () => api.post<ServiceActionResult>('/service/start')
export const stopService = () => api.post<ServiceActionResult>('/service/stop')
export const restartService = () => api.post<ServiceActionResult>('/service/restart')
export const getServiceConfig = () => api.get<ServiceConfig>('/service/config')
export const setServiceConfig = (data: ServiceConfig) => api.put<ServiceActionResult>('/service/config', data)
export const validateAccount = (data: { username: string; password: string }) =>
  api.post<ValidateAccountResult>('/validate-account', data, { timeout: 35000 })

export default api
