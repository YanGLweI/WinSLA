# WinSLA v2.2.6 - 开发环境 Tauri 应用启动与测试指南

## 🚀 快速启动步骤

### Step 1: 确保依赖服务运行

```powershell
# 打开终端 1 - 启动 Vite 前端开发服务器
cd C:\Users\YLW\Documents\PJ\WinSLA\management_app
npm run dev
```

**预期输出:**
```
VITE v6.4.3 ready in xxx ms
➜ Local: http://localhost:5174/
```

---

### Step 2: 以管理员身份启动 Tauri 管理端

**重要:** Tauri 应用需要管理员权限才能访问 SQLite 数据库和绑定本地端口。

#### **方法 A: 使用 Cargo 直接启动（推荐）**

1. **打开 PowerShell（管理员模式）**
   ```powershell
   # 右键点击 PowerShell 图标 → 以管理员身份运行
   ```

2. **导航到项目目录并编译运行**
   ```powershell
   cd C:\Users\YLW\Documents\PJ\WinSLA
   
   # 先构建（首次或修改代码后）
   cd management_app\src-tauri
   cargo build --release
   ```

3. **启动管理端应用**
   ```powershell
   .\target\release\winsla-management.exe
   ```

#### **方法 B: 使用 VSCode（如已安装）**

1. **在 VSCode 中打开整个项目**:
   ```powershell
   code C:\Users\YLW\Documents\PJ\WinSLA
   ```

2. **按 F5 调试**（如果配置了 launch.json）
   或手动运行 `management_app\src-tauri\target\release\winsla-management.exe`

---

### Step 3: 验证应用启动成功

Tauri 窗口应该弹出并显示以下内容之一：

✅ **正常情况**:
- 显示"WinSLA Management"主窗口
- 顶部导航栏包含：WinSLA、仪表盘、配对规则、应急账号等
- "配对规则"标签页可正常工作

❌ **异常情况（请检查）**:
- ❌ 报错"无法加载模块" → 缺少 Visual Studio C++ 运行库
- ❌ 报错"数据库锁定时锁定" → 另一个实例已运行
- ❌ 白屏或空界面 → 检查 Console 是否有 JavaScript 错误

---

##  完整测试流程（请逐步执行）

### TC01: 页面初始化验证 ✅

**操作**:
1. 打开配对规则标签页：http://localhost:5174/#/pairs

**验证点**:
- [ ] 页面标题显示"配对规则"
- [ ] "+ 新增主账号"蓝色按钮可见
- [ ] 表头完整显示（主账号、审批人列表、主账号 SID、状态、创建时间、操作）
- [ ] Console 无 JavaScript 错误 (F12 → Console)

---

### TC02: 创建主账号 + 同时添加审批人 ⭐ 关键测试 ⭐

**操作**:
1. 点击"+ 新增主账号"蓝色按钮

**验证**: 对话框应打开且包含所有字段

2. 填写主账号凭证:
   - Username: `ylw`
   - Password: `!Qw2!Qw2!Qw2!Qw2`

3. 点击蓝色"验证主账号"按钮

**验证**:
- [ ] 绿色提示出现："主账号已验证，准备创建配对规则"
- [ ] "创建主账号"按钮变绿可点击

4. 填写审批人凭证:
   - Username: `zbj`
   - Password: `admin123456.`

5. 点击蓝色"验证审批人"按钮

**验证**:
- [ ] 绿色提示出现："审批人已验证，可添加到主账号"
- [ ] "添加审批人"按钮变绿可点击

6. 点击绿色"创建主账号"按钮

**关键验证**（Bug 修复验证）:
- [ ] ✅ 不弹出"是否禁用账号"确认框（Bug #1 Fixed!）
- [ ] ✅ 立即刷新列表并显示新创建的主账号（Bug #3 Fixed!）
- [ ] ✅ 列表中显示审批人标签，无空白标签（Bug #2 Fixed!）
- [ ] ✅ 显示成功消息"主账号与审批人已成功关联"
- [ ] ✅ 审批人对话框保持打开（可继续添加更多审批人）

---

### TC03: JSON 数据解析验证（Bug #4） ⭐⭐⭐ 最重要 ⭐⭐⭐

**操作**:
1. 刷新页面 (F5)

**关键验证**:
- [ ] Console **不应出现**错误：`(e.approvers || []).filter is not a function`
- [ ] 所有数据正确加载，包括主账号和审批人信息
- [ ] 审批人以绿色标签形式显示

**Console 截图**:
- 按 F12
- 切换到 Console 标签
- 截图应显示干净的控制台（无红色错误）

---

### TC04: 启用/禁用开关验证（Bug #1 API 层验证） ⭐⭐⭐

**操作**:
1. 打开 DevTools Network 标签 (F12 → Network)
2. 找到刚才创建的主账号 ylw
3. 点击状态列的启用/禁用开关
4. 在确认对话框中点击"确定"

**Network 标签验证**（最关键）:
- [ ] 查看 PUT /api/accounts/{account_sid}/enable 请求
- [ ] **Status Code 必须为 200 OK**（不是 422!）
- [ ] Response Body 显示 `{"enabled": true/false}`

