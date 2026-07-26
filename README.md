# WinSLA - Windows 双账号协同认证登录系统

<p align="center">
  <strong>Windows 系统级双账号登录代理 — 实现"金库双人原则"的安全认证</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11%20x64-blue" alt="Platform" />
  <img src="https://img.shields.io/badge/language-Rust-orange" alt="Language" />
  <img src="https://img.shields.io/badge/version-2.0.0-yellow" alt="Version" />
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
- **域控集成**：通过 LDAP Simple Bind 与 Active Directory 域控制器通信验证
- **原生登录界面**：基于 Windows Credential Provider，在 LogonUI 安全桌面层提供原生双输入 Tile
- **服务桥接**：Windows Service 后台处理 AD 验证，CP 与服务通过 Named Pipe 安全通信
- **应急覆盖**：支持授权管理员在紧急情况下单人登录（需填写原因并记录审计）
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
│  │ (tokio async)│→ │(并行验证)     │  │  应急覆盖             │  │
│  └──────────────┘  └──────┬───────┘  └──────────────────────┘  │
│                    ┌──────┴───────┐                              │
│                    │  AD Bridge   │                              │
│                    │ (LDAP Bind)  │                              │
│                    └──────┬───────┘                              │
└───────────────────────────┼──────────────────────────────────────┘
                            │ LDAP (389/636)
                            ▼
              ┌──────────────────────────┐
              │   Active Directory DC    │
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
| Windows Service | Rust + tokio + windows-service | 异步 Named Pipe 服务端 + 双账号并行验证 |
| AD 认证桥接 | Rust（`ad_bridge`） | LDAP Simple Bind 验证 |
| 管理端后端 | Rust + axum + rusqlite + rust-embed | 内嵌 Web 服务 + SQLite 策略存储 |
| 管理端界面 | wry + tao（WebView2）+ Vue 3.5 + Element Plus + Vite | 原生桌面窗口承载 Vue 前端 |
| 安装部署 | NSIS + PowerShell 脚本 | CP 注册 + 服务安装 + 注册表配置 |

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

从 [GitHub Releases](https://github.com/YanGLweI/WinSLA/releases) 下载 `WinSLA-v2.0.0-Setup.exe`，以管理员身份运行。安装程序自动完成：

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
2. 登录界面出现 WinSLA 双账号 Tile。
3. 分别输入用户 A / 用户 B 的账号与密码，点击提交。
4. 两个账号均通过 AD 验证后进入桌面；任一失败则显示对应错误。

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

当双账号验证不可用时（如一人不在场），授权管理员可触发应急覆盖：选择应急登录 → 输入授权管理员凭据 → 填写应急原因（必填）→ 验证通过后允许单人登录，同时记录审计事件。

---

## 项目结构

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
│       ├── provider_com.rs       # ICredentialProvider 多接口实现
│       ├── credential_com.rs     # ICredentialProviderCredential + 序列化
│       ├── dual_auth_credential.rs  # 双账号凭据状态管理
│       ├── class_factory.rs      # COM 类工厂
│       ├── pipe_client.rs        # Named Pipe 客户端
│       ├── ui_controls.rs        # UI 字段辅助
│       └── com_types.rs          # COM/通信类型
│
├── win_service/                  # Windows Service
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs               # 服务入口
│       ├── service.rs            # 服务注册/安装
│       ├── pipe_server.rs        # tokio Named Pipe 服务端
│       ├── audit.rs              # 审计日志
│       ├── com_types.rs          # 通信协议类型
│       └── auth/
│           ├── mod.rs            # AuthError 定义
│           ├── dual_validator.rs # 双账号并行验证
│           ├── ldap_verifier.rs  # LDAP 验证器
│           ├── sspi_verifier.rs  # SSPI/NTLM 验证器
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
用户 A 输入账号密码 ──┐
                      ├──→ CP 收集凭据 ──→ Named Pipe ──→ Service
用户 B 输入账号密码 ──┘                                    │
                                                          ▼
                                              ┌─────────────────────┐
                                              │  并行 LDAP Bind 验证  │
                                              │  User A → DC        │
                                              │  User B → DC        │
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

---

## 安全设计

- **密码不落盘**：凭据仅在内存中短暂存在，验证后立即清零
- **传输保护**：CP → Service 通信使用 HMAC-SHA256 保护
- **审计追踪**：所有认证事件（成功/失败/应急覆盖）写入日志与数据库
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
# 输出: installer\WinSLA-v2.0.0-Setup.exe
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

- 灵感来源：安当 SLA、ESET Secure Authentication
- 技术参考：Microsoft Credential Provider Samples (Windows SDK)
- 依赖：[windows-rs](https://github.com/microsoft/windows-rs)、[tokio](https://tokio.rs)、[axum](https://github.com/tokio-rs/axum)、[wry](https://github.com/tauri-apps/wry)
