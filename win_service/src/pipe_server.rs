//! Named Pipe Server for WinSLA Authentication Service
//!
//! Uses tokio's built-in Windows named pipe support for async I/O.

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ServerOptions;

use crate::auth;
use crate::audit::{AuditDb, ServicePolicy};
use crate::com_types::{AuthMode, AuthRequest, AuthResponse};
use crate::ServiceState;

const PIPE_PATH: &str = r"\\.\pipe\winsla-auth-pipe";
const REGISTRY_POLICY_KEY: &str = r"SOFTWARE\WinSLA\Policy";

/// Main pipe server loop (blocking - spawns its own tokio runtime)
pub fn run_pipe_server(state: Arc<Mutex<ServiceState>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Starting pipe server on {}", PIPE_PATH);

    // Start background policy sync thread (reads policy from shared DB, writes to registry)
    let state_clone = Arc::clone(&state);
    std::thread::spawn(move || {
        if let Err(e) = run_policy_sync_loop(state_clone) {
            log::error!("Policy sync loop error: {}", e);
        }
    });

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        run_async_pipe_server(state).await
    })
}

/// Background thread that polls the shared database for policy changes and writes them to registry.
/// Runs every 10 seconds, only writes when value actually changes.
fn run_policy_sync_loop(state: Arc<Mutex<ServiceState>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
    use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY_LOCAL_MACHINE};
    
    const SYNC_INTERVAL_SECS: u64 = 10;
    let mut last_default_tile: Option<bool> = None;
    let mut last_emergency_reason: Option<bool> = None;
    
    loop {
        // Try to read policy from shared DB
        let policy = AuditDb::open()
            .ok()
            .map(|db| db.get_policy());
        
        if let Some(policy) = policy {
            let new_default_tile = Some(policy.default_tile_enabled);
            let new_emergency_reason = Some(policy.emergency_requires_reason);
            
            // Only write to registry if any value changed
            if new_default_tile != last_default_tile || new_emergency_reason != last_emergency_reason {
                let result = write_registry_policy_key(policy.default_tile_enabled, policy.emergency_requires_reason);
                match result {
                    Ok(_) => {
                        log::info!("Policy synced to registry: default_tile_enabled={}, emergency_requires_reason={}",
                            policy.default_tile_enabled, policy.emergency_requires_reason);
                        last_default_tile = new_default_tile;
                        last_emergency_reason = new_emergency_reason;
                    }
                    Err(e) => {
                        log::warn!("Failed to write registry key: {}", e);
                        // Continue syncing even if registry write fails (fail-safe)
                    }
                }
            }
        } else {
            // Can't open DB - log warning but continue
            log::warn!("Cannot open audit DB for policy sync");
        }
        
        // Wait for next sync interval
        std::thread::sleep(std::time::Duration::from_secs(SYNC_INTERVAL_SECS));
    }
}

/// Write policy configuration to Windows Registry under HKLM\SOFTWARE\WinSLA\Policy
fn write_registry_policy_key(default_tile_enabled: bool, emergency_requires_reason: bool) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{ERROR_SUCCESS};
    use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegSetValueExW, HKEY_LOCAL_MACHINE, HKEY};
    
    // Convert registry path to wide string
    let wide_path: Vec<u16> = OsStr::new(REGISTRY_POLICY_KEY)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let mut hkey = HKEY(std::ptr::null_mut());
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(wide_path.as_ptr()),
            0,
            windows::Win32::System::Registry::REG_SAM_FLAGS(0x0002), // KEY_WRITE access
            &mut hkey,
        )
    };
    
    if result != ERROR_SUCCESS {
        return Err(format!("Failed to open registry key: {:?}", result));
    }
    
    // Write DWORD value DefaultTileEnabled
    let value_name = OsStr::new("DefaultTileEnabled").encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let value_data: [u8; 4] = (if default_tile_enabled { 1u32 } else { 0u32 }).to_le_bytes();
    let result2 = unsafe {
        RegSetValueExW(
            hkey,
            windows::core::PCWSTR(value_name.as_ptr()),
            0,
            windows::Win32::System::Registry::REG_VALUE_TYPE(4), // REG_DWORD
            Some(&value_data),
        )
    };
    
    if result2 != ERROR_SUCCESS {
        unsafe { RegCloseKey(hkey); }
        return Err(format!("Failed to write DefaultTileEnabled: {:?}", result2));
    }
    
    // Write DWORD value EmergencyRequiresReason
    let value_name2 = OsStr::new("EmergencyRequiresReason").encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let value_data2: [u8; 4] = (if emergency_requires_reason { 1u32 } else { 0u32 }).to_le_bytes();
    let result3 = unsafe {
        RegSetValueExW(
            hkey,
            windows::core::PCWSTR(value_name2.as_ptr()),
            0,
            windows::Win32::System::Registry::REG_VALUE_TYPE(4), // REG_DWORD
            Some(&value_data2),
        )
    };
    
    unsafe { RegCloseKey(hkey); }
    
    if result3 != ERROR_SUCCESS {
        return Err(format!("Failed to write EmergencyRequiresReason: {:?}", result3));
    }
    
    Ok(())
}

