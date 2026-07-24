# WinSLA - Windows Dual-Account Authentication System

<p align="center">
  <strong>Windows 双账号协同登录代理 — 实现"金库双人原则"的系统级安全认证</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11%20%7C%20Server%202016%2B-blue" alt="Platform" />
  <img src="https://img.shields.io/badge/language-Rust-orange" alt="Language" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
  <img src="https://img.shields.io/badge/version-0.0.1-yellow" alt="Version" />
</p>

---

## 项目简介

WinSLA 是一个 Windows 系统级双账号认证登录代理，在 Active Directory 域控环境中要求 **两个独立 AD 账号的密码同时验证通过** 方可登录桌面。类似银行金库的"双人原则"（Two-Person Rule），适用于高安全场景：

- 财务系统终端登录
-  privileged 管理员操作授权
- 涉密计算机双人管控
- 关键基础设施访问控制

### 核心特性

- **双人双控**：两个不同用户分别输入各自 AD 域密码，两者均验证成功才允许登录
- **域控集成**：通过 LDAP Simple Bind 直接与 Active Directory 域控制器通信验证
- **Credential Provider**：在 Windows LogonUI 安全桌面层实现原生双输入界面
- **服务桥接**：Windows Service 后台处理 AD 验证，Named Pipe 安全通信
- **应急覆盖**：支持授权管理员在紧急情况下单人登录（需审批+审计）
- **审计日志**：所有认证事件记录到本地文件和 Windows Event Log
- **集中管理**：管理端提供配对规则配置、策略管理、日志查询

---

## 技术架构

```
┌─────────────────────────────────────────────────────────────────┐
│                    Windows LogonUI (Secure Desktop)               │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │           DualAuthCP.dll (Credential Provider)             │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐        │  │
│  │  │User A   │ │Pass A   │ │User B   │ │Pass B   │ [Submit]│  │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘        │  │
│  └───────────────────────┬───────────────────────────────────┘  │
└──────────────────────────┼──────────────────────────────────────┘
                           │ Named Pipe (\\.\pipe\winsla-auth-pipe)
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│              winsla-service.exe (Windows Service)                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Pipe Server  │  │Dual Validator│  │  Emergency Override  │  │
│  │ (tokio async)│→ │(parallel auth)│  │  Manager             │  │
│  └──────────────┘  └──────┬───────┘  └──────────────────────┘  │
│                           │                                      │
│                    ┌──────┴───────┐                              │
│                    │  AD Bridge   │                              │
│                    │ (LDAP Bind)  │                              │
│                    └──────┬───────┘                              │
└───────────────────────────┼──────────────────────────────────────┘
                            │ LDAP (port 389/636)
                            ▼
              ┌──────────────────────────┐
              │   Active Directory DC    │
              └──────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│         winsla-management.exe (Management App)                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ Vue 3.5 UI   │  │ SQLite Store │  │  Service Control     │  │
│  │(Element Plus)│  │(Policy/Pair) │  │  (start/stop/status) │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### 技术栈

| 组件 | 技术 | 说明 |
|------|------|------|
| Credential Provider | Rust + windows-rs 0.58 | COM DLL，加载到 LogonUI 安全桌面 |
| Windows Service | Rust + tokio + windows-service | 异步 Named Pipe 服务端 + AD 验证 |
| AD 认证桥接 | Rust + ldap3 | LDAP Simple Bind，多 DC 容错 |
| 管理端后端 | Rust + rusqlite | SQLite 策略存储 + 命令接口 |
| 管理端前端 | Vue 3.5 + Element Plus + Vite | 配置界面（预留 Tauri 2.0 集成） |
| 安装部署 | PowerShell 脚本 | CP 注册 + 服务安装 + 注册表配置 |

---

## 快速开始

### 环境要求

- **操作系统**: Windows 10/11 Enterprise 或 Windows Server 2016+
- **Rust**: >= 1.75 (stable-x86_64-pc-windows-msvc)
- **Node.js**: >= 20 (管理端前端开发)
- **域环境**: 已加入 Active Directory 域，网络可达 DC

### 构建

```powershell
# 克隆仓库
git clone https://github.com/YanGLweI/WinSLA.git
cd WinSLA

