# WinSLA v2.2.6 本地测试环境设置指南

## 🎯 测试方案概述

由于 Vite 开发服务器启动遇到问题，我们采用以下替代方案进行手动测试：

### **推荐方案：使用静态 HTML 测试页面**

1. **打开测试页面**: `file:///C:/Users/YLW/Documents/PJ/WinSLA/management_app/manual-test.html`
2. **功能验证**: 页面提供交互式测试工具和检查清单
3. **API 测试**: 自动检测后端 API 是否可用

---

## 📦 快速开始

### 方式 1: 直接打开测试页面（推荐）

```powershell
# 方法 A: 双击文件
explorer "C:\Users\YLW\Documents\PJ\WinSLA\management_app\manual-test.html"

# 方法 B: 使用 PowerShell
start "C:\Users\YLW\Documents\PJ\WinSLA\management_app\manual-test.html"
```

这将自动在默认浏览器中打开测试页面。

---

### 方式 2: 尝试启动 Vite 开发服务器

```powershell
cd management_app

# 确保依赖已安装
npm install

# 启动开发服务器（可能需要较长时间）
npm run dev
```

**预期输出**:
```
VITE v6.4.3 ready in xxx ms
➜  Local:   http://localhost:5173/
```

如果成功，访问：http://localhost:5173

---

### 方式 3: 使用 Preview 模式（稳定的预览环境）

```powershell
cd management_app
npm run preview

# 然后访问控制台显示的地址（通常是 http://127.0.0.1:4173）
```

---

## 🔍 测试检查清单

打开测试页面后，请依次执行以下测试：

### ✅ 基础功能测试

| # | 测试项 | 操作 | 预期结果 | 状态 |
|---|--------|------|---------|------|
| 1 | Vue 组件初始化 | 点击"Test Vue Initialization"按钮 | 显示 Success | ☐ |
| 2 | 表单数据结构 | 点击"Check Form Data"按钮 | 包含 validatedAccount/approver | ☐ |
| 3 | API 连接 | 点击"Check API Connectivity"按钮 | 如果服务运行则显示 OK | ☐ |

### ✅ 核心功能测试

请在实际的应用界面中进行以下测试：

#### 测试项 4: 验证成功后显示绿色标签

1. 打开应用界面
2. 点击"+ 新增主账号"
3. 输入主账号信息并验证
4. **检查**: 是否显示绿色标签 "主账号已验证，准备创建配对规则"
5. **检查**: "创建主账号"按钮是否变绿可用

#### 测试项 5: 同时添加审批人

1. 在主账号对话框中输入审批人信息
2. 验证审批人
3. **检查**: 是否显示绿色标签 "审批人已验证，可添加到主账号"
4. **检查**: "添加审批人"按钮是否变绿可用
5. 点击"创建主账号"
6. **检查**: 是否自动调用 addApprover API
7. **检查**: 两个对话框是否同时关闭

#### 测试项 6: 仅创建主账号后的自动跳转

1. 只验证主账号（不验证审批人）
2. 点击"创建主账号"
3. **检查**: 主账号对话框是否关闭
4. **检查**: 审批人对话框是否自动打开
5. **检查**: 审批人输入框是否获得焦点

#### 测试项 7: 启用/禁用开关

1. 找到任意已有记录
2. 点击状态开关
3. **检查**: 是否弹出确认对话框
4. 确认后
5. **检查**: 状态切换成功
6. **检查**: API 响应为 200 OK（不是 422）
7. **检查**: 列表刷新显示新状态

---

## 🛠️ 调试工具

### 浏览器开发者工具

按 **F12** 打开开发者工具，重点关注：

1. **Console 标签页**: 查看 JavaScript 错误和控制台日志
2. **Network 标签页**: 查看 HTTP 请求和响应
3. **Elements 标签页**: 检查 DOM 结构和样式

### 关键调试命令

在 Console 中输入：

```javascript
// 检查全局变量
console.log('Vue version:', typeof Vue !== 'undefined' ? Vue.version : 'not found');

// 检查表单数据
const formEl = document.querySelector('.el-form');
if (formEl) {
    const vnode = formEl.__vnode;
    console.log('Form props:', vnode?.props);
    console.log('validatedAccount:', vnode?.props?.validatedAccount);
}

// 监听事件绑定
const btn = document.querySelector('[onclick*="handleValidate"]');
if (btn) {
    console.log('Event listeners:', btn._events || Object.getOwnPropertyDescriptors(btn));
}
```

---

## 📊 API 接口测试

在 Network 标签页中，检查以下 API 调用：

### POST /api/accounts
- **请求体**: `{ account_sid, account_username }`
- **预期状态码**: 201 Created
- **响应**: DualPairV2 对象

### POST /api/accounts/approvers
- **请求体**: `{ account_sid, approver_sid, approver_username }`
- **预期状态码**: 201 Created
- **响应**: 空或成功消息