/// Async pipe server implementation
async fn run_async_pipe_server(state: Arc<Mutex<ServiceState>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        // Create a new pipe server instance
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(PIPE_PATH)?;

        log::info!("Waiting for client connection...");

        // Wait for a client to connect
        server.connect().await?;
        log::info!("Client connected via pipe");

        {
            let mut state_guard = state.lock().unwrap();
            state_guard.connections_accepted += 1;
            state_guard.update_activity();
        }

        let state_clone = Arc::clone(&state);

        // Handle the client in a spawned task
        tokio::spawn(async move {
            if let Err(e) = handle_client(server, state_clone).await {
                log::error!("Client handler error: {}", e);
            }
        });
    }
}

/// Handle a single client connection
async fn handle_client(
    mut pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    state: Arc<Mutex<ServiceState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read message length (u32 little-endian)
    let mut len_bytes = [0u8; 4];
    pipe.read_exact(&mut len_bytes).await?;
    let message_len = u32::from_le_bytes(len_bytes) as usize;

    if message_len > 10_000_000 {
        return Err("Invalid message size".into());
    }

    // Read message body
    let mut buf = vec![0u8; message_len];
    pipe.read_exact(&mut buf).await?;

    // Deserialize request
    let request: AuthRequest = serde_json::from_slice(&buf)?;

    log::info!(
        "Processing auth request: {} <-> {} (source: {})",
        request.user_a_username,
        request.user_b_username,
        request.logon_source
    );

    // Process authentication
    let response = process_auth_request(request, &state).await;

    // Serialize and send response with length prefix
    let response_bytes = serde_json::to_vec(&response)?;
    let len_prefix = (response_bytes.len() as u32).to_le_bytes();

    pipe.write_all(&len_prefix).await?;
    pipe.write_all(&response_bytes).await?;
    pipe.flush().await?;

    log::info!("Response sent successfully");

    Ok(())
}

/// Process an authentication request (dual-control or emergency mode)
async fn process_auth_request(request: AuthRequest, state: &Arc<Mutex<ServiceState>>) -> AuthResponse {
    let response = match request.mode {
        AuthMode::Dual => process_dual_auth(&request).await,
        AuthMode::Emergency => process_emergency_auth(&request).await,
    };

    // Record result in service state
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.update_activity();
        if response.is_success() {
            state_guard.record_success();
        } else {
            state_guard.record_failure();
        }
    }

    response
}