# 构建所有组件 (Release)
cargo build --release --workspace

# 运行测试
cargo test --workspace
```

构建产物位于 `target/release/`：

| 文件 | 说明 |
|------|------|
| `DualAuthCP.dll` | Credential Provider DLL |
| `winsla-service.exe` | Windows 认证服务 |
| `winsla-management.exe` | 管理端应用 |

### 安装部署

#### 方式一：NSIS 安装程序（推荐）

从 [GitHub Releases](https://github.com/YanGLweI/WinSLA/releases) 下载 `WinSLA-v0.0.1-Setup.exe`，以管理员身份运行：

1. 确认安全警告对话框
2. 选择安装目录（默认 `%SystemRoot%\System32\winsla\`）
3. 安装程序自动完成：
   - 复制 DLL/EXE 到安装目录
   - 注册 Credential Provider CLSID 到注册表
   - 安装并启动 Windows Service
   - 创建开始菜单快捷方式

#### 方式二：PowerShell 脚本（手动）

```powershell
# 以管理员身份运行
.\scripts\install.ps1

# 自定义安装路径
.\scripts\install.ps1 -InstallPath "D:\WinSLA"

# 卸载
.\scripts\unregister.ps1
```

#### 方式三：完全手动

```powershell
# 1. 复制文件
mkdir $env:SystemRoot\System32\winsla
copy target\release\DualAuthCP.dll $env:SystemRoot\System32\winsla\
copy target\release\winsla-service.exe $env:SystemRoot\System32\winsla\

# 2. 注册 Credential Provider (CLSID)
reg add "HKLM\SOFTWARE\Classes\CLSID\{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}" /ve /d "WinSLA Dual-Auth CP" /f
reg add "HKLM\SOFTWARE\Classes\CLSID\{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}\InprocServer32" /ve /d "$env:SystemRoot\System32\winsla\DualAuthCP.dll" /f
reg add "HKLM\SOFTWARE\Classes\CLSID\{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}\InprocServer32" /v ThreadingModel /d Apartment /f
reg add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}" /ve /d "WinSLA Dual-Auth" /f

# 3. 安装并启动服务
sc.exe create "WinSLA Service" binPath="$env:SystemRoot\System32\winsla\winsla-service.exe --service" start=auto
net start "WinSLA Service"
```

---

## 使用方法

### 服务管理

```powershell
# 查看服务状态
sc.exe query "WinSLA Service"

# 启动/停止/重启服务
net start "WinSLA Service"
net stop "WinSLA Service"

# 独立模式运行（开发调试用，无需注册为服务）
.\target\release\winsla-service.exe
# 输出: Starting pipe server in standalone mode...
# 监听: \\.\pipe\winsla-auth-pipe

