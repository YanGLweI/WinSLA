# WinSLA 系统架构详解

## 🎯 设计目标

构建一个 Windows 系统级双账号协同认证登录代理（Dual-Account Authentication Provider），实现类似银行金库"双人原则"的安全机制：两个不同用户必须分别输入各自的 AD 域密码，两者均验证成功后方可完成登录。

## 📐 整体架构

```
┌───────────────────────────────────────────────────────────────────┐
│                         USER SPACE (Normal Session)                │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │              Management App (Tauri + Vue 3.5)                │ │
│  │                                                              │ │
│  │  - Service Status Monitor                                    │ │
│  │  - Dual-Pair Policy Config                                   │ │
│  │  - Audit Log Viewer                                          │ │
│  │  - Emergency Override Handler                                │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                    │
└───────────────────────────────────────────────────────────────────┘
         │                                    │
         │ HTTP/WS                            │ RPC/Named Pipe
         ▼                                    ▼
┌───────────────────────────────────────────────────────────────────┐
│                         SYSTEM SERVICE (SYSTEM Context)            │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │           WinSLA Service (winsla-service.exe)                │ │
│  │                                                              │ │
│  │  Named Pipe Server: \\.\pipe\winsla-auth-pipe               │ │
│  │                                                              │ │
│  │  ┌──────────────────────────────────────────────────────┐  │ │
│  │  │           Authentication Module                       │  │ │
│  │  │                                                      │  │ │
│  │  │  • DualValidator (并行验证两账号)                      │  │ │
│  │  │  • LDAP Verifier (Simple Bind, Port 389/636)         │  │ │
│  │  │  • SSPI Verifier (NTLM/Kerberos fallback)            │  │ │
│  │  └────────────────────────────┬─────────────────────────┘  │ │
│  │                               │                              │ │
│  │  ┌────────────────────────────┴─────────────────────────┐  │ │
│  │  │           Audit Logger                                │  │ │
│  │  │  • Write to Windows Event Log                         │  │ │
│  │  │  • Local SQLite DB (encrypted templates)              │  │ │
│  │  └───────────────────────────────────────────────────────┘  │ │
│  └─────────────────────────────────────────────────────────────┘ │
│         │                                    │                    │
│    LDAP/LDAPS                           Kerberos                  │
│         ▼                                    ▼                    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                 DOMAIN CONTROLLER (Active Directory)         │ │
│  │                                                              │ │
│  │  • User Validation (Bind with DN+Password)                  │ │
│  │  • Account Status Check (Locked, Expired)                   │ │
│  │  • SID Lookup                                               │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                    │
└───────────────────────────────────────────────────────────────────┘
         │                                    │
         │ NTLM/Kerberos Tickets              │ Group Membership      │
         ▼                                    ▼                        │
┌───────────────────────────────────────────────────────────────────┐
│                     SECURE DESKTOP (LogonUI Process)               │
│                                                                    │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │          Credential Provider (DualAuthCP.dll)                │ │
│  │  • COM CLSID: {E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}       │ │
│  │  • Loaded by: LogonUI.exe (via LSASS)                       │ │
│  │                                                               │ │
│  │  ┌──────────────────────────────────────────────────────┐   │ │
│  │  │          UI Layer                                      │   │ │
│  │  │                                                        │   │ │
│  │  │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │   │ │
│  │  │  │User A     │  │Pass A    │  │Submit    │           │   │ │
│  │  │  │[EDIT]    │  │[PASSWORD]│  │ [BUTTON] │           │   │ │
│  │  │  └──────────┘  └──────────┘  └──────────┘           │   │ │
│  │  │                                                        │   │ │
│  │  │  ┌──────────┐  ┌──────────┐                          │   │ │
│  │  │  │User B     │  │Pass B    │                          │   │ │
│  │  │  │[EDIT]    │  │[PASSWORD]│                          │   │ │
│  │  │  └──────────┘  └──────────┘                          │   │ │
│  │  └───────────────────────────────────────────────────────┘   │ │
│  │                                                               │ │
│  │  ┌──────────────────────────────────────────────────────┐   │ │
│  │  │          IPC Client                                   │   │ │
│  │  │  • Named Pipe Client to winsla-service               │   │ │
│  │  │  • AuthRequest → Service → AuthResponse              │   │ │
│  │  │  • Timeout handling (default: 30s)                   │   │ │
│  │  └───────────────────────────────────────────────────────┘   │ │
│  └─────────────────────────────────────────────────────────────┘ │
│        │                                                             │
│  ┌─────┴────────────────────────────────────────────────────┐      │
│  │                    LSA (Local Security Authority)         │      │
│  │  • Creates logon session                                  │      │
│  │  • Generates access token                                 │      │
│  │  • Applies user rights                                    │      │
│  └───────────────────────────────────────────────────────────┘      │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

## 🗂️ 核心模块详解

### 1. Credential Provider (`cp_provider`)

#### 职责
加载到 LogonUI 进程空间，提供自定义 UI 并收集双账号凭证。

#### 技术要点
```rust
// CLSID 注册到以下路径
HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Auth\LogonProviders\<GUID>

