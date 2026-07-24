# WinSLA 部署与测试指南

## 🚀 本地开发环境搭建

### 1. 系统要求检查
```powershell
# 检查 Rust 版本
rustc --version  # 需要 >= 1.75

# 检查 .NET (用于某些工具)
dotnet --version

# 检查 Node.js
node --version  # 推荐 v20+
npm --version
```

### 2. 安装依赖

#### Rust 组件
```powershell
# 安装 MSVC 工具链（如果还没有）
rustup default stable-x86_64-pc-windows-msvc
rustup component add rust-src
```

#### Node 管理端依赖
```bash
cd management_app
npm install
```

### 3. 编译项目

```powershell
# 从 workspace 根目录编译所有 Rust 组件
cargo build --release --workspace
```

这将生成三个可执行文件：
- `target\release\DualAuthCP.dll`
- `target\release\winsla-service.exe`
- `target\release\ad_bridge.dll` (可选库)

## 🔧 测试环境设置

### 方案 A：使用 Hyper-V 虚拟域环境（推荐）

```powershell
# 1. 启用 Hyper-V（需要在 BIOS/UEFI 开启虚拟化）
Enable-WindowsOptionalFeature -Online -FeatureName Microsoft-Hyper-V -All

# 2. 创建 Windows Server 虚拟机
New-VM -Name "WinServerDC" -MemoryStartupBytes 4GB

# 3. 安装 AD DS 角色并提升为 Domain Controller
Install-WindowsFeature AD-Domain-Services -IncludeManagementTools

# 4. 创建测试用户
Import-Module ActiveDirectory
New-ADUser -Name "TestUserA" -AccountPassword (ConvertTo-SecureString "Pass123!" -AsPlainText -Force) -Enabled $true
New-ADUser -Name "TestUserB" -AccountPassword (ConvertTo-SecureString "Pass123!" -AsPlainText -Force) -Enabled $true

# 5. 将客户端机器加入域
Add-Computer -DomainName "example.com" -Credential (Get-Credential)
```

### 方案 B：使用现有企业环境

只需确保：
1. 有可用的 Domain Controller
2. 有两个测试账户
3. DNS 解析正常

## 📦 安装流程

### Step 1: 编译 Release 版本
```powershell
cargo build --release --workspace
```

### Step 2: 以管理员运行安装脚本
```powershell
Set-Location c:\Users\YLW\Documents\PJ\WinSLA
Start-Process powershell -ArgumentList "Set-Location '$PWD'; & '.\scripts\install.ps1'" -Verb RunAs
```

### Step 3: 验证服务状态
```powershell
# 查询服务
sc query "WinSLA Service"

# 查看 Event Viewer
eventvwr.msc
# 导航到：Windows Logs → Application
```

## 🧪 功能测试

### 测试 1: 服务运行状态
```powershell
# PowerShell
$service = Get-Service "WinSLA Service" -ErrorAction SilentlyContinue
if ($service) {
    Write-Host "Service found: $($service.Status)"
} else {
    Write-Host "Service not installed"
}
```

### 测试 2: 命名管道通信
```rust
// ad_bridge/tests/pipe_test.rs
#[tokio::test]
async fn test_named_pipe_server() {
    use tokio::net::windows::named_pipe::ClientOptions;
    
    // Start server in background
    let handle = tokio::spawn(async move {
        // Server code here
    });
    
    // Client connection
    let client = ClientOptions::new()
        .open(r"\\.\pipe\winsla-auth-pipe")
        .await?;
    
    assert!(client.is_connected());
    handle.abort();
}
```

### 测试 3: LDAP 认证模拟
```bash
# 手动测试 LDAPS 连接
ldapsearch -x -H ldap://dc.example.com -D "CN=admin,DC=example,DC=com" -W -b "DC=example,DC=com" "(sAMAccountName=testusera)"
```

### 测试 4: 完整登录流程（VM 中测试）

**警告：此操作会影响当前用户会话！**

1. **注销当前用户**
   ```powershell
   shutdown /l
   ```

2. **观察 LogonUI**
   - 应该看到两个输入组
   - User A + Password + User B + Password