# 设置日志级别
$env:RUST_LOG = "debug"
.\target\release\winsla-service.exe
```

### Named Pipe 通信协议

CP 与 Service 之间通过 Named Pipe 通信，协议格式：

```
┌────────────────────────────────────────────────┐
│  请求: [4 bytes length LE] [JSON payload]       │
│  响应: [4 bytes length LE] [JSON payload]       │
└────────────────────────────────────────────────┘
```

**请求体 (AuthRequest):**
```json
{
  "request_id": "uuid-v4",
  "user_a_username": "DOMAIN\\userA",
  "user_a_password_hash": [/* HMAC-SHA256 bytes */],
  "user_b_username": "DOMAIN\\userB",
  "user_b_password_hash": [/* HMAC-SHA256 bytes */],
  "timestamp": "2026-01-01T00:00:00Z"
}
```

**响应体 (AuthResponse):**
```json
"Success"                                    // 双账号验证通过
{"FailUserA": "Invalid credentials"}         // 用户 A 验证失败
{"FailUserB": "Account locked"}              // 用户 B 验证失败
{"BothFailed": ["err_a", "err_b"]}           // 两者均失败
"Timeout"                                    // 验证超时
"NetworkUnavailable"                         // 无法连接 DC
```

### 管理端

```powershell
# 运行管理端
.\target\release\winsla-management.exe
```

管理端功能：
- 配对规则管理（哪些用户必须成对登录）
- 应急账号配置（紧急情况下允许单人登录）
- 审计日志查询
- 策略参数设置

### 应急覆盖

当双账号验证不可用时（如一人不在场），授权管理员可触发应急覆盖：

1. 在 CP 界面选择"应急登录"
2. 输入授权管理员凭据
3. 填写应急原因（必填）
4. 系统验证管理员 SID 是否在授权列表中
5. 通过后允许单人登录，同时记录审计事件

---

## 测试

### 单元测试

```powershell
# 运行全部测试 (32 tests)
cargo test --workspace

# 按模块运行
cargo test -p cp_provider       # 9 tests: 状态机、GUID、序列化
cargo test -p winsla-service    # 10 tests: 验证逻辑、应急覆盖、审计
cargo test -p ad-bridge         # 7 tests: LDAP DN构建、多DC容错
cargo test -p winsla-management # 6 tests: SQLite CRUD、策略配置

# 显示测试输出
cargo test --workspace -- --nocapture
```

### 集成测试：Named Pipe 通信

无需域环境即可测试 CP ↔ Service 通信链路：

```powershell
# 终端 1: 启动服务（独立模式）
$env:RUST_LOG = "debug"
.\target\release\winsla-service.exe

# 终端 2: 使用 PowerShell 发送测试请求
$pipe = New-Object System.IO.Pipes.NamedPipeClientStream('.', 'winsla-auth-pipe', 'InOut')
$pipe.Connect(5000)

# 构造测试请求
$request = @{
    request_id = [guid]::NewGuid().ToString()
    user_a_username = "TESTDOMAIN\userA"
    user_a_password_hash = [byte[]]@(1,2,3,4)
    user_b_username = "TESTDOMAIN\userB"
    user_b_password_hash = [byte[]]@(5,6,7,8)
    timestamp = (Get-Date).ToUniversalTime().ToString("o")
} | ConvertTo-Json

$bytes = [System.Text.Encoding]::UTF8.GetBytes($request)
$lenBytes = [BitConverter]::GetBytes([int]$bytes.Length)

# 发送
$pipe.Write($lenBytes, 0, 4)
$pipe.Write($bytes, 0, $bytes.Length)
$pipe.Flush()

# 接收响应
$respLen = New-Object byte[] 4
$pipe.Read($respLen, 0, 4) | Out-Null
$respBuf = New-Object byte[] ([BitConverter]::ToInt32($respLen, 0))
$pipe.Read($respBuf, 0, $respBuf.Length) | Out-Null
[System.Text.Encoding]::UTF8.GetString($respBuf)

$pipe.Close()
```

### 集成测试：LDAP 认证（需域环境）

```powershell
# 确保网络可达域控 (默认端口 389)
Test-NetConnection -ComputerName dc01.yourdomain.com -Port 389