// 实现的 COM 接口
ICredentialProvider2::GetCredentialAt() -> ICredentialProviderCredential
ICredentialProviderCredential::SetCredentials() // 用户点击提交时调用
ICredentialProviderCredential::GetSerialization() // 传递给 LSA
```

#### 关键数据结构
```rust
pub struct DualAuthCredential {
    user_a_username: String,      // "domain\\usera" or "usera@domain.com"
    user_a_password_hash: Vec<u8>, // HMAC-SHA256(password) 
    user_b_username: String,
    user_b_password_hash: Vec<u8>,
    
    status: CredentialState, // Empty | Waiting | Submitting | Verified
}
```

#### 命名管道通信协议
```json
// Request from CP to Service
{
  "request_id": "uuid-v4",
  "user_a_username": "domain\\admin_a",
  "user_a_password_hash": "hex-encoded-hmac-salt-...",
  "user_b_username": "domain\\admin_b",
  "user_b_password_hash": "hex-encoded-hmac-salt-...",
  "timestamp": "2026-07-23T10:30:00Z"
}

// Response from Service to CP
{
  "result": "success|fail_user_a|fail_user_b|both_failed|timeout",
  "error_message": null,
  "audit_id": "uuid-of-audit-entry"
}
```

---

### 2. Windows Service (`win_service`)

#### 职责
充当中间层，处理身份验证逻辑、审计日志记录以及与 AD 通信。

#### 服务组件

##### Named Pipe Server
```rust
const PIPE_PATH: &str = r"\\.\pipe\winsla-auth-pipe";

ServerOptions::new().open(PIPE_PATH).await?;
// ACL 配置确保只有 LSASS/logonui 可访问
```

##### 双账号验证器
```rust
pub async fn validate_dual_accounts(
    user_a: (&str, &[u8]),
    user_b: (&str, &[u8]),
) -> Result<(), AuthError> {
    // 并行验证（提高响应速度）
    let (res_a, res_b) = tokio::join!(
        verify_single(user_a),
        verify_single(user_b),
    );
    
    match (res_a, res_b) {
        (Ok(_), Ok(_)) => Ok(()),
        (Err(e), Ok(_)) => Err(AuthError::UserAFailed(e)),
        (Ok(_), Err(e)) => Err(AuthError::UserBFailed(e)),
        (Err(ea), Err(eb)) => Err(AuthError::BothFailed(ea, eb)),
    }
}
```

##### LDAP 验证器
```rust
pub async fn simple_bind(dc: &str, dn: &str, password: &str) -> Result<(), AuthError> {
    // 使用 LDAPS (port 636) 优先于 LDAP (389)
    let mut ldap = Ldap::connect(&[format!("ldaps://{}", dc)]).await?;
    
    // 简单绑定尝试
    ldap.simple_bind(dn, password).await?;
    
    // 如果成功则返回
    ldap.close().await.ok();
    Ok(())
}
```

##### SSPI 验证器
```rust
pub async fn authenticate_ntlm(
    username: &str,
    password: &str,
    domain: &str,
) -> Result<(), AuthError> {
    // 备选路径：当 LDAP 不可用时使用 NTLM
    unsafe {
        use windows::Win32::Security::Authentication::Identity::Ntlm::*;
        
        let mut cred_handle = CredHandle::default();
        let mut context = ContextHandle::default();
        
        InitializeSecurityContextW(...) -> SEC_E_OK ?
    }
}
```

---

### 3. AD Bridge (`ad_bridge`)

#### 职责
通用 LDAP/SSPI 客户端封装，供 Service 和管理端复用。

#### 功能清单
```rust
pub struct DomainAuthClient {
    config: DomainConfig,
}