/// Dual-control login: lockout check -> pairing rule -> real credential verification
async fn process_dual_auth(request: &AuthRequest) -> AuthResponse {
    let account = request.user_a_username.clone();
    let approver = request.user_b_username.clone();

    let db = AuditDb::open().ok();
    if db.is_none() {
        log::warn!("Failed to open shared database; policy checks degraded");
    }
    let policy = db.as_ref().map(|d| d.get_policy()).unwrap_or_default();

    // Step 0: lockout check (keyed by primary account)
    if let Some(d) = &db {
        if let Some(secs) = d.get_lock_remaining_secs(&account) {
            log::warn!("Login rejected: account {} is locked ({}s remaining)", account, secs);
            let _ = d.record_auth(&account, &approver, "locked_attempt",
                Some("账号处于锁定期，本次尝试被拒绝"), Some(&request.logon_source));
            return AuthResponse::Locked { remaining_secs: secs };
        }
    }

    // Step 1: Check pairing rules first (strict order validation)
    match auth::dual_validator::check_pairing_rule(&account, &approver).await {
        Ok(()) => {
            log::info!("Pairing rule validation passed");
        }
        Err(auth::AuthError::InvalidCredentials(msg)) => {
            log::warn!("Pairing rule rejected login: {}", msg);
            if let Some(d) = &db {
                let _ = d.record_auth(&account, &approver, "pairing_violation", Some(&msg), Some(&request.logon_source));
            }
            // Pairing violations count toward the lockout threshold
            return record_failure_and_build_response(&db, &account, &policy,
                AuthResponse::FailUserA(msg));
        }
        Err(e) => {
            log::error!("Pairing rule check error: {}", e);
            return AuthResponse::NetworkUnavailable;
        }
    }

    // Step 2: Verify both accounts' credentials for real (LogonUserW)
    let result = auth::dual_validator::validate_dual_accounts(
        &account,
        &request.user_a_password,
        &approver,
        &request.user_b_password,
    )
    .await;

    let response = match result {
        Ok(canonical) => {
            log::info!("Authentication successful for {} + {} (canonical: {})", account, approver, canonical);
            if let Some(d) = &db {
                d.reset_login_failures(&account);
            }
            AuthResponse::Success { canonical_username: canonical }
        }
        Err(e) => {
            // Handle password expired error specifically
            if let auth::AuthError::PasswordExpired(expired_user) = &e {
                log::warn!("Password expired for user: {}", expired_user);
                if let Some(d) = &db {
                    let _ = d.record_auth(&account, &approver, "password_expired", Some("密码已过期，需要通过 Windows 原生对话框修改密码"), Some(&request.logon_source));
                }
                return AuthResponse::PasswordExpired(expired_user.clone());
            }
            
            let base = match e {
                auth::AuthError::InvalidCredentials(msg) => {
                    log::warn!("Auth failed (invalid credentials): {}", msg);
                    AuthResponse::FailUserA(msg)
                }
                auth::AuthError::BothFailed(msg_a, msg_b) => {
                    log::warn!("Both users failed: A={}, B={}", msg_a, msg_b);
                    AuthResponse::BothFailed(msg_a, msg_b)
                }
                auth::AuthError::NetworkUnavailable => {
                    log::error!("Network unavailable during auth");
                    AuthResponse::NetworkUnavailable
                }
                auth::AuthError::Timeout => {
                    log::error!("Auth timeout");
                    AuthResponse::Timeout
                }
                other => {
                    log::error!("Auth error: {}", other);
                    AuthResponse::FailUserA(other.to_string())
                }
            };
            record_failure_and_build_response(&db, &account, &policy, base)
        }
    };

    // Write audit record to shared SQLite database
    let (result_str, error_msg) = describe_response(&response);
    if let Some(d) = &db {
        if let Err(e) = d.record_auth(&account, &approver, result_str, error_msg.as_deref(), Some(&request.logon_source)) {
            log::warn!("Failed to write audit record to DB: {}", e);
        }
    }

    response
}

/// Emergency override login: policy switch -> reason -> authorization -> password
async fn process_emergency_auth(request: &AuthRequest) -> AuthResponse {
    let username = request.user_a_username.clone();
    let reason = request.reason.trim().to_string();
    log::warn!("Emergency login attempt: user={}, reason={}", username, reason);

    let db = match AuditDb::open() {
        Ok(d) => d,
        Err(e) => {
            log::error!("Emergency login unavailable: cannot open shared DB: {}", e);
            return AuthResponse::EmergencyDenied("应急服务不可用（数据库错误）".to_string());
        }
    };
    let policy = db.get_policy();

    // 1. Policy switch
    if !policy.allow_emergency_override {
        let _ = db.record_auth(&username, "", "emergency_denied", Some("应急覆盖已被策略禁用"), Some(&request.logon_source));
        return AuthResponse::EmergencyDenied("应急覆盖已被管理员禁用".to_string());
    }

    // 2. Reason required
    if policy.emergency_requires_reason && reason.is_empty() {
        return AuthResponse::EmergencyDenied("必须填写应急登录原因".to_string());
    }

    // 3. Lockout applies to emergency accounts too
    if let Some(secs) = db.get_lock_remaining_secs(&username) {
        let _ = db.record_auth(&username, "", "locked_attempt", Some("应急账号处于锁定期"), Some(&request.logon_source));
        return AuthResponse::Locked { remaining_secs: secs };
    }

    // 4. Authorization check: username must be in the emergency account list
    let input_bare = auth::dual_validator::extract_bare_username(&username);
    let authorized = db.get_emergency_accounts();
    let matched = authorized.iter().any(|(_sid, name)| {
        auth::dual_validator::extract_bare_username(name) == input_bare
    });
    if !matched {
        let msg = format!("账号 '{}' 未被授权应急登录", username);
        let _ = db.record_auth(&username, "", "emergency_denied", Some(&msg), Some(&request.logon_source));
        return AuthResponse::EmergencyDenied(msg);
    }

    // 5. Real password verification
    let (u, p) = (username.clone(), request.user_a_password.clone());
    let verify = tokio::task::spawn_blocking(move || {
        auth::dual_validator::verify_password_windows(&u, &p)
    }).await;

    match verify {
        Ok(Ok(canonical)) => {
            db.reset_login_failures(&username);
            let audit_msg = format!("应急登录批准，原因：{}", reason);
            let _ = db.record_auth(&username, "", "emergency_override", Some(&audit_msg), Some(&request.logon_source));
            log::warn!("EMERGENCY OVERRIDE APPROVED: user={}, reason={} (canonical: {})", username, reason, canonical);
            AuthResponse::Success { canonical_username: canonical }
        }
        Ok(Err(e)) => {
            // Check if password has expired (special marker in error message)
            let error_msg = e.to_string();
            if error_msg.contains("PASSWORD_EXPIRED") {
                log::warn!("Password expired for emergency user: {}", username);
                let _ = db.record_auth(&username, "", "password_expired",
                    Some("密码已过期，需要通过 Windows 原生对话框修改密码"), Some(&request.logon_source));
                return AuthResponse::PasswordExpired(username.clone());
            }
            
            let (remaining, locked) = db.record_login_failure(
                &username, policy.max_retry_count, policy.lockout_duration_minutes);
            let _ = db.record_auth(&username, "", "emergency_denied",
                Some(&format!("密码验证失败：{}", e)), Some(&request.logon_source));
            if let Some(secs) = locked {
                return AuthResponse::Locked { remaining_secs: secs };
            }
            AuthResponse::EmergencyDenied(format!("{}（剩余尝试次数：{}）", e, remaining))
        }
        Err(e) => {
            AuthResponse::EmergencyDenied(format!("验证任务异常：{}", e))
        }
    }
}