# 使用 ad_bridge 库测试 LDAP Bind
# 在 Rust 测试中:
cargo test -p ad-bridge -- --nocapture
# 或编写集成测试:
```

```rust
// tests/ldap_integration.rs (需域环境)
#[test]
#[ignore] // 仅在域环境中手动运行: cargo test -- --ignored
fn test_ldap_bind_real_dc() {
    let client = ad_bridge::DomainAuthClient::new(
        "ldap://dc01.yourdomain.com",
        "YOURDOMAIN.COM",
    );
    let result = client.verify_credentials("testuser", "testpass");
    assert!(result.is_ok() || result.is_err()); // 不应 panic
}
```

### Credential Provider 测试（需虚拟机）

> **⚠️ 重要**: CP 注册后直接影响登录界面。务必在 Hyper-V/VMware 虚拟机中测试！

**测试环境准备：**

1. 创建 Windows 10/11 虚拟机
2. 加入测试域（或搭建独立 AD DC）
3. 创建快照（用于快速恢复）

**测试步骤：**

```powershell
# 1. 在 VM 中构建并安装
cargo build --release
.\scripts\install.ps1

# 2. 重启或注销触发 LogonUI 重新加载 CP
# 方法 A: 注销当前用户
# 方法 B: 重启 LogonUI (管理员 CMD)
taskkill /f /im LogonUI.exe

# 3. 在登录界面验证:
#    - 应出现 WinSLA 双账号输入面板
#    - 输入两组凭据后点击提交
#    - 观察验证结果
```

**故障恢复（如果 CP 导致无法登录）：**

```powershell
# 方法 1: 安全模式
# 重启 → F8/Shift+重启 → 安全模式 → 管理员 CMD:
reg delete "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}" /f

# 方法 2: 从另一管理员账户
reg delete "HKLM\SOFTWARE\Classes\CLSID\{A5A5A5A5-B6B6-C7C7-D8D8-E9E9E9E9E9E9}" /f

# 方法 3: 使用安装程序卸载
.\WinSLA-v0.0.1-Setup.exe  # 选择卸载
```

### 测试矩阵

| 测试类型 | 环境要求 | 命令/方法 |
|----------|----------|-----------|
| 单元测试 | 无 | `cargo test --workspace` |
| Pipe 通信 | 无域 | 独立模式 + PowerShell 脚本 |
| LDAP 认证 | 域环境 | `cargo test -p ad-bridge -- --ignored` |
| CP 界面 | VM + 域 | 安装后注销/重启 |
| 应急覆盖 | 无域 | `cargo test -p winsla-service emergency` |
| 数据库 CRUD | 无 | `cargo test -p winsla-management` |
| 安装/卸载 | 管理员 VM | NSIS Setup 或 install.ps1 |

---

## 项目结构

```
WinSLA/
├── Cargo.toml                    # Workspace 根配置
├── README.md
├── LICENSE
├── .gitignore
│
├── cp_provider/                  # Credential Provider DLL
│   ├── Cargo.toml
│   ├── build.rs                  # Windows linker 配置
│   └── src/
│       ├── lib.rs                # DllMain + COM 导出函数
│       ├── dual_auth_credential.rs  # 双账号凭据状态管理
│       ├── credential_provider.rs   # 凭据提交逻辑
│       ├── pipe_client.rs        # Named Pipe 客户端
│       ├── com_types.rs          # 通信协议类型
│       ├── ui_controls.rs        # UI 字段辅助
│       └── main.rs               # 测试入口
│
├── win_service/                  # Windows Service
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # 服务入口 + ServiceState
│       ├── pipe_server.rs        # tokio Named Pipe 服务端
│       ├── com_types.rs          # 通信协议类型
│       ├── audit.rs              # 审计日志模块
│       └── auth/
│           ├── mod.rs            # AuthError 定义
│           ├── dual_validator.rs # 双账号并行验证
│           ├── ldap_verifier.rs  # LDAP 验证器
│           ├── sspi_verifier.rs  # SSPI/NTLM 验证器
│           └── emergency.rs      # 应急覆盖机制
│
├── ad_bridge/                    # AD/LDAP 认证库
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                # DomainAuthClient
│       └── ldap.rs               # LDAP Simple Bind 实现
│
├── management_app/               # 管理端
│   ├── package.json              # Vue 3.5 + Element Plus
│   ├── vite.config.ts
│   ├── index.html
│   ├── App.vue
│   ├── main.ts
│   ├── tauri.conf.json           # Tauri 2.0 配置（预留）
│   └── src-tauri/
│       ├── Cargo.toml
│       ├── lib.rs                # 应用配置
│       ├── main.rs               # 入口
│       ├── commands.rs           # 管理命令
│       └── database.rs           # SQLite 策略存储
│
├── installer/                    # NSIS 安装程序
│   └── winsla-installer.nsi     # 安装脚本 (CP注册+服务+卸载)
│
└── scripts/                      # 部署脚本
    ├── install.ps1               # 安装（CP注册+服务安装）
    └── unregister.ps1            # 卸载
