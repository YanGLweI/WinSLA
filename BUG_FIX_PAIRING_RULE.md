# WinSLA Bug 修复验证脚本

## 问题描述
创建主账号时可以不关联任何审批人，但只要成功创建第一个主账号，即使没有关联审批人，也会自动把策略配置里的"默认登录 Tile"关闭。这可能导致死锁导致锁屏或重启电脑后无法登录系统。

## 修复方案概述
将禁用 default tile 的时机从"创建主账号"延迟到"添加第一个审批人"

## 修改文件清单

### 1. backend: management_app/src-tauri/server.rs

#### 修改 create_account 函数 (L214-230)
**修改前**: 创建第一个账号时自动禁用 default tile
**修改后**: 只创建账号，不修改策略

```rust
async fn create_account(...) -> impl IntoResponse {
    // ✅ Bug 修复：不再在创建账号时自动禁用默认 Tile，改为在添加审批人时处理
    let account = match db.create_account_pair(&req.account_sid, &req.account_username) {
        Ok(account) => account,
        Err(_) => return (StatusCode::CONFLICT, "Account already exists").into_response(),
    };
    
    Json(account).into_response()
}
```

#### 修改 add_approver 函数 (L233-285)
**修改前**: 仅添加审批人，不处理策略
**修改后**: 添加审批人时检查是否为第一条完整配对，如果是则禁用 default tile

```rust
async fn add_approver(State(db): State<AppState>, Json(req): Json<AddApproverRequest>) -> impl IntoResponse {
    let result = {
        let db = db.lock().unwrap();
        db.add_approver_to_account(&req.account_sid, &req.approver_sid, &req.approver_username)
    };
    
    match result {
        Ok(true) => {
            // ✅ Bug 修复：检查是否为第一条完整配对（有审批人的账号）
            let should_disable_tile = {
                let db = db.lock().unwrap();
                
                // 获取该账号的 approvers 列表
                let existing_approvers = db.conn.query_row(
                    "SELECT approvers FROM dual_pairs_v2 WHERE account_sid = ?1",
                    params![&req.account_sid],
                    |row| row.get::<_, String>(0)
                ).ok();
                
                // 检查当前账号是否有审批人
                let has_approvers_in_this_account = existing_approvers.as_ref().map(|s| {
                    serde_json::from_str::<Vec<serde_json::Value>>(s).ok().map(|v| !v.is_empty()).unwrap_or(false)
                }).unwrap_or(false);
                
                // 获取所有账号
                let all_accounts = db.get_all_accounts().unwrap_or_default();
                
                // 检查是否只有这一个账号且它有审批人
                has_approvers_in_this_account && all_accounts.len() == 1
            };
            
            drop(db);
            
            // ✅ 如果是第一条完整配对，禁用默认 Tile
            if should_disable_tile {
                let mut policy_config = crate::database::PolicyConfig::default();
                let db = db.lock().unwrap();
                
                // 读取现有配置
                if let Ok(cfg) = db.get_policy() {
                    policy_config = cfg;
                }
                
                if policy_config.default_tile_enabled {
                    policy_config.default_tile_enabled = false;
                    let _ = db.save_policy(&policy_config);
                    
                    #[cfg(windows)]
                    {
                        if let Err(e) = write_policy_to_registry(&policy_config) {
                            eprintln!("Warning: Failed to update registry when adding first approver: {}", e);
                        }
                    }
                }
            }
            
            StatusCode::CREATED.into_response()
        },
        // ... 其他分支不变
    }
}
```

### 2. frontend: management_app/src/views/DualPairs.vue

#### 修改 handleAddAccount 函数 (L136-190)
**修改前**: 创建账号后可能直接添加审批人并处理嵌套响应
**修改后**: 创建账号后打开审批人对话框，等待用户操作

