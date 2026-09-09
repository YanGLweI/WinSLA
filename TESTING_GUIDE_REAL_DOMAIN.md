# WinSLA v2.2.6 - 真实域控环境端到端自动化测试脚本

## 测试目标
在本机开发环境中，使用真实域控账号进行完整的端到端功能测试，不依赖 Mock API。

## 测试账号信息
- **主账号**: ylw / !Qw2!Qw2!Qw2!Qw2
- **审批人**: zbj / admin123456.

## 测试环境
- Frontend: http://localhost:5174 (或 5175)
- Backend API: http://localhost:19830
- Domain Controller: 真实域控环境已接入

---

## 测试流程

### 步骤 1: 环境准备检查

```powershell
# 检查 Node 进程是否运行
Get-Process -Name node -ErrorAction SilentlyContinue | 
    Select-Object Id,StartTime | Format-Table

# 检查 Vite 开发服务器端口
netstat -ano | findstr "5174 5175"

# 检查 Rust 管理端 API 服务器
netstat -ano | findstr "19830"
```

### 步骤 2: 启动必要服务（如果需要）

```powershell
# 如果 Vite 未运行，启动前端开发服务器
cd management_app
npm run dev

# 如果 Rust 管理端未运行，需要启动它
# cd management_app\src-tauri
# cargo run --release
```

### 步骤 3: 执行端到端测试

请按照以下步骤手动或自动执行完整测试：

#### TC01: 页面加载与初始状态验证
1. 打开浏览器访问：http://localhost:5174/#/pairs (或 5175)
2. 验证页面标题显示"配对规则"
3. 检查导航栏可见性（WinSLA, 仪表盘，配对规则，应急账号等）
4. 确认"+ 新增主账号"按钮存在且可用
5. 记录初始列表状态（应为空或显示已有数据）

#### TC02: 创建主账号 + 同时添加审批人
**测试步骤**:
1. 点击"+ 新增主账号"按钮
2. 填写主账号信息:
   - Username: ylw
   - Password: !Qw2!Qw2!Qw2!Qw2
3. 点击"验证主账号"
4. 填写审批人信息:
   - Username: zbj
   - Password: admin123456.
5. 点击"验证审批人"
6. 点击"创建主账号"

**预期结果**:
- ✅ 绿色标签"主账号已验证，准备创建配对规则"出现
- ✅ 绿色标签"审批人已验证，可添加到主账号"出现
- ✅ "创建主账号"和"添加审批人"按钮变绿启用
- ✅ 创建成功后显示"主账号与审批人已成功关联"
- ✅ 对话框保持打开（可以继续添加更多审批人）
- ✅ 列表中立即显示新创建的主账号及其审批人
- ✅ 无错误提示

#### TC03: 只创建主账号后跳转添加审批人
**测试步骤**:
1. 再次点击"+ 新增主账号"
2. 填写主账号凭证并验证
3. 点击"创建主账号"（不填写审批人）

**预期结果**:
- ✅ 显示"主账号已创建，请添加第一个审批人"
- ✅ 审批人对话框自动弹出
- ✅ Focus 移到审批人输入框
- ✅ 列表已刷新显示新创建的主账号（即使还没有审批人）

#### TC04: 批量添加审批人
**测试步骤**:
1. 找到已创建的主账号 ylw 记录
2. 点击该行右侧的"+ 审批人"按钮
3. 输入第二个审批人的凭证并验证
4. 点击"添加审批人"

**预期结果**:
- ✅ 两个审批人以标签形式显示在同一行
- ✅ 每个标签都可单独关闭（删除）
- ✅ 不影响其他审批人的显示

#### TC05: 启用/禁用开关（Bug #1 验证）
**测试步骤**:
1. 找到任意主账号记录
2. 点击状态列的开关控件
3. 在确认对话框中点击"确定"
4. 打开浏览器 DevTools → Network 标签
5. 查找 PUT /api/accounts/{id}/enable 请求

**预期结果**:
- ✅ HTTP 状态码为 200 OK (NOT 422!)
- ✅ 响应体包含正确的 enabled 值
- ✅ UI 开关状态正确更新
- ✅ 无任何错误提示

#### TC06: 删除单个审批人
**测试步骤**:
1. 在审批人列表中点击某个标签的"x"
2. 在确认对话框中点击"确定"

**预期结果**:
- ✅ 该审批人标签消失
- ✅ 其他审批人保留不受影响
- ✅ 列表正常刷新显示剩余内容

#### TC07: JSON 数据解析验证（Bug #4 验证）
**测试步骤**:
1. 执行完上述所有操作后
2. 刷新页面 (F5)
3. 查看 Console 是否有 JavaScript 错误