### PUT /api/accounts/{sid}/enable
- **请求体**: `{ enabled: true/false }`
- **预期状态码**: 200 OK
- **警告**: **不应出现 422 Unprocessable Entity**

---

## 🚨 常见问题排查

### 问题 1: 测试页面空白

**可能原因**: 
- 前端资源未正确加载
- Vue 组件渲染失败

**解决方法**:
1. 按 F12 打开开发者工具
2. 切换到 Console 标签
3. 查看是否有红色错误信息
4. 检查 Network 标签页，确认所有资源都成功加载（蓝色状态码）

---

### 问题 2: 验证按钮无反应

**可能原因**:
- handleValidateAccount 函数未正确定义
- API 调用超时或失败

**解决方法**:
1. 检查 Console 是否有 JS 错误
2. 检查 Network 是否发送了正确的请求
3. 确认后端服务正在运行（端口 19830）

---

### 问题 3: 绿色标签不显示

**可能原因**:
- form.value.validatedAccount 状态未正确更新
- Vue 响应式系统失效

**解决方法**:
```javascript
// 在 Console 中运行调试代码
const test = () => {
    // 查找表单元素
    const formEls = document.querySelectorAll('.el-form-item');
    console.log('Form elements found:', formEls.length);
    
    // 检查验证状态
    const tags = document.querySelectorAll('.el-tag[type="success"]');
    console.log('Success tags found:', tags.length);
};

test();
```

---

## 📝 测试结果记录模板

请按以下格式记录您的测试结果：

```markdown
## WinSLA v2.2.6 测试结果报告

### 测试信息
- **日期**: [填写日期]
- **时间**: [填写时间]
- **浏览器**: [Chrome/Firefox/Edge + 版本]
- **测试 URL**: manual-test.html 或 http://localhost:5173

### 功能测试结果

#### Bug Fix Verification
| Bug ID | 描述 | 预期结果 | 实际结果 | 状态 |
|--------|------|---------|---------|------|
| Bug 1 | 启用/禁用开关无 422 错误 | 正常切换 | [填写] | ☐ Pass/☐ Fail |
| Bug 2 | 审批人验证状态同步 | 验证后立即可用 | [填写] | ☐ Pass/☐ Fail |
| Bug 3 | 同时添加主账号和审批人 | 自动添加 | [填写] | ☐ Pass/☐ Fail |

#### New Features
| Feature | 描述 | 实际结果 | 状态 |
|---------|------|---------|------|
| 绿色验证标签 | 显示"已验证"状态 | [填写] | ☐ Pass/☐ Fail |
| 按钮自动启用 | 验证后变绿可用 | [填写] | ☐ Pass/☐ Fail |
| 自动跳转逻辑 | 创建后自动打开审批人对话框 | [填写] | ☐ Pass/☐ Fail |

### API 测试结果

| API EndPoint | 方法 | 状态码 | 响应正确 | 状态 |
|--------------|------|--------|---------|------|
| /api/accounts | POST | [填写] | ☐ Yes/☐ No | ☐ Pass/☐ Fail |
| /api/accounts/approvers | POST | [填写] | ☐ Yes/☐ No | ☐ Pass/☐ Fail |
| /api/accounts/:id/enable | PUT | [填写] | ☐ Yes/☐ No | ☐ Pass/☐ Fail |

### 发现的问题

[在此处记录任何异常行为或未通过的功能]

### 结论

☐ **所有测试通过** - 可以部署到虚拟机
☐ **部分通过** - 需要修复某些问题后重新测试
☐ **存在严重问题** - 需要重新分析需求和修改代码

### 建议的下一步行动

[填写具体的后续操作]
```

---

## 💡 最佳实践建议

1. **先做基础测试**: 确保 Vue 组件和 API 连接正常
2. **逐一验证功能**: 按照测试清单顺序进行
3. **详细记录结果**: 使用提供的模板记录每个测试项
4. **截图保存**: 对通过的测试项和出现的错误进行截图
5. **及时反馈**: 将测试结果和问题反馈给开发团队

---

## 🔄 后续流程

根据测试结果决定下一步：

**✅ 全部通过**:
- 重新编译生产版本
- 生成新的 NSIS 安装包
- 部署到虚拟机进行端到端测试

**⚠️ 部分通过**:
- 针对具体问题编写修复方案
- 修复后进行回归测试
- 确认所有功能正常后再部署

**❌ 未通过**:
- 收集详细的错误信息和日志
- 重新分析需求文档
- 可能需要调整整体实现方案

---

## 📞 获取帮助

如果在测试过程中遇到任何问题：

1. 检查 Console 和 Network 标签中的错误信息
2. 使用调试工具定位问题
3. 记录详细的复现步骤和错误日志
4. 联系开发团队寻求帮助

---

祝您测试顺利！🎉