impl DomainAuthClient {
    /// 验证单账号凭据
    pub async fn verify_credentials(&self, username: &str, password: &str) -> Result<bool>
    
    /// 获取用户 SID（用于审计和策略匹配）
    pub async fn get_user_sid(&self, username: &str) -> Result<Option<String>>
    
    /// 检查账户状态（锁定、过期等）
    pub async fn is_account_locked(&self, username: &str) -> Result<bool>
}
```

#### 配置格式
```toml
[domain]
dc_addresses = ["dc1.example.com", "dc2.example.com"]
admin_dn = "CN=admin,OU=Admins,DC=example,DC=com"
base_dn = "DC=example,DC=com"

# SSL/TLS 设置
tls_verify_cert = true
trusted_ca_cert_path = "/path/to/ca.crt"
```

---

### 4. Tauri Management App (`management_app`)

#### 前端架构
```vue
<!-- App.vue -->
<script setup>
import { ref } from 'vue'

const serviceStatus = ref(false)
const pairRules = ref([])
const auditLogs = ref([])

// Tauri 命令调用
import { invoke } from '@tauri-apps/api/tauri'

invoke('start_service')
invoke('get_audit_logs', { start: 0, limit: 100 })
</script>
```

#### 后端命令 (`commands.rs`)
```rust
#[tauri::command]
async fn get_audit_logs(start: usize, limit: usize) -> Result<Vec<AuditEntry>, String> {
    use sqlx::Row;
    
    let rows = sqlx::query("SELECT * FROM audit_log ORDER BY timestamp DESC LIMIT ? OFFSET ?")
        .bind(limit as i64)
        .bind(start as i64)
        .fetch_all(pool)
        .await?;
    
    Ok(rows.into_iter().map(|r| r.try_get("data")).collect())
}

#[tauri::command]
fn configure_dual_pair(user_a_sid: String, user_b_sid: String) -> Result<(), String> {
    use sqlx::Transaction;
    
    // BEGIN;
    // INSERT INTO dual_pairs...
    // COMMIT;
    Ok(())
}
```

#### SQLite Schema
```sql
CREATE TABLE dual_pairs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_a_sid VARCHAR(255) NOT NULL UNIQUE,
    user_b_sid VARCHAR(255) NOT NULL UNIQUE,
    enabled BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    request_id UUID NOT NULL,
    user_a_sid VARCHAR(255),
    user_b_sid VARCHAR(255),
    result ENUM('success', 'fail_user_a', 'fail_user_b', 'both_fail'),
    error_message TEXT,
    client_hostname VARCHAR(255)
);

CREATE INDEX idx_audit_timestamp ON audit_log(timestamp DESC);
CREATE INDEX idx_audit_request ON audit_log(request_id);
```

---

## 🔐 安全架构

### 数据流加密

#### 1. 密码传输保护
```
CP (Secure Desktop) → Named Pipe → Service (SYSTEM)

方法 A: HMAC-SHA256 (当前实现，开发阶段)
password_hash = HMAC_SHA256(password, dynamic_salt_from_pipe_connection)

方法 B: AES-256-GCM (生产推荐)
- 使用临时 Diffie-Hellman 密钥交换
- 对称加密每个消息包
```

#### 2. 模板存储加密（未来扩展指纹时）
```rust
use aes_gcm::{Aes256Gcm, KeyInit, aead::{OsRng}};