```typescript
async function handleAddAccount() {
  if (!form.value.validatedAccount) {
    ElMessage.warning('请先验证主账号')
    return
  }
  
  try {
    // 第一步：创建主账号（不再自动禁用 default tile）
    await createAccount(
      form.value.account_sid,
      form.value.account_username
    )
    
    ElMessage.success('主账号已创建')
    
    // ✅ 直接打开审批人对话框，等待用户添加审批人
    addAccountDialog.value = false
    addApproverDialog.value = true
    
    // 先设置值，再打开对话框，触发更新
    currentAccount.value = {
      account_sid: form.value.account_sid,
      account_username: form.value.account_username,
      approvers: '[]',
      enabled: true,
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString()
    }
    
    await nextTick()
    await nextTick()
    await nextTick()
    setTimeout(() => {
      const inputEl = document.querySelector('#approver-username-input')
      if (inputEl) inputEl.focus()
    }, 100)
    
    load()
  } catch (e: any) {
    if (e.response?.status === 409) {
      ElMessage.error('该主账号已存在')
    } else {
      ElMessage.error('创建失败：' + e.message)
    }
  }
}
```

#### 修改 handleConfirmAddApprover 函数 (L246-323)
**修改前**: 添加审批人后仅提示应急账号配置
**修改后**: 添加审批人后检查是否为第一条完整配对，自动禁用 default tile 并提示配置应急账号

```typescript
async function handleConfirmAddApprover() {
  // 修改为检查 form.value.validatedApprover
  if (!currentAccount.value || !form.value.validatedApprover || !form.value.approver_sid) {
    ElMessage.warning('请先验证审批人账号')
    return
  }
  
  try {
    console.log('Adding approver to:', currentAccount.value.account_sid, 'with approver:', form.value.approver_sid)
    
    await addApprover(
      currentAccount.value.account_sid,
      form.value.approver_sid,
      form.value.approver_username
    )
    ElMessage.success('审批人已添加到主账号')
    addApproverDialog.value = false
    
    // 重新加载数据
    await load()
    
    // ✅ Bug 修复：检查是否为第一条完整配对，如果是则禁用 default tile 并提示配置应急账号
    const { getAccounts, getPolicy } = await import('../api')
    const accountsResponse = await getAccounts()
    const policyResponse = await getPolicy()
    
    const accounts = accountsResponse.data || []
    const policyData = policyResponse.data || {}
    
    // 判断是否有至少一个审批人的完整配对
    const hasAnyCompletePair = accounts.some(acc => {
      let approversArray = []
      try {
        if (typeof acc.approvers === 'string') {
          approversArray = JSON.parse(acc.approvers)
        } else if (Array.isArray(acc.approvers)) {
          approversArray = acc.approvers
        }
      } catch (e) {
        console.error('Failed to parse approvers:', e)
      }
      return approversArray.length > 0
    })
    
    // ✅ 只有当这是第一个完整配对且 default_tile_enabled 仍为 true 时，才禁用并提示
    if (accounts.length === 1 && hasAnyCompletePair && policyData.default_tile_enabled !== false) {
      // 自动禁用 default tile
      const updatedPolicy = {
        ...policyData,
        default_tile_enabled: false
      }
      await updatePolicy(updatedPolicy)
      
      ElMessageBox.confirm(
        '已自动禁用 Windows 默认登录 Tile。为保障极端情况下仍可访问系统，强烈建议配置应急账号。\n\n确定前往应急账号配置页面吗？',
        '配置应急账号提醒',
        {
          confirmButtonText: '立即配置',
          cancelButtonText: '稍后处理（将重新启用默认 Tile）',
          type: 'warning'
        }
      ).then(() => {
        window.location.hash = '#/emergency'
      }).catch(async () => {
        // 如果用户取消，恢复启用 default tile
        await updatePolicy({
          ...policyData,
          default_tile_enabled: true
        })
        await load()
        ElMessage.success('默认 Tile 已重新启用，保障您可以正常登录')
      })
    }
  } catch (e: any) {
    console.error('Add approver failed:', e.response?.status, e.response?.data, e.message)
    if (e.response?.status === 409) {
      ElMessage.warning('该审批人已存在')
    } else {
      ElMessage.error('添加失败：' + (e.response?.data?.message || e.message))
    }
  }
}
```

