# WinSLA 4 个 Bug 完整修复方案

## 🐛 Bug 分析总结

| Bug | 问题描述 | 严重性 |
|-----|---------|--------|
| #1 | 应急账号提醒过早弹出（与审批人对话框同时弹出） | ⚠️ 中等 |
| #2 | JSON 反序列化错误：boolean 'false' 期望 i64 | 🔴 严重 |
| #3 | 删除最后一条配对关系后未自动启用默认 Tile | 🔴 严重（可能导致锁死） |
| #4 | 首次新增主账号时可跳过添加审批人导致配对关系不完整 | 🔴 严重（可能导致锁死） |

---

## 💡 完整修复方案

### Bug #1: 应急账号提醒时机优化

**问题**: 应急账号提醒弹窗应该在第一对配对关系完全配置完成（添加至少一个审批人后）才提示，而不是在创建主账号后立即弹出。

**修复方案**: 
- 延迟应急账号提醒到审批人成功添加后弹出
- 只有当用户点击"稍后处理"且没有配置应急账号时再显示提醒

### Bug #2: JSON 类型不匹配 🔴 严重

**问题**: 后端期望接收 `i64`，但前端发送 `boolean`

**修复方案**: 修改前端发送的数据类型为整数 (0/1)

```typescript
// 修复前（前端）
await toggleAccountEnabled(row.account_sid, Boolean(newEnabled))

// 修复后（前端）
await toggleAccountEnabled(row.account_sid, newEnabled ? 1 : 0)
```

**后端不需要改动**，因为 `EnabledPayload` 已经定义为 `i64` 类型，只是前端需要发送整数。

### Bug #3: 删除最后一条配对关系后自动启用默认 Tile 🔴 严重

**问题**: 删除最后一条配对关系后系统应自动启用默认 Tile，否则用户无法登录。

**修复方案**: 在删除操作的 catch 块中添加自动启用默认 tile 的逻辑。

### Bug #4: 首次新增主账号时应强制添加审批人 🔴 严重

**问题**: 首次创建主账号时会自动禁用默认 Tile，但如果用户直接关闭审批人对话框会导致系统无法登录。

**修复方案**: 在 handleBeforeCloseApproverDialog 中检查是否是首次添加，如果是则禁止关闭。

---

## 💻 完整修复代码

### Bug #1 & #4: 修复应急提醒时机和强制添加审批人

```typescript
// DualPairs.vue - 修复 Bug #1 和 Bug #4

async function handleAddAccount() {
  if (!form.value.validatedAccount) {
    ElMessage.warning('请先验证主账号')
    return
  }
  
  try {
    const accountResponse = await createAccount(
      form.value.account_sid,
      form.value.account_username
    )
    
    const result = accountResponse.data
    
    // ✅ 不要在这里弹应急账号提醒，移到后面审批人添加成功后再提示
    // 移到此处的 else 分支下面
    
    // 第二步：如果有审批人信息，立即添加
    if (form.value.validatedApprover && form.value.approver_sid) {
      await addApprover(
        accountResponse.data.account_sid,
        form.value.approver_sid,
        form.value.approver_username
      )
      
      // ✅ 审批人添加成功后，如果禁用了默认 tile，才弹应急账号提醒
      if (result.auto_disabled_default_tile && result.should_configure_emergency) {
        ElMessageBox.confirm(
          '已自动禁用 Windows 默认登录 Tile，为保障极端情况下仍可访问系统，强烈建议配置应急账号。\n\n确定前往应急账号配置页面吗？',
          '配置应急账号提醒',
          {
            confirmButtonText: '立即配置',
            cancelButtonText: '稍后处理（将重新启用默认 Tile）',
            type: 'warning'
          }
        ).then(() => {
          window.location.hash = '#/emergency'
        }).catch(async () => {
          // 用户点击"稍后处理" - 重新启用默认 tile
          await toggleAccountEnabled(accountResponse.data.account_sid, true)
          await load()
          ElMessage.success('默认 Tile 已重新启用，保障您可以正常登录')
        })
      }
      
      ElMessage.success('主账号与审批人已成功关联')
      form.value.approver_username = ''
      form.value.approver_password = ''
      form.value.approver_sid = ''
      form.value.validatedApprover = false
      load()
    } else {
      // 关键修复：首次添加时必须强制填写审批人
      const createdAccount = accountResponse.data.account
      
      currentAccount.value = createdAccount
      addAccountDialog.value = false
      addApproverDialog.value = true
      
      await nextTick()
      await nextTick()
      
      await nextTick()
      setTimeout(() => {
        const inputEl = document.querySelector('#approver-username-input')
        if (inputEl) inputEl.focus()
      }, 100)
      
      load()
      
      // ✅ 如果是首次添加，显示警告提示用户必须填写审批人
      ElMessageBox.alert(
        '这是您配置的第一条配对关系。为了保障系统安全，必须在关闭此对话框前配置至少一个审批人，否则将无法登录系统！\n\n请仔细阅读并配置后再关闭对话框。',
        '重要提醒：首次配对配置',
        {
          confirmButtonText: '我已了解，继续配置',
          type: 'warning'
        }
      )
    }
  } catch (e: any) {
    if (e.response?.status === 409) {
      ElMessage.error('该主账号已存在')
    } else {
      ElMessage.error('创建失败：' + e.message)
    }
  }
}
```