// 根密钥来自 DPAPI
let root_key = CryptProtectData(user_secret.as_mut_slice(), None)?;

// 派生会话密钥
let session_key = HKDF::<Sha256>::derive_from(
    &(root_key, user_sid.as_bytes()),
    b"winsla-fingerprint-template",
);

// AES-GCM 加密
let nonce = OsRng.gen();
let encrypted_template = Aes256Gcm::new(session_key)
    .encrypt(&nonce, template_bytes)
    .unwrap();
```

#### 3. Named Pipe ACL
```powershell
# PowerShell: 设置权限仅允许 SYSTEM 和 TrustedInstaller
$securityDescriptor = New-Object System.Security.AccessControl.RawSecurityDescriptor -ArgumentList "O:BAG:SYD:P(A;;FA;;;SY)(A;;FA;;;WD)"
$sdByteArray = New-Object byte[]($securityDescriptor.BinaryLength)
$securityDescriptor.GetBinaryForm($sdByteArray, 0)

Set-ItemProperty -Path "HKLM:\SYSTEM\CurrentControlSet\Services\WinSLA Service" `
                 -Name "PipeACL" -Value $sdByteArray
```

---

## ⚡ 性能优化策略

### 1. 并行验证
```rust
tokio::join!(verify_a, verify_b) // < 1s vs sequential 2s
```

### 2. 连接池复用
```rust
// LDAP connections should be pooled across requests
let pool = LdapPool::new(vec![dc_urls], max_size: 10)?;
```

### 3. 超时管理
```rust
tokio::time::timeout(Duration::from_secs(30), auth_operation)
    .await
    .unwrap_or_else(|| AuthResponse::Timeout)
```

### 4. 缓存最近成功凭据
```rust
LRU_CACHE.insert(sid, encrypted_token_ttl_5min);
```

---

## 🛠️ 部署拓扑

### 单机版（MVP）
```
Client PC (Joined to DOMAIN)
├── Credential Provider DLL (logonui.exe context)
├── Windows Service (SYSTEM context)
└── SQLite DB (%ALLUSERSPROFILE%\WinSLA\data.db)

Domain Controllers: LDAP over TLS
```

### 在线版（Future）
```
Management Console (Web Admin Portal)
├── Central SQL DB (PostgreSQL/MySQL)
├── API Gateway (REST/gRPC)
└── WebSocket for real-time logs

Each Client PC:
├── Credential Provider (same as single-machine)
├── Service (with sync agent)
│   └── HTTPS pull policy every 5 min
└── Cache locally (offline support)
```

---

## 🧪 测试覆盖矩阵

| 场景 | CP Test | Service Test | E2E | Expected |
|------|---------|--------------|-----|----------|
| Both users correct | ✓ | ✓ | ✓ | Login success |
| User A wrong | ✓ | ✓ | ✓ | FailUserA message |
| User B wrong | ✓ | ✓ | ✓ | FailUserB message |
| Both wrong | ✓ | ✓ | ✓ | BothFailed message |
| DC offline | ✗ | ✓ | ○ | Fallback to cached tokens |
| Network timeout | ✓ | ✓ | ✓ | Timeout response |
| Invalid format | ✓ | ✓ | ✓ | Format error |
| Cancel submission | ✓ | ✗ | ✓ | Return to initial state |

✅ Unit/Integration, ○ Integration only, ✗ Not applicable (CP doesn't call AD directly)

---

## 📝 后续增强方向

1. **生物识别扩展**（原需求指纹）
   - 添加 ZKTeco/DigitalPersona SDK 支持
   - Template storage in TPM-backed HSM
   
2. **策略引擎升级**
   - Time-based rules (only allow during work hours)
   - Device compliance checks (BitLocker enabled?)
   
3. **高可用支持**
   - Multi-DC failover with priority
   - Regional load balancing (for online version)
   
4. **审计合规**
   - SIEM integration (Splunk/Sentinel)
   - Real-time alerting on suspicious patterns

---

*Last Updated: 2026-07-23*