/// Record a failed dual-auth attempt against the lockout policy and build the
/// final response (converts to Locked when the threshold is reached, otherwise
/// appends a remaining-attempts hint to credential failures).
fn record_failure_and_build_response(
    db: &Option<AuditDb>,
    account: &str,
    policy: &ServicePolicy,
    base: AuthResponse,
) -> AuthResponse {
    let d = match db {
        Some(d) => d,
        None => return base,
    };
    let (remaining, locked) = d.record_login_failure(
        account, policy.max_retry_count, policy.lockout_duration_minutes);
    if let Some(secs) = locked {
        return AuthResponse::Locked { remaining_secs: secs };
    }
    match base {
        AuthResponse::FailUserA(msg) => AuthResponse::FailUserA(with_attempts_hint(msg, remaining)),
        AuthResponse::FailUserB(msg) => AuthResponse::FailUserB(with_attempts_hint(msg, remaining)),
        AuthResponse::BothFailed(a, b) => AuthResponse::BothFailed(with_attempts_hint(a, remaining), b),
        other => other,
    }
}

fn with_attempts_hint(msg: String, remaining: u32) -> String {
    format!("{}（剩余尝试次数：{}）", msg, remaining)
}

/// Map a response to (audit_result_str, error_message)
fn describe_response(response: &AuthResponse) -> (&'static str, Option<String>) {
    match response {
        AuthResponse::Success { .. } => ("success", None),
        AuthResponse::FailUserA(msg) => ("fail_user_a", Some(msg.clone())),
        AuthResponse::FailUserB(msg) => ("fail_user_b", Some(msg.clone())),
        AuthResponse::BothFailed(a, b) => ("fail_both", Some(format!("{}; {}", a, b))),
        AuthResponse::Locked { remaining_secs } => {
            ("locked", Some(format!("账号已锁定，剩余 {} 秒", remaining_secs)))
        }
        AuthResponse::EmergencyDenied(reason) => {
            // Categorize denial reason for finer-grained audit logs
            if reason.contains("已被策略禁用") || reason.contains("已被管理员禁用") {
                ("emergency_denied_policy", Some(reason.clone()))
            } else if reason.contains("未被授权") {
                ("emergency_denied_unauthorized", Some(reason.clone()))
            } else {
                ("emergency_denied", Some(reason.clone()))
            }
        },
        AuthResponse::PasswordExpired(username) => {
            ("password_expired", Some(format!("密码已过期：{}", username)))
        }
        AuthResponse::NetworkUnavailable => {
            ("network_unavailable", Some("Network unavailable".to_string()))
        }
        AuthResponse::Timeout => ("timeout", Some("Authentication timeout".to_string())),
    }
}