3. **输入测试凭据**
   ```
   User A: DOMAIN\TestUserA
   Pass A: Pass123!
   
   User B: DOMAIN\TestUserB  
   Pass B: Pass123!
   ```

4. **检查结果**
   - 成功：自动登录桌面
   - 失败：显示错误消息并阻止登录

## 🔍 调试技巧

### Credential Provider 调试
```rust
// cp_provider/src/lib.rs
#[cfg(debug_assertions)]
fn init_com() -> Result<(), anyhow::Error> {
    use std::io::Write;
    
    let mut file = std::fs::File::create("C:\\temp\\cp_debug.log")?;
    writeln!(file, "CP loaded at {}", chrono::Utc::now())?;
    Ok(())
}
```

### 使用 WinDbg 分析崩溃
```powershell
# 启动 WinDbg Preview
winbgui.exe

# Attach to LogonUI process
Debug → Attach to Process → logonui.exe

# 设置符号服务器
.debug.sympath SRV*C:\symbols*https://msdl.microsoft.com/download/symbols
.reload /f C:\Path\To\DualAuthCP.dll

# 重新触发登录，捕获异常
```

### 日志收集
```powershell
# 获取服务日志
Get-EventLog -LogName Application -Source "WinSLA Service" -Last 100

# 实时监听事件
Get-EventLog -LogName Application -Source "WinSLA Service" -Follow
```

## ⚠️ 常见问题及解决方案

### 问题 1: Credential Provider 无法加载

**症状**: LogonUI 未显示自定义 UI

**排查步骤**:
```powershell
# 1. 检查 DLL 签名
signtool verify /pa DualAuthCP.dll

# 2. 检查注册表项
reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Auth\LogonProviders"

# 3. 验证 CLSID 匹配
# 期望值：{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}
```

**解决方案**:
- 重新运行 install.ps1
- 重启计算机（或至少重启 LogonUI 进程）

### 问题 2: Service 无法启动

**症状**: 服务状态为 STOPPED，错误代码 1067

**排查**:
```powershell
# 查看详细错误
wevtutil qe System /c:10 /q:"*[System[Provider[@Name='WinSLA Service']]]" /f:text

# 手动启动服务调试
# 修改注册表添加 /debug 参数
Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\WinSLA Service" `
                 -Name "ImagePath" -Value "\"C:\Windows\System32\winsla\winsla-service.exe\" /debug"
```

**解决方案**:
- 检查端口占用（Named Pipe 路径冲突）
- 查看 Event Viewer 日志
- 确保有足够的权限访问 Named Pipe

### 问题 3: LDAP 验证超时

**症状**: 验证卡住或超时

**排查**:
```powershell
# 测试 DC 连通性
nslookup dc.example.com
telnet dc.example.com 636  # 或使用 Test-NetConnection

# 测试 LDAP bind
ldapwhoami -x -H ldaps://dc.example.com -D "cn=admin..." -W
```

**解决方案**:
- 检查防火墙规则
- 确保 LDAPS 证书有效
- 验证 DNS 解析

## 📊 性能基准测试

### 验证耗时测量
```rust
use std::time::Instant;

let start = Instant::now();
let result = validate_dual_accounts(...).await;
let duration = start.elapsed();

log::info!("Total validation time: {:?}", duration);
```

**目标指标**:
- 单次 LDAP 绑定：< 500ms
- 双账号并行验证：< 1s（理想），< 2s（可接受）
- 总登录延迟：< 3s

## 🔄 回滚方案

如果遇到问题影响登录：

```powershell
# 以安全模式启动或使用紧急恢复介质
.\scripts\unregister.ps1

# 或手动卸载
sc delete "WinSLA Service"
Remove-Item "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Auth\LogonProviders\E4D9F6E7..."
```

## 🎯 自动化测试框架建议

### 单元测试
```bash
cargo test --workspace
```

### E2E 测试
```python
#!/usr/bin/env python3
# tests/e2e/login_test.py

def test_dual_login_flow():
    # 1. Start LogonUI with CP
    # 2. Enter credentials via UI automation
    # 3. Verify login success/failure
    # 4. Check audit logs
```

---

**注意**: 在生产环境部署前，务必在隔离的 VM 环境中进行充分测试！