**预期结果**:
- ✅ 无"(e.approvers || []).filter is not a function"错误
- ✅ 所有数据正确加载和显示
- ✅ 审批人数组正确解析为 JavaScript 对象

---

## 📊 测试结果记录模板

请复制以下模板并填写实际测试结果：

```markdown
# WinSLA v2.2.6 - 真实域控端到端测试报告

**测试日期**: 2026-09-08  
**测试环境**: Windows 本地开发环境 + 真实域控  
**Frontend URL**: http://localhost:5174/#/pairs  
**API URL**: http://localhost:19830  

## Test Results Summary

| Test Case ID | Description | Status | Notes |
|--------------|-------------|--------|-------|
| TC01 | Page Load & Initial State | ☐ Pass / ☐ Fail | ___ |
| TC02 | Create Account + Add Approver Together | ☐ Pass / ☐ Fail | ___ |
| TC03 | Create Account Only then Auto-jump | ☐ Pass / ☐ Fail | ___ |
| TC04 | Batch Add Approvers | ☐ Pass / ☐ Fail | ___ |
| TC05 | Enable/Disable Toggle (HTTP 200) | ☐ Pass / ☐ Fail | ___ |
| TC06 | Remove Single Approver | ☐ Pass / ☐ Fail | ___ |
| TC07 | JSON Parsing Validation | ☐ Pass / ☐ Fail | ___ |

## Detailed Findings

### TC01: Page Load & Initial State
- [ ] Frontend loads successfully
- [ ] Navigation visible and correct
- [ ] "+ New Main Account" button present
- [ ] No console errors on load

### TC02: Create Account + Add Approver Together
**Test Data**:
- Main Account: ylw / !Qw2!Qw2!Qw2!Qw2
- Approver: zbj / admin123456.

- [ ] Validation tags appear correctly
- [ ] Buttons enable after validation
- [ ] Success message shows
- [ ] Dialog remains open (can add more)
- [ ] Table refreshes showing new account
- [ ] Both accounts created with approver attached

**Screenshots**: 
- Before validation: [attach]
- After validation: [attach]
- After creation: [attach]

### TC03: Create Account Only then Auto-jump
- [ ] Creation message appears
- [ ] Approver dialog auto-opens
- [ ] Focus moves to approver input
- [ ] List shows new account immediately

### TC04: Batch Add Approvers
- [ ] Multiple approvers displayed as tags
- [ ] Each tag can be removed independently
- [ ] No UI issues with multiple entries

### TC05: Enable/Disable Toggle (Bug #1 Fix Verification)
**Critical Check**:
- [ ] HTTP status code: 200 OK (NOT 422!)
- [ ] Response body correct
- [ ] UI toggle works properly

**Network Tab Screenshot**: [attach]

### TC06: Remove Single Approver
- [ ] Specific approver removed
- [ ] Other approvers preserved
- [ ] List refreshes correctly

### TC07: JSON Parsing Validation (Bug #4 Fix Verification)
**Critical Check**:
- [ ] NO "filter is not a function" error
- [ ] All data loads correctly after refresh
- [ ] Console clean (no critical errors)

## Critical Bug Fixes Verified

| Bug ID | Issue | Verification |
|--------|-------|--------------|
| Bug #1 | Toggle returns HTTP 200 | ✅ VERIFIED if TC05 passes |
| Bug #2 | Empty labels display | ✅ VERIFIED if TC06 passes |
| Bug #3 | List not refreshing after create | ✅ VERIFIED if TC03 passes |
| Bug #4 | JSON parsing error | ✅ VERIFIED if TC07 passes |

## Conclusion

[ ] ALL TESTS PASSED - Ready for deployment
[ ] Some tests failed - See notes above
[ ] Critical bugs found - Cannot deploy yet

## Recommended Next Steps
_______________________________________________________________________________
_______________________________________________________________________________
_______________________________________________________________________________

Report Generated By: [Your Name]
Test Environment: [Machine Name/Domain Info]
Time Taken: __ minutes
```

---

## 🛠️ 自动化测试脚本 (可选)

如果您想使用命令行自动化测试，可以运行：

```powershell
# 确保开发服务器运行
cd C:\Users\YLW\Documents\PJ\WinSLA\management_app
npm run dev

# 等待启动完成后，在浏览器中手动执行测试流程
# 或使用 Playwright/Cypress 等 E2E 工具录制测试
```

## 🔍 调试建议

如果测试失败，请检查：
1. **浏览器 Console** - F12 → Console 标签，查看 JavaScript 错误
2. **Network 标签** - F12 → Network 标签，检查 API 响应状态码
3. **后端日志** - 查看 winsla-service 的日志文件
4. **数据库** - 检查 SQLite 数据库中的数据结构是否正确

---

祝您测试顺利！如有任何问题，请提供详细的错误信息和截图。