**截图要求**:
- Network 标签截图应清晰显示：
  - Request URL
  - Status: 200 OK ✅
  - Response: {"enabled": ...}

---

### TC05: 批量添加审批人

**操作**:
1. 点击已创建账号右侧的"+ 审批人"按钮
2. 输入第二个审批人凭证（如有）或任意有效凭证
3. 验证并添加

**验证**:
- [ ] 两个审批人以独立标签形式显示在同一行
- [ ] 每个标签都可单独删除（点击 ×）
- [ ] 不影响其他审批人

---

### TC06: 删除单个审批人

**操作**:
1. 在审批人列表中点击某个标签的红色 ×
2. 确认删除对话框

**验证**:
- [ ] 该审批人标签消失
- [ ] 其他审批人保留
- [ ] 列表正常显示剩余内容

---

### TC07: 只创建主账号后自动跳转

**操作**:
1. 再次点击"+ 新增主账号"
2. 仅填写主账号凭证并验证
3. **不填写审批人信息**
4. 点击"创建主账号"

**验证**:
- [ ] 显示"主账号已创建，请添加第一个审批人"
- [ ] 审批人对话框自动弹出
- [ ] Focus 移动到审批人用户名输入框
- [ ] 列表已显示新创建的主账号（即使没有审批人）

---

## 📊 测试结果记录模板

复制此模板填写实际测试结果：

```markdown
# WinSLA v2.2.6 - 开发环境手动测试报告

**测试日期**: 2026-09-08  
**浏览器**: Chrome/Firefox/Edge ___ 版本  
**Tauri 版本**: 2.2.6  

## Test Results

| TC | 测试项 | 状态 | 备注 |
|----|--------|------|------|
| TC01 | 页面初始化 | ☐ Pass / ☐ Fail | ___ |
| TC02 | 创建账号 + 添加审批人 | ☐ Pass / ☐ Fail | Bug #1, #2, #3 |
| TC03 | JSON 解析验证 | ☐ Pass / ☐ Fail | **Bug #4 最关键** |
| TC04 | HTTP 200 验证 | ☐ Pass / ☐ Fail | **Bug #1 API 层** |
| TC05 | 批量添加审批人 | ☐ Pass / ☐ Fail | ___ |
| TC06 | 删除单个审批人 | ☐ Pass / ☐ Fail | ___ |
| TC07 | 自动跳转逻辑 | ☐ Pass / ☐ Fail | Bug #3 补充 |

## Critical Findings

### Bug #1 Fix (Toggle Switch)
- **Before**: Returns HTTP 422 ❌
- **After**: Returns HTTP 200 ✅
- **Evidence**: Network tab screenshot attached

### Bug #2 Fix (Empty Labels)
- **Before**: Shows empty tags ❌
- **After**: Filters out empty users ✅
- **Evidence**: Screenshot shows only valid approver labels

### Bug #3 Fix (List Refresh)
- **Before**: List not refreshing after create ❌
- **After**: List refreshes immediately ✅
- **Evidence**: New account appears in table

### Bug #4 Fix (JSON Parsing)
- **Before**: "(e.approvers || []).filter is not a function" ❌
- **After**: JSON correctly parsed and filtered ✅
- **Evidence**: Console clean, no errors

## Conclusion

☐ ALL TESTS PASSED - Ready for deployment  
☐ Some issues found - See notes above  
☐ Critical bugs remaining - Cannot deploy

## Attachments

Screenshots should include:
- tc02-dialog-screenshot.png (after validation)
- tc03-console-clean.png (no errors after refresh)
- tc04-network-200-ok.png (PUT request status)
- tc05-multiple-approvers.png (multiple green tags)

Notes:
_______________________________________________________________________________
_______________________________________________________________________________
_______________________________________________________________________________
```

---

## 🔧 常见问题排查

### 问题 1: 页面白屏或加载中
**原因**: 后端 API 未启动或未正确响应  
**解决**: 
- 检查 Tauri 应用是否正常启动
- 查看 Console 是否有 CORS 错误
- 确认端口 5174 被 Vite 占用

### 问题 2: 弹窗"无法打开对话框"
**原因**: Vue 组件渲染失败  
**解决**:
- 清除浏览器缓存 (Ctrl+Shift+Delete)
- Hard refresh (Ctrl+F5)
- 检查 Console 是否有 JS 错误

### 问题 3: "该主账号已存在"
**原因**: 数据库中已有相同 SID 的记录  
**解决**:
- 检查 `C:\ProgramData\WinSLA\winsla.db`
- 或更换不同的域账号测试

### 问题 4: JSON 解析错误
**原因**: 数据库中 approvers 字段格式不正确  
**解决**:
- 这是 Bug #4，已在最新版修复
- 确认使用的是最新版本安装包

---

## 💡 测试建议

1. **按顺序执行**: TC01 → TC02 → TC03 → TC04 → TC05 → TC06 → TC07
2. **截图保存**: 对每个关键步骤截图保存证据
3. **注意 Console**: 时刻关注浏览器 Console 是否有错误
4. **检查 Network**: TC04 时必须查看 Network 标签的状态码
5. **详细记录**: 将结果填入测试报告模板

祝您测试顺利！有任何问题请随时提供详细信息！🚀
