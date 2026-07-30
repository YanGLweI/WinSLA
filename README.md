# WinSLA - Windows 双账号协同认证登录系统

<p align="center">
  <strong>Windows 系统级双账号登录代理 — 实现"金库双人原则"的安全认证</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-blue" alt="Platform" />
  <img src="https://img.shields.io/badge/language-Rust-orange" alt="Language" />
  <img src="https://img.shields.io/badge/version-2.2.0-yellow" alt="Version" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
</p>

---

## 项目简介

WinSLA 是一个 Windows 系统级双账号认证登录代理。在 Active Directory 域环境中，它要求 **两个独立 AD 账号的密码同时验证通过** 方可登录桌面，类似银行金库的"双人原则"（Two-Person Rule）。适用于高安全场景：

- 财务系统终端登录
- 特权管理员操作授权
- 涉密计算机双人管控
- 关键基础设施访问控制

### 核心特性

- **双人双控**：两个不同用户分别输入各自 AD 域密码，两者均验证成功才允许登录
- **真实验证**：通过 Windows `LogonUserW` API 进行真实密码验证（支持域账号/本地账号）
- **原生登录界面**：基于 Windows Credential Provider，在 LogonUI 安全桌面层提供原生双输入 Tile
- **双 Tile 设计**：登录界面同时显示「双控登录」和「应急登录」两个 Tile
- **服务桥接**：Windows Service 后台处理验证，CP 与服务通过 Named Pipe 安全通信
- **失败锁定**：可配置的失败次数阈值与锁定时长，防止暴力破解
- **应急覆盖**：支持授权管理员在紧急情况下单人登录（需填写原因并记录审计）
- **离线缓存**：AD/LDAP网络不可达时可用本地缓存凭据进行应急验证（需预先配置）
- **审计日志**：所有认证事件记录到本地数据库与日志
- **集中管理**：管理端 GUI 提供仪表盘、配对规则、应急账号、审计日志、策略配置

---

## 技术架构

```
┌─────────────────────────────────────────────────────────────────┐
│                Windows LogonUI (Secure Desktop)                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │           DualAuthCP.dll (Credential Provider)             │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │  │
│  │  │User A   │ │Pass A   │ │User B   │ │Pass B   │ [提交]  │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘        │  │
│  └───────────────────────┬───────────────────────────────────┘  │
└──────────────────────────┼──────────────────────────────────────┘
                           │ Named Pipe (\\.\pipe\winsla-auth-pipe)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              winsla-service.exe (Windows Service)                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Pipe Server  │  │Dual Validator│  │  Emergency Override  │  │
│  │ (tokio async)│→ │(LogonUserW)  │  │  应急覆盖             │  │
│  └──────────────┘  └──────┬───────┘  └──────────────────────┘  │
│                    ┌──────┴───────┐                              │
│                    │  AD Bridge   │                              │
│                    │ (LDAP Bind)  │                              │
│                    └──────┬───────┘                              │
└───────────────────────────┼──────────────────────────────────────┘
                            │ LDAP (389/636) / LogonUserW
                            ▼
              ┌──────────────────────────┐
              │   Active Directory DC    │
              │   / 本地 SAM 数据库       │
              └──────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│         winsla-management.exe (管理端 GUI)                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Vue 3.5 UI   │  │ SQLite 存储  │  │  服务控制 / 审计查询  │  │
│  │(Element Plus)│  │ (rusqlite)   │  │  (axum + WebView2)   │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 技术栈

| 组件 | 技术 | 说明 |
|------|------|------|
| Credential Provider | Rust + windows-rs 0.58 | COM DLL（`DualAuthCP.dll`），静态链接 CRT，加载到 LogonUI 安全桌面 |
| Windows Service | Rust + tokio + windows-service | 异步 Named Pipe 服务端 + LogonUserW 真实验证 |
| AD 认证桥接 | Rust（`ad_bridge`） | LDAP Simple Bind 验证（备用） |
| 管理端后端 | Rust + axum + rusqlite + rust-embed | 内嵌 Web 服务 + SQLite 策略存储 |
| 管理端界面 | wry + tao（WebView2）+ Vue 3.5 + Element Plus + Vite | 原生桌面窗口承载 Vue 前端 |
| 安装部署 | NSIS + PowerShell 脚本 | CP 注册 + 服务安装 + 注册表配置 |
| 安全加固 | zeroize | 敏感字段内存零化 |

---

## 快速开始

### 环境要求

- **操作系统**：Windows 10/11 x64（已在 25H2 / Build 26100 验证）
- **Rust**：>= 1.75（stable-x86_64-pc-windows-msvc）
- **Node.js**：>= 20（管理端前端构建）
- **NSIS**：3.x（打包安装程序）
- **域环境**：目标机器已加入 Active Directory 域，网络可达 DC

### 构建

```powershell
git clone https://github.com/YanGLweI/WinSLA.git
cd WinSLA