```

---

## 认证流程

```
用户 A 输入账号密码 ──┐
                      ├──→ CP 收集凭据 ──→ Named Pipe ──→ Service
用户 B 输入账号密码 ──┘                                    │
                                                          ▼
                                              ┌─────────────────────┐
                                              │  并行 LDAP Bind 验证  │
                                              │  User A → DC        │
                                              │  User B → DC        │
                                              └──────────┬──────────┘
                                                         │
                                          ┌──────────────┼──────────────┐
                                          ▼              ▼              ▼
                                     两者成功        一方失败        网络超时
                                          │              │              │
                                          ▼              ▼              ▼
                                     放行登录       阻止+提示      降级策略
```

---

## 安全设计

- **密码不落盘**：凭据仅在内存中短暂存在，验证后立即清零
- **传输保护**：CP → Service 通信使用 SHA-256 哈希 + 盐值
- **审计追踪**：所有认证事件（成功/失败/应急覆盖）写入日志
- **应急管控**：应急覆盖需授权账号 + 填写原因 + 限时生效
- **最小权限**：Service 以 LocalSystem 运行，CP 在安全桌面隔离执行

---

## 路线图

- [x] **v0.0.1** — 基础架构 POC（当前版本）
  - CP DLL 框架 + COM 导出
  - Service Named Pipe 通信
  - LDAP 认证（同步，多 DC 容错）
  - SQLite 策略存储
  - 应急覆盖机制
  - 审计日志
- [ ] **v0.1.0** — 完整 COM vtable 实现
  - ICredentialProvider 完整接口
  - ICredentialProviderCredential 字段交互
  - LogonUI 实际渲染测试
- [ ] **v0.2.0** — 管理端完善
  - Tauri 2.0 集成
  - Vue 前端完整界面
  - 域用户搜索和配对
- [ ] **v0.3.0** — 安全加固
  - EV 代码签名
  - DPAPI 密钥保护
  - 离线缓存降级
- [ ] **v1.0.0** — 在线版
  - 集中式策略服务器
  - HTTPS + mTLS 同步
  - Web Admin Portal

---

## 开发指南

### 构建安装包

```powershell
# 安装 NSIS 3.x (https://nsis.sourceforge.io)
winget install NSIS.NSIS

# 构建 Release 二进制
cargo build --release --workspace

# 编译 NSIS 安装程序
& "C:\Program Files (x86)\NSIS\makensis.exe" installer\winsla-installer.nsi
# 输出: installer\WinSLA-v0.0.1-Setup.exe
```

### 注意事项

> **警告**: Credential Provider 注册后会影响系统登录流程。请始终在虚拟机中测试，并确保有恢复手段（安全模式 + 注册表删除 CLSID）。

- CP 调试需要 VM + 内核调试器（崩溃会导致无法进入桌面）
- LDAP 验证需要网络可达域控制器
- 安装/卸载脚本必须以管理员权限运行

---

## 许可证

[MIT License](LICENSE)

---

## 致谢

- 灵感来源：安当 SLA、ESET Secure Authentication
- 技术参考：Microsoft Credential Provider Samples (Windows SDK)
- 依赖：[windows-rs](https://github.com/microsoft/windows-rs)、[tokio](https://tokio.rs)、[ldap3](https://github.com/inejge/ldap3)