## 预期行为

### 修复后的正确流程

1. **创建第一个主账号（无审批人）**
   - `default_tile_enabled` = true (保持不变)
   - 显示消息："主账号已创建"
   - 打开审批人对话框

2. **为该主账号添加第一个审批人**
   - 检测到是第一条完整配对
   - 自动设置 `default_tile_enabled` = false
   - 写入注册表确保策略生效
   - 弹出对话框提示："已自动禁用 Windows 默认登录 Tile...是否配置应急账号"
     - 点击"立即配置" → 跳转到应急账号页面
     - 点击"稍后处理" → 恢复启用 default_tile_enabled = true

3. **后续创建更多主账号**
   - `default_tile_enabled` 不受影响

4. **删除所有配对或审批人**
   - 现有逻辑已经正确处理（Bug #4 修复）

## 测试步骤

### 前置条件
1. 确保服务停止：`systemctl stop winsla-service` 或任务管理器
2. 清除旧数据库（可选）：删除 `%PROGRAMDATA%\WinSLA\winsla.db`
3. 启动 Tauri 应用：`cd management_app && npm run tauri dev`

### 测试场景 1: 创建主账号但不添加审批人
1. 打开"主账号与审批人配置"页面
2. 点击"新增主账号"
3. 输入主账号信息并验证
4. 点击"创建主账号"
5. **验证**: 
   - ✅ 显示"主账号已创建"
   - ✅ 审批人对话框打开
   - ✅ 策略配置的"默认登录 Tile"仍然为 **开启**

### 测试场景 2: 创建主账号并添加审批人（第一条完整配对）
1. 在上一步基础上，输入审批人信息并验证
2. 点击"确认添加审批人"
3. **验证**:
   - ✅ 显示"审批人已添加到主账号"
   - ✅ 弹出"配置应急账号提醒"对话框
   - ✅ 策略配置的"默认登录 Tile"变为 **关闭**
   - ✅ 如果选择"立即配置" → 跳转到应急账号页面
   - ✅ 如果选择"稍后处理" → 恢复启用 default tile

### 测试场景 3: 多次创建主账号
1. 删除第 2 个测试账号
2. 创建第二个主账号
3. **验证**: `default_tile_enabled` 状态不变

### 测试场景 4: 浏览器自动化测试
使用 PowerShell 脚本:

```powershell
# 启动浏览器测试
.\scripts\browser-test-pairing.ps1

# 或通过 Selenium
npm run test:e2e
```

## 验证清单

- [x] 后端：移除 `create_account` 中的自动禁用逻辑
- [x] 后端：`add_approver` 中添加检查并为第一条完整配对禁用 default tile
- [x] 前端：简化 `handleAddAccount`，不再处理嵌套响应
- [x] 前端：在 `handleConfirmAddApprover` 中实现完整的检查和禁用逻辑
- [x] 前端：支持用户取消时的回退机制
- [x] API: `updatePolicy` 已存在于导入列表中

## 注意事项

1. **数据库路径**: 确保读取正确的数据库路径 (`C:/ProgramData/WinSLA/winsla.db`)
2. **注册表写入**: 必须同时更新注册表以确保策略立即生效
3. **错误处理**: 捕获 JSON 解析错误和数据库查询错误
4. **用户友好**: 提供明确的提示信息，允许用户回退操作

## 风险评估

- **风险等级**: 低
- **影响范围**: 配对规则创建流程
- **回滚方案**: 保留原有代码注释，可随时恢复

## 下一步行动

1. ✅ 代码修改完成
2. 🔲 本地编译测试
3. 🔲 集成测试
4. 🔲 E2E 测试
5. 🔲 文档更新
6. 🔲 提交到 Git