# 构建所有组件 (Release)
cargo build --release

# 构建管理端前端（输出到 management_app/src-tauri/frontend/dist）
cd management_app
npm install
npm run build
cd ..

# 重新构建以嵌入最新前端资源
cargo build --release
```

构建产物位于 `target/release/`：

| 文件 | 说明 |
|------|------|
| `DualAuthCP.dll` | Credential Provider DLL |
| `winsla-service.exe` | Windows 认证服务 |
| `winsla-management.exe` | 管理端 GUI 应用 |

### 安装部署

#### 方式一：NSIS 安装程序（推荐）

从 [GitHub Releases](https://github.com/YanGLweI/WinSLA/releases) 下载 `WinSLA-v2.0.9-Setup.exe`，以管理员身份运行。安装程序自动完成：

- 复制 DLL/EXE 到 `C:\Program Files\WinSLA`
- 注册 Credential Provider CLSID 到 64 位注册表视图
- 安装并启动 Windows 服务（`WinSLA Service`）
- 创建开始菜单/桌面快捷方式

#### 方式二：PowerShell 脚本

```powershell
# 以管理员身份运行
.\scripts\install.ps1

# 卸载
.\scripts\unregister.ps1
```

> **⚠️ 重要**：Credential Provider 注册后直接影响系统登录流程，请务必在虚拟机中测试，并确保有恢复手段（安全模式 + 删除 CLSID 注册表项，或运行 `scripts\emergency-recovery.ps1`）。

---

## 使用方法

### 双账号登录

1. 安装并重启后，`Win+L` 锁屏或注销。
2. 登录界面出现 WinSLA 双账号 Tile（双控登录）。
3. 分别输入主账号 / 审批人的账号与密码，点击提交。
4. 两个账号均通过验证后进入桌面；任一失败则显示对应错误。
5. 连续失败达到阈值（默认3次）后账号锁定，需等待锁定时长（默认10分钟）后重试。

### 服务管理

```powershell
sc.exe query "WinSLA Service"
net start "WinSLA Service"
net stop "WinSLA Service"

# 独立模式运行（开发调试，无需注册为服务）
.\target\release\winsla-service.exe
```

### 管理端

```powershell
.\target\release\winsla-management.exe
```

管理端功能模块：

| 模块 | 说明 |
|------|------|
| 仪表盘 | 服务状态与系统概览 |
| 配对规则 | 配置哪些用户必须成对登录 |
| 应急账号 | 紧急情况下允许单人登录的授权账号 |
| 审计日志 | 查询所有认证事件 |
| 策略配置 | 认证策略参数设置 |

### 应急覆盖

当双账号验证不可用时（如一人不在场），授权管理员可触发应急覆盖：

1. 在登录界面选择「应急登录」Tile
2. 输入授权管理员凭据
3. 填写应急原因（必填）
4. 验证通过后允许单人登录，同时记录审计事件（`emergency_override`）

应急账号需在管理端「应急账号」模块中预先配置。

### 应急处理方式

#### 场景一：服务进程意外崩溃

如果 `winsla-service.exe` 因异常退出导致登录失败（双控/应急均无法使用）：

1. **尝试重启服务**（无需注销或重启）：
   ```powershell
   # 以管理员身份运行
   net start "WinSLA Service"
   ```

2. **如果服务无法启动**，检查事件查看器 → Windows 日志 → 应用程序中的错误信息。

3. **临时禁用 WinSLA 恢复默认登录**：
   ```powershell
   # 方式一：禁用 Credential Provider
   scripts\emergency-uninstall.ps1
   
   # 方式二：直接删除 CLSID 注册表项
   reg delete "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Provider" /v "{WinSLA-CLSID}" /f
   ```

4. **重启系统**即可使用原始 AD 账号登录。

---

#### 场景二：服务进程卡死或无响应

```powershell
# 强制停止服务
taskkill /F /IM winsla-service.exe

# 等待 CP 缓存刷新（约 5-10 秒），或直接重启资源管理器
restart-appxprovisionedpackage –Online -PackageName Microsoft.Windows.ContentDeliveryPlatform_cw5n1h2txyewy
restart-appxprovisionedpackage –Online -PackageName Microsoft.Win32WebViewHost_cw5n1h2txyewy

