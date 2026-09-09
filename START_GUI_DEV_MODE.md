# WinSLA v2.2.6 - 开发环境图形界面测试指南

## 🎯 目标
在开发环境中直接运行完整的图形界面应用进行测试（不使用 Mock，使用真实后端）

---

## 🔧 核心问题诊断

经过检查，发现当前项目的架构是：

### 架构说明
- **前端**: Vite Vue3 应用在 `management_app/` 目录
- **后端 API**: Rust Axum HTTP 服务器（绑定在端口 19830）
- **GUI 窗口**: Rust Wry/TAO WebView2 容器（嵌入前端页面）

### 为什么找不到可执行文件？

Tauri 管理的 `cargo run` **不会自动编译为 GUI 应用**，因为：

1. 这是一个自定义的 Rust + Web 技术栈
2. 需要使用 `main.rs` 作为入口点启动完整的应用程序

---

## ✅ 正确的启动方式

### Step 1: 确保 Vite 前端已运行

打开 PowerShell 终端 1：
```powershell
cd C:\Users\YLW\Documents\PJ\WinSLA\management_app
npm run dev
```

这会启动前端开发服务器在 http://localhost:5174

### Step 2: 以调试模式启动 Rust GUI 应用

**方法 A: 直接在 VSCode 中运行（推荐）**

如果您有 VSCode：
1. 打开整个项目文件夹：`code C:\Users\YLW\Documents\PJ\WinSLA`
2. 按 F5 启动调试（如果配置了 launch.json）
3. 或手动运行：

**方法 B: 命令行启动**

打开新的 PowerShell 窗口（管理员权限可选）：
```powershell
cd C:\Users\YLW\Documents\PJ\WinSLA\management_app\src-tauri
cargo run --release
```

这会编译并运行包含图形界面的完整应用！

---

## 📊 预期结果

成功启动后应该会看到：

1. **Tauri GUI 窗口弹出**
   - 标题："WinSLA Management"
   - 内容：加载 http://localhost:5174 的前端页面
   
2. **导航栏显示**
   - "WinSLA", "仪表盘", "配对规则", "应急账号" 等标签

3. **功能正常**
   - 所有 API 调用通过本地 Rust 后端
   - 数据库读写正常
   - 可以进行完整的功能测试

---

## 🔍 故障排查

### 问题 1: cargo run 没有弹窗窗口

**原因**: 可能编译失败或被错误处理

**解决步骤**:
```powershell
# 清理旧编译
cargo clean

# 重新构建并查看详细输出
cd management_app\src-tauri
cargo build --release --verbose

# 成功后找到生成的 exe
dir target\release\winsla-management.exe
```

### 问题 2: 找不到 wry/tao 依赖

**解决**:
```powershell
# 安装缺失的系统依赖
choco install visualstudio2022workload-nativedesktopdev -y
# 或手动安装 Visual Studio Build Tools
```

### 问题 3: WebView2 未安装

**检查**:
```powershell
# 检查 WebView2 是否安装
Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00EC333A9E6E}" | Select-Object pv
```

如果没有安装，从 Microsoft 官网下载：https://developer.microsoft.com/en-us/microsoft-edge/webview2/

---

## 🚀 最简单的启动脚本

创建一个快速启动脚本来自动化整个过程：

**management_app\start-gui-test.ps1**:
```powershell
Write-Host "=== Starting WinSLA Development GUI Application ===" -ForegroundColor Cyan

# Step 1: Start Vite frontend (if not running)
$viteProcess = Get-Process node -ErrorAction SilentlyContinue | Where-Object {
    $_.CommandLine -like "*vite*" -and $_.CommandLine -like "*5174*"
}

if (-not $viteProcess) {
    Write-Host "Starting Vite frontend on port 5174..." -ForegroundColor Yellow
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd management_app; npm run dev" -PassThru
    Start-Sleep -Seconds 5
} else {
    Write-Host "Vite frontend already running on 5174" -ForegroundColor Green
}

# Step 2: Build and run GUI application
Write-Host "`nBuilding GUI application..." -ForegroundColor Yellow
Set-Location management_app\src-tauri
Start-Sleep -Seconds 1
cargo build --release --no-default-features --features embedded-webview2 2>&1 | Select-String "Finished"

# Step 3: Launch the application
if (Test-Path target\release\winsla-management.exe) {
    Write-Host "`n✅ GUI application ready!" -ForegroundColor Green
    Write-Host "Launching application in 3 seconds... Press Ctrl+C to abort" -ForegroundColor Gray
    Start-Sleep -Seconds 3
    
    Start-Process "target\release\winsla-management.exe"
    Write-Host "`n🎉 Application launched! Test at http://localhost:5174/#/pairs" -ForegroundColor Magenta
} else {
    Write-Host "`n❌ Build failed or executable not found" -ForegroundColor Red
    Write-Host "Please check the errors above." -ForegroundColor Gray
}

Write-Host "`nPress any key to exit this script..." -ForegroundColor DarkGray
$null = $host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
```

使用方式：
```powershell
cd management_app
.\start-gui-test.ps1
```

---

## 📋 测试流程（与之前相同）

启动 GUI 应用后，按照以下顺序测试：

1. **TC01**: 页面加载验证（无 Console 错误）
2. **TC02**: 创建主账号 + 同时添加审批人（关键！）
3. **TC03**: JSON 解析验证（刷新页面无 filter is not a function）
4. **TC04**: HTTP 200 开关验证（Network 标签查看状态码）
5. **TC05-07**: 其他功能验证

---

## 🎯 总结

**核心要点**:

| 项目 | 解决方案 |
|------|---------|
| 前端服务器 | `npm run dev` (已在运行) |
| 后端 API | 包含在 GUI 应用中 |
| GUI 窗口 | `cargo run --release` |
| 访问地址 | GUI 窗口自动加载 http://localhost:5174 |
| 数据库 | SQLite 在 C:\ProgramData\WinSLA\winsla.db |
| 域控连接 | 使用真实 AD 验证（无需 Mock） |

**立即开始**:
```powershell
# 打开新窗口运行 GUI
cd management_app\src-tauri
cargo run --release
```

等待 GUI 窗口出现，然后开始测试！有任何问题请随时告诉我！🚀
