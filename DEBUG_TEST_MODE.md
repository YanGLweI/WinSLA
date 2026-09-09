# WinSLA v2.2.6 - 测试模式启动指南

## ⚠️ **问题诊断**

从之前的检测发现：**当前开发环境可能未加入域或无法连接域控制器**。

这导致：
- ❌ 域账户验证失败（ylw/zbj 无法通过 LSA 验证）
- ❌ API 返回"账号密码错误"

---

## 🔧 **解决方案选择**

### **方案 A: 使用真实有效域账户（需先确认）**

#### 步骤 1: 验证您的域账户是否可用

```powershell
# 方法 1: 使用 Windows Credential Manager 验证
control keymgr.dll

# 方法 2: 尝试域账户登录
# 在其他机器上尝试登录 ylw / !Qw2!Qw2!Qw2!Qw2
```

#### 步骤 2: 如果账户确实有效但验证失败

请提供以下信息：
1. 完整错误消息（从浏览器 Console 复制）
2. 截图显示错误位置
3. 确认机器是否在域内（Win+R → sysdm.cpl）

---

### **方案 B: 创建 Mock 测试版本（推荐快速测试 UI）**

如果您只是想测试 UI 功能和 Bug 修复效果，我们可以创建一个**不需要真实域控验证的测试版本**：

#### 修改验证逻辑为 Mock 模式

```rust
// 临时修改 validate_account 函数（仅用于本地测试）
async fn validate_account(Json(req): Json<ValidateAccountRequest>) -> Json<ValidateAccountResponse> {
    let username = req.username.clone();
    
    // Mock 模式：直接验证成功，不实际调用 LSA
    let sid = format!("S-1-5-21-test-{}", chrono::Local::now().timestamp());
    let display_name = username;
    
    Json(ValidateAccountResponse {
        success: true,
        sid,
        display_name,
        message: "验证成功".to_string(),
    })
}
```

#### 启动方式

```powershell
cd management_app\src-tauri
cargo run --release
```

然后使用任意用户名和密码都可以成功验证！

---

## 🎯 **我的建议**

基于当前情况，我建议：

### **立即执行（最快路径）：**

1. **创建本地测试账号**（如果系统有本地管理员账号）
   ```
   Username: localadmin
   Password: YourPassword123!
   ```

2. **或者使用 Mock 模式测试 UI**（无需真实凭证）
   - 我可以帮您临时修改验证代码
   - 测试所有 UI 功能和 Bug 修复
   - 测试完成后恢复原代码

3. **然后再部署到虚拟机进行真实域控测试**

---

## 📋 **请您选择**

请告诉我您希望：

**A**: 继续调试真实的域控验证（需要提供更多信息）  
**B**: 创建 Mock 测试版本快速验证 UI 功能  
**C**: 先部署安装包到虚拟机进行端到端测试  

选择后我会立即为您提供对应的解决方案！🚀

---

## 🔍 **额外信息收集**

如果需要进一步诊断，请运行：

```powershell
# 检查机器域名
hostname && Get-CimInstance Win32_ComputerSystem | Select-Object Domain

# 检查是否有域账户缓存
net user

# 查看 Tauri 应用日志
Get-Content C:\Users\YLW\AppData\Local\WinSLA\logs\*.log -Tail 50
```

请将结果告诉我以便更精准的诊断！