# 或在任务管理器中结束 "Windows 资源管理器" 进程
```

---

#### 场景三：AD/LDAP 网络不可达（域控制器离线）

当域控制器不可达时，启用本地离线缓存验证：

1. **管理端策略配置** → 启用「离线缓存"

2. **确保应急账号已提前在离线缓存中预置凭据**（安装后首次登录需在线完成缓存初始化）

3. **登录界面选择「应急登录」**，使用已缓存的应急账号凭证登录。

> ⚠️ **重要提醒**：建议在虚拟机中测试所有应急方案！安装前创建系统快照，确保可以快速回滚。

---

#### 场景四：无法进入桌面（CP 注册导致系统不稳定）

1. **进入安全模式**：
   - 重启电脑，在登录界面出现前按 `F8`（或使用 Windows 恢复环境）
   - 选择「带命令提示符的安全模式」

2. **删除 CLSID 注册表项**：
   ```cmd
   reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Provider" /v "{WinSLA-CLSID}" /t REG_SZ /d "" /f
   ```

3. **或者运行应急卸载脚本**：
   ```cmd
   powershell -ExecutionPolicy Bypass -File scripts\emergency-recovery.ps1
   ```

4. **重启系统**恢复正常登录。

---

#### 场景五：需要完全卸载

```powershell
# PowerShell 脚本卸载
scripts\unregister.ps1

# 或手动卸载
sc.exe delete "WinSLA Service"
Remove-Item "C:\Program Files\WinSLA" -Recurse -Force
reg delete "HKLM\SOFTWARE\WinSLA" /f
```

---

### 调试与诊断

#### 查看服务状态
```powershell
sc.exe query "WinSLA Service"
Get-Service -Name "WinSLA Service"
```

#### 查看 CP 注册状态
```powershell
# 64 位注册表视图
reg query "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Provider" /f "WinSLA" /s
```

#### 查看审计日志
```powershell
# 管理端 → 审计日志模块
# 或直接查询数据库
sqlite3 "%ProgramFiles%\WinSLA\data\winsla.db" "SELECT * FROM audit_log ORDER BY created_at DESC LIMIT 20;"
```

---

### 常见问题

```
WinSLA/
├── Cargo.toml                    # Workspace 根配置
├── README.md
├── ARCHITECTURE.md               # 架构详解
├── LICENSE
├── .gitignore
│
├── cp_provider/                  # Credential Provider DLL
│   ├── Cargo.toml
│   ├── build.rs                  # linker 配置
│   └── src/
│       ├── lib.rs                # DllMain + COM 导出
│       ├── provider_com.rs       # ICredentialProvider 多接口实现（双 Tile）
│       ├── credential_com.rs     # ICredentialProviderCredential + 序列化
│       ├── dual_auth_credential.rs  # 双账号凭据状态管理
│       ├── class_factory.rs      # COM 类工厂
│       ├── pipe_client.rs        # Named Pipe 客户端
│       ├── ui_controls.rs        # UI 字段辅助
│       └── com_types.rs          # COM/通信类型（AuthMode: Dual/Emergency）
│
├── win_service/                  # Windows Service
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # 服务入口
│       ├── service.rs            # 服务注册/安装
│       ├── pipe_server.rs        # tokio Named Pipe 服务端（策略路由）
│       ├── audit.rs              # 审计日志 + login_attempts 锁定表
│       ├── com_types.rs          # 通信协议类型（明文密码+AuthMode）
│       └── auth/
│           ├── mod.rs            # AuthError 定义
│           ├── dual_validator.rs # LogonUserW 真实验证
│           ├── ldap_verifier.rs  # LDAP 验证器（备用）
│           ├── sspi_verifier.rs  # SSPI/NTLM 验证器（备用）
│           └── emergency.rs      # 应急覆盖机制
│
├── ad_bridge/                    # AD/LDAP 认证库
│   └── src/
│       ├── lib.rs                # DomainAuthClient
│       └── ldap.rs               # LDAP Simple Bind 实现
│
├── management_app/               # 管理端
│   ├── package.json              # Vue 3.5 + Element Plus
│   ├── vite.config.ts
│   ├── src/                      # Vue 前端（仪表盘/配对/应急/审计/策略）
│   └── src-tauri/                # Rust 后端（axum + rusqlite + wry/tao）
│       ├── Cargo.toml
│       ├── build.rs              # winres 版本信息嵌入
│       ├── main.rs / lib.rs
│       ├── server.rs             # axum Web 服务
│       ├── database.rs           # SQLite 策略存储
│       ├── commands.rs           # 管理命令
│       ├── frontend.rs           # 内嵌前端资源
│       ├── gui.rs                # WebView2 窗口
│       └── wincred.rs            # Windows 凭据交互
│
├── installer/                    # NSIS 安装程序
│   └── winsla-installer.nsi      # 安装脚本（CP注册+服务+卸载）
│
├── assets/
│   └── winsla.ico
│
└── scripts/                      # 部署/运维脚本
    ├── install.ps1               # 安装
    ├── unregister.ps1            # 卸载
    ├── register-cp.ps1           # CP 注册
    ├── manual-register.ps1       # 手动注册
    ├── diagnose-cp.ps1           # CP 诊断
    ├── troubleshoot-tile.ps1     # Tile 不显示排查
    ├── flush-cp-cache.ps1        # 刷新 CP 缓存
    ├── emergency-recovery.ps1    # 应急恢复
    ├── emergency-uninstall.ps1   # 应急卸载
    ├── create-package.ps1        # 部署包打包
    ├── full-package.ps1          # 完整打包
    ├── build_installer.ps1       # NSIS 构建
    └── auto-deploy.ps1           # 自动部署
