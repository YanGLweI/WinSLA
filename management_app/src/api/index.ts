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
  user_a_sid: string
  user_b_sid: string
  user_a_name: string
  user_b_name: string
  enabled: boolean
  created_at: string
  updated_at: string
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
  user_a_sid: string
  user_b_sid: string
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

export default api