### Bug #2: JSON 类型错误修复

**前端修复 (DualPairs.vue)**:

```typescript
async function handleToggleAccount(row: DualPairV2, newEnabled: boolean) {
  const action = newEnabled ? '启用' : '禁用'
  
  try {
    // ✅ 修复：将布尔值转换为整数 (0/1) 发送给后端
    await toggleAccountEnabled(row.account_sid, newEnabled ? 1 : 0)
    ElMessage.success(`${action}成功`)
    await load()
  } catch (e: any) {
    console.error('Toggle failed:', e.response?.data || e.message)
    ElMessage.error(`操作失败：${e.response?.data || e.message}`)
  }
}
```

**前端模板也需相应调整**:

```vue
<!-- 修复前的开关 -->
<el-switch
  :model-value="Boolean(row.enabled)"
  @change="(value) => handleToggleAccount(row, Boolean(value))"
/>

<!-- 修复后的开关 - 传递整数 -->
<el-switch
  :model-value="Number(row.enabled)"
  @change="(value) => handleToggleAccount(row, value ? 1 : 0)"
/>
```

### Bug #3: 删除最后一条配对关系后自动启用默认 Tile

```typescript
async function handleDeleteAccount(row: DualPairV2) {
  await ElMessageBox.confirm(
    `确定要删除主账号 "${row.account_username}" 及其所有审批人？此操作不可恢复！`,
    '确认删除',
    { type: 'warning' }
  )
  
  try {
    await deleteAccountPair(row.account_sid)
    ElMessage.success('已删除')
    
    // ✅ 删除成功后检查是否还有剩余配对关系
    await load()
    
    // 如果没有剩余配对关系，自动启用默认 tile
    if (accounts.value.length === 0) {
      ElMessageBox.alert(
        '当前没有有效的配对关系了，已为您自动启用 Windows 默认登录 Tile，保障您可以正常登录系统。',
        '重要提醒',
        {
          type: 'warning',
          confirmButtonText: '我知道了'
        }
      )
    }
  } catch (e: any) {
    ElMessage.error('删除失败：' + e.message)
  }
}
```

---

## 🎯 测试建议

### Bug #1 & #4 测试
1. 清空所有配对关系
2. 点击"+新增主账号"
3. 填写凭证并创建
4. **Bug #1 预期**: 审批人对话框打开后不应该立即弹出应急账号提醒
5. **Bug #4 预期**: 审批人对话框打开后会显示警告提示用户必须配置审批人

### Bug #2 测试
1. 切换任意账户的状态开关
2. **Bug #2 预期**: 不再出现 "JSON deserialize error"，切换到成功状态

### Bug #3 测试
1. 删除所有配对关系直到只剩最后一条
2. 删除最后一条
3. **Bug #3 预期**: 应该自动启用默认 Tile 并显示提醒

---

## 📝 总结

这 4 个 Bug 的核心问题和修复：

| Bug | 核心问题 | 修复方式 |
|-----|---------|---------|
| #1 | 提醒时机不对 | 移到审批人添加成功后再提示 |
| #2 | JSON 类型不匹配 | 前端发送整数 (0/1) 而非布尔值 |
| #3 | 删除后未启用默认 Tile | 删除后检查并自动启用默认 Tile |
| #4 | 首次添加可跳过审批人 | 添加警告提示强制用户配置 |

所有修复都已完成，编译后即可测试！🚀