```

---

## 认证流程

```
主账号输入账号密码 ──┐
                      ├──→ CP 收集凭据 ──→ Named Pipe ──→ Service
审批人输入账号密码 ──┘                                    │
                                                          ▼
                                              ┌─────────────────────┐
                                              │  并行 LogonUserW 验证 │
                                              │  主账号 → DC/SAM     │
                                              │  审批人 → DC/SAM     │
                                              └──────────┬──────────┘
                                          ┌──────────────┼──────────────┐
                                          ▼              ▼              ▼
                                     两者成功        一方失败        网络超时
                                          │              │              │
                                          ▼              ▼              ▼
                              CredPack 序列化       阻止+提示       降级策略
                              → LSA 登录
```

双账号验证通过后，CP 使用 `CredPackAuthenticationBufferW` 生成 `KERB_INTERACTIVE_LOGON` 序列化缓冲区，经 `GetSerialization` 返回给 LogonUI，由 LSA 完成实际登录。

**支持的账号格式**：
- `DOMAIN\user` - 域账号（NetBIOS 域名）
- `user@domain.suffix` - UPN 格式
- `user` - 本地账号（SAM 数据库）

---

## 安全设计

- **密码不落盘**：凭据仅在内存中短暂存在，验证后立即清零
- **内存零化**：使用 `zeroize::Zeroizing` 包装敏感字段，析构时自动零化内存
- **传输保护**：CP → Service 通信使用本机 Named Pipe（SYSTEM↔SYSTEM 隔离）
- **失败锁定**：可配置的失败次数阈值与锁定时长，防止暴力破解
- **审计追踪**：所有认证事件（成功/失败/应急覆盖/锁定）写入日志与数据库
- **应急管控**：应急覆盖需授权账号 + 填写原因
- **最小权限**：Service 以 LocalSystem 运行，CP 在安全桌面隔离执行
- **静态 CRT**：DLL 静态链接 CRT，避免干净系统缺少 VC++ 运行库导致加载失败

---

## 开发指南

### 构建安装包

```powershell
# 安装 NSIS 3.x
winget install NSIS.NSIS

# 构建 Release 二进制
cargo build --release

# 编译 NSIS 安装程序
& "C:\Program Files (x86)\NSIS\makensis.exe" installer\winsla-installer.nsi
# 输出: installer\WinSLA-v2.0.9-Setup.exe
```

### 测试

```powershell
# 运行全部单元测试
cargo test --workspace

# 按模块运行
cargo test -p cp_provider
cargo test -p winsla-service
cargo test -p ad-bridge
```

> **⚠️ Credential Provider 测试必须在虚拟机中进行**：CP 注册后直接影响登录界面，崩溃或配置错误可能导致无法进入桌面。务必先创建快照，并备好 `scripts\emergency-recovery.ps1` 恢复手段。

### 关键注意事项

- CP 调试需要 VM（崩溃会导致无法进入桌面）
- LDAP 验证需要网络可达域控制器
- 安装/卸载脚本必须以管理员权限运行
- `CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION` 结构体必须严格匹配 SDK 布局（32 字节、无 `cbSize`），否则会导致 LogonUI 崩溃
- `GetSerialization` 返回凭据必须使用 `CPGSR_RETURN_CREDENTIAL_FINISHED = 2`

---

## 许可证

[MIT License](LICENSE)

---

## 致谢

- 技术参考：Microsoft Credential Provider Samples (Windows SDK)
- 依赖：[windows-rs](https://github.com/microsoft/windows-rs)、[tokio](https://tokio.rs)、[axum](https://github.com/tokio-rs/axum)、[wry](https://github.com/tauri-apps/wry)
