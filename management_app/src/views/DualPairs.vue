<script setup lang="ts">
import { ref, onMounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { 
  validateAccount as validateAccountApi,
  getAccounts,
  createAccount,
  addApprover,
  removeApprover,
  toggleAccountEnabled,
  deleteAccountPair,
  type DualPairV2,
  type ApproverInfo,
  updatePolicy
} from '../api'

const accounts = ref<DualPairV2[]>([])
const loading = ref(false)
const addAccountDialog = ref(false)
const addApproverDialog = ref(false)
const currentAccount = ref<DualPairV2 | null>(null)

const form = ref({
  account_username: '',
  account_password: '',
  account_sid: '',
  validatedAccount: false,
  
  approver_username: '',
  approver_password: '',
  approver_sid: '',
  validatedApprover: false
})

async function load() {
  loading.value = true
  try {
    const { data } = await getAccounts()
    // 关键修复：将 enabled 字段从数字 (0/1) 显式转换为布尔值 (true/false)
    // 避免 el-switch 组件初始化时因类型不匹配自动触发 change 事件
    accounts.value = data.map(account => {
      let approversArray = []
      try {
        if (typeof account.approvers === 'string') {
          approversArray = JSON.parse(account.approvers)
        } else if (Array.isArray(account.approvers)) {
          approversArray = account.approvers
        }
      } catch (e) {
        console.error('Failed to parse approvers JSON:', e)
        approversArray = []
      }
      
      return {
        ...account,
        enabled: Boolean(account.enabled),  // ✅ 强制转换为布尔值
        approvers: approversArray.filter(a => a.username && a.username.trim().length > 0)
      }
    })
    
    // ✅ 首次加载完成后清空标志，防止后续操作误报成功消息
    if (isInitialLoad) {
      isInitialLoad = false
    }
  } catch (e: any) {
    ElMessage.error('加载失败：' + e.message)
  }
  loading.value = false
}

// ✅ 修改 load 为 loadAccounts 避免 with 自动调用
let isInitialLoad = true

async function handleValidateAccount() {
  if (!form.value.account_username || !form.value.account_password) {
    ElMessage.warning('请填写主账号的用户名和密码')
    return
  }
  
  try {
    validatingAccount.value = true
    const { data } = await validateAccountApi({ 
      username: form.value.account_username, 
      password: form.value.account_password 
    })
    
    if (data.success) {
      form.value.account_sid = data.sid
      form.value.account_username = data.display_name
      form.value.validatedAccount = true
      ElMessage.success(data.message)
    } else {
      ElMessage.error(data.message)
    }
  } catch (e: any) {
    ElMessage.error('验证失败：' + e.message)
  } finally {
    validatingAccount.value = false
  }
}

async function handleValidateApprover() {
  // ✅ 关键修复：直接从 DOM 获取值作为后备方案
  const usernameInput = document.querySelector('#approver-username-input') as HTMLInputElement
  const passwordInput = Array.from(document.querySelectorAll('input[type="password"]')).find(el => el.parentElement?.parentElement?.querySelector('[class*="审批信息"]')) as HTMLInputElement
  
  const enteredUsername = usernameInput?.value || form.value.approver_username
  const enteredPassword = passwordInput?.value || form.value.approver_password
  
  if (!enteredUsername || !enteredPassword) {
    ElMessage.warning('请填写审批人的用户名和密码')
    return
  }
  
  try {
    validatingApprover.value = true
    const { data } = await validateAccountApi({ 
      username: enteredUsername, 
      password: enteredPassword 
    })
    
    if (data.success) {
      form.value.approver_sid = data.sid
      form.value.approver_username = data.display_name
      form.value.validatedApprover = true
      ElMessage.success(data.message)
    } else {
      ElMessage.error(data.message)
    }
  } catch (e: any) {
    ElMessage.error('验证失败：' + e.message)
  } finally {
    validatingApprover.value = false
  }
}

async function handleAddAccount() {
  if (!form.value.validatedAccount || !form.value.validatedApprover) {
    ElMessage.warning('必须同时验证主账号和审批人才能创建')
    return
  }
  
  // ✅ Bug Fix: 校验主账号和审批人不能是同一个账号
  if (form.value.account_sid === form.value.approver_sid) {
    ElMessage.error('主账号和审批人必须是不同的账号')
    return
  }
  
  try {
    // ✅ Bug Fix: 一次性创建完整的配对规则 (主账号 + 第一个审批人)
    await createAccount(
      form.value.account_sid,
      form.value.account_username
    )
    
    // 立即添加第一个审批人
    await addApprover(
      form.value.account_sid,
      form.value.approver_sid,
      form.value.approver_username
    )
    
    ElMessage.success('主账号与审批人已成功关联')
    addAccountDialog.value = false
    load()
    resetAccountForm()
  } catch (e: any) {
    if (e.response?.status === 409) {
      ElMessage.error('该主账号已存在')
    } else {
      ElMessage.error('创建失败：' + e.message)
    }
  }
}

// ✅ 防止 onMounted 自动触发 success message
let isLoading = false

// ✅ Bug Fix: 任何时候都应该可以添加审批人，不检查 enabled 状态
async function handleOpenAddApprover(row: DualPairV2) {
  // ✅ 移除了 if (!row.enabled) 检查，允许在任何状态下添加审批人
  
  // ⚠️ 关键修复：先清空之前的验证状态，再设置当前账号
  form.value.approver_username = ''
  form.value.approver_password = ''
  form.value.approver_sid = ''
  form.value.validatedApprover = false
  
  // ✅ 使用对象解构确保 Vue 能正确追踪变化
  currentAccount.value = { ...row }
  
  addApproverDialog.value = true
  
  // 等待 DOM 更新后聚焦
  await nextTick()
  setTimeout(() => {
    const inputEl = document.querySelector('#approver-username-input')
    inputEl?.focus()
  }, 100)
}

async function handleConfirmAddApprover() {
  if (!currentAccount.value || !form.value.validatedApprover || !form.value.approver_sid) {
    ElMessage.warning('请先验证审批人账号')
    return
  }
  
  // ✅ Bug Fix: 校验主账号和审批人不能是同一个账号
  if (currentAccount.value.account_sid === form.value.approver_sid) {
    ElMessage.error('主账号和审批人必须是不同的账号')
    return
  }
  
  try {
    await addApprover(
      currentAccount.value.account_sid,
      form.value.approver_sid,
      form.value.approver_username
    )
    ElMessage.success('审批人已添加到主账号')
    addApproverDialog.value = false
    load()
  } catch (e: any) {
    console.error('Add approver failed:', e.response?.status, e.response?.data, e.message)
    if (e.response?.status === 409) {
      ElMessage.warning('该审批人已存在')
    } else {
      ElMessage.error('添加失败：' + (e.response?.data?.message || e.message))
    }
  }
}

async function handleRemoveApprover(accountSid: string, approverSid: string) {
  await ElMessageBox.confirm('确定要删除该审批人吗？', '确认删除', {
    type: 'warning'
  })
  
  try {
    await removeApprover(accountSid, approverSid)
    
    // 重新加载数据
    await load()
    
    // ✅ Bug 修复：检查是否还有剩余审批人
    const account = accounts.value.find(a => a.account_sid === accountSid)
    let remainingApprovers = []
    if (account) {
      try {
        if (typeof account.approvers === 'string') {
          remainingApprovers = JSON.parse(account.approvers)
        } else if (Array.isArray(account.approvers)) {
          remainingApprovers = account.approvers
        }
      } catch (e) {
        console.error('Failed to parse approvers:', e)
      }
    }
    
    // 如果没有剩余审批人了，显示提示（后端会自动禁用）
    if (remainingApprovers.length === 0) {
      ElMessage.success(`已移除最后一个审批人，该配对规则已被自动禁用`)
      
      // 检查是否是最后一条完整配对（只有这一个账号且没有审批人）
      const anyCompletePair = accounts.value.some(acc => {
        try {
          let arr = []
          if (typeof acc.approvers === 'string') {
            arr = JSON.parse(acc.approvers)
          } else if (Array.isArray(acc.approvers)) {
            arr = acc.approvers
          }
          return arr.length > 0
        } catch (e) {
          return false
        }
      })
      
      if (!anyCompletePair) {
        // 启用 default tile
        const { getPolicy, updatePolicy } = await import('../api')
        const policyResponse = await getPolicy()
        const policyData = policyResponse.data || {}
        
        await updatePolicy({
          ...policyData,
          default_tile_enabled: true
        })
        
        ElMessage.success('所有配对规则无效，默认登录 Tile 已启用')
      }
    }
  } catch (e: any) {
    ElMessage.error('删除失败：' + e.message)
  }
}

async function handleToggleAccount(row: DualPairV2, newEnabled: boolean) {
  const action = newEnabled ? '启用' : '禁用'
  
  try {
    // ⚠️ 注意：后端期望 i64 而非 boolean，所以这里发送 Number(newEnabled) (0 或 1)
    console.log('Attempting to toggle account:', row.account_sid, 'to:', newEnabled ? 'enabled (true)' : 'disabled (false)')
    const response = await fetch(`/api/accounts/${encodeURIComponent(row.account_sid)}/enable`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ enabled: Number(newEnabled) })  // ✅ 发送整数 0/1
    })
    
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`)
    }
    
    ElMessage.success(`${action}成功`)
    await load()
  } catch (e: any) {
    console.error('Toggle failed:', e)
    ElMessage.error(`操作失败：${e.message || e}`)
    // 恢复 UI 状态
    row.enabled = !newEnabled
  }
}

async function handleDeleteAccount(row: DualPairV2) {
  await ElMessageBox.confirm(
    `确定要删除主账号 "${row.account_username}" 及其所有审批人？此操作不可恢复！`,
    '确认删除',
    { type: 'warning' }
  )
  
  try {
    await deleteAccountPair(row.account_sid)
    ElMessage.success('已删除')
    await load()
    
    // ✅ Bug #4 修复：如果无剩余配对，自动启用默认 tile
    if (accounts.value.length === 0) {
      // 获取当前策略配置
      const { getPolicy, updatePolicy } = await import('../api')
      const policyConfig = await getPolicy()
      
      // 更新策略配置，启用默认 Tile
      await updatePolicy({
        ...policyConfig.data,
        default_tile_enabled: true
      })
      
      // 刷新策略列表以显示最新状态
      window.location.hash = '#/policy'
      setTimeout(() => {
        window.location.hash = '#/pairs'
        ElMessage.success('默认登录 Tile 已启用，请前往策略配置页面验证')
      }, 500)
    }
  } catch (e: any) {
    ElMessage.error('删除失败：' + e.message)
  }
}

// Form state helpers
const validatingAccount = ref(false)
const validatingApprover = ref(false)

function resetAccountForm() {
  form.value = {
    account_username: '',
    account_password: '',
    account_sid: '',
    validatedAccount: false,
    approver_username: '',
    approver_password: '',
    approver_sid: '',
    validatedApprover: false
  }
}

// 重置审批人表单（仅清空审批人信息）
function resetApproverForm() {
  form.value.approver_username = ''
  form.value.approver_password = ''
  form.value.approver_sid = ''
  form.value.validatedApprover = false
}

// 打开主账号对话框时重置表单
function handleOpenAddAccount() {
  resetAccountForm()
  addAccountDialog.value = true
}

// Bug #3: 防止直接关闭审批人对话框，除非配置了至少一个审批人或取消禁用默认 tile
async function handleBeforeCloseApproverDialog() {
  if (!currentAccount.value) {
    return true;
  }
  
  const hasApprovedApproverInDatabase = currentAccount.value.approvers !== '[]';
  
  // 如果账号还没有添加任何审批人到数据库，显示确认提示
  if (!hasApprovedApproverInDatabase) {
    await ElMessageBox.confirm(
      '您当前正在为新创建的主账号添加第一个审批人。\n\n如果现在关闭对话框，该配对规则将暂时没有审批人，但主账号已创建成功。您确定要关闭吗？',
      '添加审批人提示',
      {
        confirmButtonText: '关闭但不添加',
        cancelButtonText: '继续添加审批人',
        type: 'warning'
      }
    )
    // 无论用户选择什么，都允许通过 × 关闭，实际禁用逻辑由 @close 处理
    return true
  }
  
  return true; // 已有审批人在数据库中，可以直接关闭
}

// ✅ 对话框关闭后的回调 - 在这里执行禁用逻辑
async function handleApproverDialogClosed() {
  // 检查是否是新创建的账号 (没有审批人在数据库中)
  if (currentAccount.value && currentAccount.value.approvers === '[]') {
    try {
      const { toggleAccountEnabled } = await import('../api')
      
      // 禁用当前账号
      await toggleAccountEnabled(currentAccount.value.account_sid, false)
      
      ElMessage.success(`已移除最后一个审批人，该配对规则已被自动禁用`)
      
      // 刷新数据以检查是否需要启用 default tile
      await load()
    } catch (e) {
      console.error('Failed to disable account:', e)
      ElMessage.warning('禁用规则失败，请手动在列表中操作')
    }
  }
}

onMounted(load)
</script>

<template>
  <div class="page">
    <div class="page-toolbar">
      <span class="page-title">主账号与审批人配置</span>
      <span class="page-count">{{ accounts.length }}条规则</span>
      <div class="toolbar-actions">
        <el-button size="small" @click="load">刷新</el-button>
        <el-button size="small" type="primary" @click="handleOpenAddAccount">+ 新增主账号</el-button>
      </div>
    </div>
  
    <el-table :data="accounts" v-loading="loading" border stripe height="100%">
      <!-- 主账号信息 -->
      <el-table-column prop="account_username" label="主账号" min-width="120" />
      
      <!-- 审批人群组展示 -->
      <el-table-column label="审批人列表" min-width="250">
        <template #default="{ row }">
          <div v-for="approver in row.approvers" :key="approver.sid || Math.random()" style="margin-bottom: 4px;">
            <!-- 过滤掉无效的空标签 -->
            <el-tag 
              v-if="approver.username && approver.username.length > 0"
              :type="approver.enabled ? 'success' : 'info'" 
              size="small"
              closable
              @close="handleRemoveApprover(row.account_sid, approver.sid)"
            >
              {{ approver.username }}
            </el-tag>
          </div>
          <!-- 如果没有审批人，显示提示文字 -->
          <el-text v-if="!row.approvers || row.approvers.length === 0" type="secondary" style="color: #999; font-size: 12px;">
            暂无审批人
          </el-text>
        </template>
      </el-table-column>
      
      <!-- 主账号 SID -->
      <el-table-column prop="account_sid" label="主账号 SID" min-width="200" show-overflow-tooltip />
      
      <!-- 状态 -->
      <el-table-column label="状态" width="80" align="center">
        <template #default="{ row }">
          <el-switch
            v-model="row.enabled"
            @change="(value) => handleToggleAccount(row, Boolean(value))"
          />
        </template>
      </el-table-column>
      
      <!-- 创建时间 -->
      <el-table-column prop="created_at" label="创建时间" width="170" />
      
      <!-- 操作 -->
      <el-table-column label="操作" width="160" align="center" fixed="right">
        <template #default="{ row }">
          <!-- ✅ Bug Fix: 任何时候都应该可以添加审批人，不检查 enabled 状态 -->
          <el-button 
            size="small" 
            type="primary"
            @click="handleOpenAddApprover(row)"
          >
            + 审批人
          </el-button>
          <el-button 
            size="small" 
            type="danger"
            @click="handleDeleteAccount(row)"
          >
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  
    <!-- 新增主账号对话框 -->
    <el-dialog 
      v-model="addAccountDialog" 
      title="新增主账号与审批人" 
      width="460px" 
      :close-on-click-modal="false"
    >
      <el-form label-width="80px" size="small">
        <el-divider content-position="left">主账号（实际登录者）</el-divider>
        
        <el-form-item label="用户名">
          <el-input 
            v-model="form.account_username" 
            placeholder="alice 或 DOMAIN\alice 或 alice@domain.com"
            @keydown.enter="handleValidateAccount"
            id="account-username-input"
          />
        </el-form-item>
        
        <el-form-item label="密码">
          <el-input 
            v-model="form.account_password" 
            type="password"
            placeholder="请输入主账号密码"
            @keydown.enter="handleValidateAccount"
          />
        </el-form-item>
        
        <el-form-item>
          <el-button type="primary" @click="handleValidateAccount" :loading="validatingAccount">
            验证主账号
          </el-button>
        </el-form-item>
        
        <el-form-item v-if="form.validatedAccount">
          <el-tag type="success">主账号已验证</el-tag>
        </el-form-item>
        
        <el-divider content-position="left">审批人信息（第一个审批人）</el-divider>
        
        <el-form-item label="用户名">
          <el-input 
            v-model="form.approver_username" 
            placeholder="bob 或 DOMAIN\bob 或 bob@domain.com"
            @keydown.enter="handleValidateApprover"
            id="approver-username-input"
          />
        </el-form-item>
        
        <el-form-item label="密码">
          <el-input 
            v-model="form.approver_password" 
            type="password"
            placeholder="请输入审批人密码"
            @keydown.enter="handleValidateApprover"
          />
        </el-form-item>
        
        <el-form-item>
          <el-button type="primary" @click="handleValidateApprover" :loading="validatingApprover">
            验证审批人
          </el-button>
        </el-form-item>
        
        <el-form-item v-if="form.validatedApprover">
          <el-tag type="success">审批人已验证</el-tag>
        </el-form-item>
        
        <el-form-item>
          <el-button type="success" @click="handleAddAccount" :disabled="!form.validatedAccount || !form.validatedApprover">
            创建主账号与审批人
          </el-button>
          <el-button @click="addAccountDialog = false">取消</el-button>
        </el-form-item>
      </el-form>
    </el-dialog>
  
    <!-- 添加审批人对话框 -->
    <el-dialog 
      v-model="addApproverDialog" 
      :title="`为 ${currentAccount?.account_username || '用户'} 添加审批人`" 
      width="460px" 
      :close-on-click-modal="false"
      :before-close="handleBeforeCloseApproverDialog"
      @close="handleApproverDialogClosed"
    >
      <el-form label-width="80px" size="small">
        <el-divider content-position="left">审批人信息</el-divider>
        
        <el-form-item label="用户名">
          <el-input 
            id="approver-username-input"
            v-model="form.approver_username" 
            placeholder="bob 或 DOMAIN\bob 或 bob@domain.com"
            @keydown.enter="handleValidateApprover"
          />
        </el-form-item>
        
        <el-form-item label="密码">
          <el-input 
            v-model="form.approver_password" 
            type="password"
            placeholder="请输入审批人密码"
            @keydown.enter="handleValidateApprover"
          />
        </el-form-item>
        
        <el-form-item>
          <el-button type="primary" @click="handleValidateApprover" :loading="validatingApprover">
            验证审批人
          </el-button>
        </el-form-item>
        
        <el-form-item v-if="form.validatedApprover">
          <el-tag type="success">审批人已验证，可添加到主账号</el-tag>
        </el-form-item>
        
        <el-form-item>
          <el-button 
            type="success" 
            @click="handleConfirmAddApprover" 
            :disabled="!form.validatedApprover"
          >
            添加审批人
          </el-button>
          <el-button @click="addApproverDialog = false">取消</el-button>
        </el-form-item>
      </el-form>
    </el-dialog>
  </div>
</template>

<style scoped>
.page {
  padding: 20px;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.page-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  padding: 12px;
  background: #f5f7fa;
  border-radius: 4px;
}

.page-title {
  font-size: 18px;
  font-weight: bold;
  color: #303133;
}

.page-count {
  font-size: 14px;
  color: #909399;
}

.toolbar-actions {
  display: flex;
  gap: 8px;
}

.table-wrap {
  flex: 1;
  overflow: auto;
}
</style>
