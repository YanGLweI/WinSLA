//! Axum REST API server for WinSLA Management

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::database::{Database, DailyStat};
use crate::commands;

const SERVICE_NAME: &str = "WinSLA Service";

pub type AppState = Arc<Mutex<Database>>;

/// Computer information response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerInfo {
    pub hostname: String,
    pub domain: String,
}

// ============================================================================
// Request/Response Types for one-to-many pairing (v2)
// ============================================================================

#[derive(Deserialize)]
struct AddApproverRequest {
    account_sid: String,
    approver_sid: String,
    approver_username: String,
}

/// Create the axum router with all API routes
pub fn create_router(db: AppState) -> Router {
    Router::new()
        .route("/api/status", get(get_status))
        // v2 endpoints (one-to-many)
        .route("/api/accounts", get(list_accounts).post(create_account))
        .route("/api/accounts/approvers", post(add_approver))
        .route("/api/accounts/{account_sid}/approvers/{approver_sid}", delete(remove_approver))
        .route("/api/accounts/{account_sid}/enable", put(toggle_account_enable))
        .route("/api/accounts/{account_sid}", delete(delete_account_pair))
        // Keep old endpoints for backward compatibility (until frontend is updated)
        .route("/api/pairs", get(list_accounts).post(add_pair))
        .route("/api/pairs/{id}", delete(delete_pair))
        .route("/api/emergency", get(list_emergency).post(add_emergency))
        .route("/api/emergency/{id}", delete(delete_emergency))
        .route("/api/audit", get(list_audit))
        .route("/api/stats/daily", get(get_daily_stats))
        .route("/api/computer-info", get(get_computer_info))
        .route("/api/policy", get(get_policy).put(update_policy))
        .route("/api/service/start", post(service_start))
        .route("/api/service/stop", post(service_stop))
        .route("/api/service/restart", post(service_restart))
        .route("/api/service/config", get(get_service_config).put(set_service_config))
        .route("/api/validate-account", post(validate_account))
        .fallback(serve_frontend)
        .with_state(db)
}

/// Start the HTTP server on the given port
pub async fn start_server(db: AppState, port: u16) -> anyhow::Result<()> {
    let app = create_router(db);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Management server listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

// ─── API Handlers ───────────────────────────────────────────────

async fn get_status(State(db): State<AppState>) -> Json<commands::ServiceStatus> {
    let running = commands::is_service_running();

    // Get real authentication statistics from the shared database
    let (connections_accepted, successful_auths, failed_auths) = {
        let db = db.lock().unwrap();
        db.get_auth_stats().unwrap_or((0, 0, 0))
    };

    Json(commands::ServiceStatus {
        running,
        version: env!("CARGO_PKG_VERSION").to_string(),
        connections_accepted,
        successful_auths,
        failed_auths,
    })
}

async fn list_accounts(State(db): State<AppState>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_all_accounts() {
        Ok(accounts) => Json(accounts).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct AddPairRequest {
    account_username: String,
    approver_username: String,
    #[serde(default)]
    account_sid: String,
    #[serde(default)]
    approver_sid: String,
}

async fn add_pair(State(db): State<AppState>, Json(req): Json<AddPairRequest>) -> impl IntoResponse {
    use serde_json::json;
    
    let db = db.lock().unwrap();
    
    // 检查是否为第一条配对
    let existing_pairs = match db.get_all_dual_pairs() {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    
    // 添加新配对
    let new_pair = match db.add_dual_pair(&req.account_sid, &req.approver_sid, &req.account_username, &req.approver_username) {
        Ok(pair) => pair,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    
    // 如果这是第一条配对，自动禁用默认 Tile
    let auto_disabled_default_tile = existing_pairs.is_empty();
    let mut policy_config = db.get_policy().unwrap_or_default();
    
    if auto_disabled_default_tile && policy_config.default_tile_enabled {
        // 自动设置为禁用并保存到 DB
        policy_config.default_tile_enabled = false;
        if let Err(e) = db.save_policy(&policy_config) {
            log::warn!("Failed to save policy after adding first pair: {}", e);
        }
    }
    
    // 获取应急账号数量
    let has_emergency_accounts = db.get_emergency_accounts().map(|acc| !acc.is_empty()).unwrap_or(false);
    
    drop(db);
    
    // 立即更新注册表
    #[cfg(windows)]
    if auto_disabled_default_tile && !policy_config.default_tile_enabled {
        if let Err(e) = write_policy_to_registry(&policy_config) {
            eprintln!("Warning: Failed to update registry when adding first pair: {}", e);
        }
    }
    
    // 构建响应，携带附加信息供前端决策
    (StatusCode::CREATED, Json(json!({
        "pair": new_pair,
        "auto_disabled_default_tile": auto_disabled_default_tile,
        "has_emergency_accounts": has_emergency_accounts,
        "should_configure_emergency": auto_disabled_default_tile && !has_emergency_accounts
    }))).into_response()
}

async fn delete_pair(State(db): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    
    // 先检查是否有其他配对
    let existing_pairs = match db.get_all_dual_pairs() {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    
    // 计算剩余配对数（排除当前要删除的）
    let remaining_pairs: Vec<_> = existing_pairs.iter().filter(|p| p.id != id.as_str()).collect();
    
    // 删除配对
    match db.remove_dual_pair(&id) {
        Ok(_) => {
            // 如果删除后剩余 0 条配对，且当前已禁用默认 Tile，则自动恢复
            if remaining_pairs.is_empty() {
                let mut policy_config = db.get_policy().unwrap_or_default();
                if !policy_config.default_tile_enabled {
                    policy_config.default_tile_enabled = true;
                    let _ = db.save_policy(&policy_config);
                    
                    drop(db);
                    
                    // 更新注册表
                    #[cfg(windows)]
                    if let Err(e) = write_policy_to_registry(&policy_config) {
                        eprintln!("Warning: Failed to restore registry when removing last pair: {}", e);
                    }
                }
            }
            
            StatusCode::NO_CONTENT.into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ============================================================================
// v2 Handlers (one-to-many pairing)
// ============================================================================

#[derive(Deserialize)]
struct CreateAccountRequest {
    account_sid: String,
    account_username: String,
}

async fn create_account(State(db): State<AppState>, Json(req): Json<CreateAccountRequest>) -> impl IntoResponse {
    use serde_json::json;
    
    let db = db.lock().unwrap();
    
    let account = match db.create_account_pair(&req.account_sid, &req.account_username) {
        Ok(account) => account,
        Err(_) => return (StatusCode::CONFLICT, "Account already exists").into_response(),
    };
    
    // ✅ Bug 修复：不再在创建账号时自动禁用默认 Tile，改为在添加审批人时处理
    // 只返回普通响应
    Json(account).into_response()
}

// ============================================================================
// v2 Handlers (one-to-many pairing)
// ============================================================================

async fn add_approver(State(db): State<AppState>, Json(req): Json<AddApproverRequest>) -> impl IntoResponse {
    
    let result = {
        let db = db.lock().unwrap();
        db.add_approver_to_account(&req.account_sid, &req.approver_sid, &req.approver_username)
    };
    
    match result {
        Ok(true) => {
            // ✅ Bug 修复：检查是否为第一条完整配对（有审批人的账号）并禁用 default tile
            let should_disable_tile = {
                let db_guard = db.lock().unwrap();
                
                // 获取该账号的 approvers 列表 - 通过 get_all_accounts 来获取
                let all_accounts = db_guard.get_all_accounts().unwrap_or_default();
                
                // 找到当前账号的 approvers
                let current_account = all_accounts.iter()
                    .find(|acc| acc.account_sid == req.account_sid);
                
                // 检查当前账号是否有审批人
                let has_approvers_in_this_account = current_account.map(|acc| {
                    serde_json::from_str::<Vec<serde_json::Value>>(&acc.approvers)
                        .ok()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false)
                }).unwrap_or(false);
                
                // 检查是否只有这一个账号且它有审批人
                has_approvers_in_this_account && all_accounts.len() == 1
            };  // db_guard 在这里被 drop
            
            // ✅ 如果是第一条完整配对，禁用默认 Tile
            if should_disable_tile {
                let mut policy_config = crate::database::PolicyConfig::default();
                let db_guard = db.lock().unwrap();
                
                // 读取现有配置
                if let Ok(cfg) = db_guard.get_policy() {
                    policy_config = cfg;
                }
                
                if policy_config.default_tile_enabled {
                    policy_config.default_tile_enabled = false;
                    let _ = db_guard.save_policy(&policy_config);
                    
                    #[cfg(windows)]
                    {
                        if let Err(e) = write_policy_to_registry(&policy_config) {
                            eprintln!("Warning: Failed to update registry when adding first approver: {}", e);
                        }
                    }
                }
            }
            
            StatusCode::CREATED.into_response()
        },
        Ok(false) => (StatusCode::CONFLICT, "Approver already exists").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn remove_approver(State(db): State<AppState>, Path((account_sid, approver_sid)): Path<(String, String)>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.remove_approver_from_account(&account_sid, &approver_sid) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Approver not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn toggle_account_enable(
    State(db): State<AppState>, 
    Path(account_sid): Path<String>, 
    Json(enabled_payload): Json<EnabledPayload>
) -> impl IntoResponse {
    let db = db.lock().unwrap();
    let account_sid = account_sid.as_str();
    // ✅ 将整数转换为布尔值
    let enabled = enabled_payload.enabled != 0;
    
    match db.set_account_enabled(account_sid, enabled) {
        Ok(_) => Json(serde_json::json!({"account_sid": account_sid, "enabled": enabled})).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_account_pair(State(db): State<AppState>, Path(account_sid): Path<String>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    
    match db.remove_account_pair(&account_sid) {
        Ok(true) => {
            // ✅ Bug #4: 删除后显示提示，告知用户需要手动去策略配置页面启用默认 Tile
            let response_data = serde_json::json!({"deleted": true});
            (StatusCode::NO_CONTENT, Json(response_data)).into_response()
        },
        Ok(false) => (StatusCode::NOT_FOUND, "Account not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_emergency(State(db): State<AppState>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_emergency_accounts() {
        Ok(accounts) => Json(accounts).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct AddEmergencyRequest {
    sid: String,
    username: String,
    #[serde(default)]
    reason: String,
}

#[derive(Deserialize)]
struct EnabledPayload {
    enabled: i64,  // ✅ 改为接收整数 (0/1)，而不是布尔值
}

async fn add_emergency(State(db): State<AppState>, Json(req): Json<AddEmergencyRequest>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.add_emergency_account(&req.sid, &req.username, &req.reason, "admin") {
        Ok(account) => (StatusCode::CREATED, Json(account)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_emergency(State(db): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    // Emergency accounts table uses id field
    let result = db.remove_emergency_account(&id);
    match result {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct AuditQuery {
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 { 50 }

async fn list_audit(State(db): State<AppState>, Query(q): Query<AuditQuery>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_audit_log(q.limit) {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_daily_stats(State(db): State<AppState>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_daily_stats(7) {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_computer_info() -> Json<ComputerInfo> {
    use std::env;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    
    // Get computer name (hostname)
    let hostname = env::var("COMPUTERNAME")
        .unwrap_or_else(|_| "Unknown".to_string());
    
    // Get domain/workgroup
    let mut domain = String::from("WORKGROUP");
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::{
            GetComputerNameExW, COMPUTER_NAME_FORMAT,
        };
        
        // ComputerNameDnsDomain = 2
        let mut size: u32 = 0;
        let _ = unsafe { GetComputerNameExW(COMPUTER_NAME_FORMAT(2), windows::core::PWSTR::null(), &mut size) };
        
        if size > 0 {
            let mut buf: Vec<u16> = vec![0; size as usize];
            let ok = unsafe {
                GetComputerNameExW(
                    COMPUTER_NAME_FORMAT(2),
                    windows::core::PWSTR(buf.as_mut_ptr()),
                    &mut size,
                )
            };
            if ok.is_ok() && size > 0 {
                domain = String::from_utf16_lossy(&buf[..size as usize]);
            }
        }
    }
    
    Json(ComputerInfo { hostname, domain })
}

async fn get_policy(State(db): State<AppState>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_policy() {
        Ok(config) => Json(config).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn update_policy(State(db): State<AppState>, Json(config): Json<crate::database::PolicyConfig>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.save_policy(&config) {
        Ok(_) => {
            // 同时写入注册表，确保策略立即生效
            #[cfg(windows)]
            {
                if let Err(e) = write_policy_to_registry(&config) {
                    eprintln!("Warning: Failed to write policy to registry: {}", e);
                    // 不返回错误，因为数据库已保存成功
                }
            }
            StatusCode::NO_CONTENT.into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// 将策略写入 Windows 注册表
#[cfg(windows)]
fn write_policy_to_registry(config: &crate::database::PolicyConfig) -> Result<(), String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY_LOCAL_MACHINE, HKEY, 
        REG_SAM_FLAGS, REG_VALUE_TYPE, REG_OPTION_NON_VOLATILE
    };
    
    let key_path = OsStr::new(r"SOFTWARE\WinSLA\Policy")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    
    let mut hkey = HKEY(std::ptr::null_mut());
    
    // 创建或打开注册表键
    let result = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(key_path.as_ptr()),
            0,
            windows::core::PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            REG_SAM_FLAGS(0x0002), // KEY_WRITE
            None,
            &mut hkey,
            None, // disposition (可选)
        )
    };
    
    if result != ERROR_SUCCESS {
        let err_msg = format!("RegCreateKeyExW failed: {:?} (error code: {})", result, result.0);
        eprintln!("{}", err_msg);
        return Err(err_msg);
    }
    
    // 写入 DefaultTileEnabled 值
    let value_name = OsStr::new("DefaultTileEnabled")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let value_data: [u8; 4] = (if config.default_tile_enabled { 1u32 } else { 0u32 }).to_le_bytes();
    
    let result2 = unsafe {
        RegSetValueExW(
            hkey,
            windows::core::PCWSTR(value_name.as_ptr()),
            0,
            REG_VALUE_TYPE(4), // REG_DWORD
            Some(&value_data),
        )
    };
    
    if result2 != ERROR_SUCCESS {
        unsafe { let _ = RegCloseKey(hkey); }
        let err_msg = format!("RegSetValueExW(DefaultTileEnabled) failed: {:?}", result2);
        eprintln!("{}", err_msg);
        return Err(err_msg);
    }
    
    // 写入 EmergencyRequiresReason 值
    let value_name2 = OsStr::new("EmergencyRequiresReason")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<u16>>();
    let value_data2: [u8; 4] = (if config.emergency_requires_reason { 1u32 } else { 0u32 }).to_le_bytes();
    
    let result3 = unsafe {
        RegSetValueExW(
            hkey,
            windows::core::PCWSTR(value_name2.as_ptr()),
            0,
            REG_VALUE_TYPE(4), // REG_DWORD
            Some(&value_data2),
        )
    };
    
    unsafe { let _ = RegCloseKey(hkey); }
    
    if result3 != ERROR_SUCCESS {
        let err_msg = format!("RegSetValueExW(EmergencyRequiresReason) failed: {:?}", result3);
        eprintln!("{}", err_msg);
        return Err(err_msg);
    }
    
    eprintln!("Successfully wrote DefaultTileEnabled={}, EmergencyRequiresReason={} to registry",
        config.default_tile_enabled, config.emergency_requires_reason);
    Ok(())
}

// ─── Service Control ─────────────────────────────────────────────

#[derive(Serialize)]
struct ServiceActionResult {
    success: bool,
    message: String,
}

#[derive(Serialize, Deserialize)]
struct ServiceConfig {
    auto_start: bool,
}

fn run_sc(args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("sc.exe")
        .args(args)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Failed to execute sc.exe: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("{}{}", stdout, stderr))
    }
}

fn run_net(args: &[&str]) -> Result<String, String> {
    use std::os::windows::process::CommandExt;
    let output = std::process::Command::new("net")
        .args(args)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .map_err(|e| format!("Failed to execute net: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        // net start returns error if already running, etc.
        Err(stdout.trim().to_string())
    }
}

async fn service_start() -> Json<ServiceActionResult> {
    match run_net(&["start", SERVICE_NAME]) {
        Ok(msg) => Json(ServiceActionResult { success: true, message: msg.trim().to_string() }),
        Err(msg) => {
            if msg.contains("已经启动") || msg.contains("already been started") {
                Json(ServiceActionResult { success: true, message: "服务已在运行中".to_string() })
            } else {
                Json(ServiceActionResult { success: false, message: msg })
            }
        }
    }
}

async fn service_stop() -> Json<ServiceActionResult> {
    match run_net(&["stop", SERVICE_NAME]) {
        Ok(msg) => Json(ServiceActionResult { success: true, message: msg.trim().to_string() }),
        Err(msg) => {
            if msg.contains("尚未启动") || msg.contains("not been started") {
                Json(ServiceActionResult { success: true, message: "服务未在运行".to_string() })
            } else {
                Json(ServiceActionResult { success: false, message: msg })
            }
        }
    }
}

async fn service_restart() -> Json<ServiceActionResult> {
    // Stop then start
    let _ = run_net(&["stop", SERVICE_NAME]);
    std::thread::sleep(std::time::Duration::from_millis(1000));
    match run_net(&["start", SERVICE_NAME]) {
        Ok(msg) => Json(ServiceActionResult { success: true, message: format!("服务已重启\n{}", msg.trim()) }),
        Err(msg) => Json(ServiceActionResult { success: false, message: msg }),
    }
}

async fn get_service_config() -> Json<ServiceConfig> {
    // Query service start type via sc qc
    let auto_start = match run_sc(&["qc", SERVICE_NAME]) {
        Ok(output) => {
            // AUTO_START means start type is auto
            output.contains("AUTO_START")
        }
        Err(_) => false,
    };
    Json(ServiceConfig { auto_start })
}

async fn set_service_config(Json(config): Json<ServiceConfig>) -> Json<ServiceActionResult> {
    let start_type = if config.auto_start { "auto" } else { "demand" };
    match run_sc(&["config", SERVICE_NAME, &format!("start= {}", start_type)]) {
        Ok(_) => Json(ServiceActionResult {
            success: true,
            message: if config.auto_start { "已设置为开机自动启动".to_string() } else { "已设置为手动启动".to_string() },
        }),
        Err(msg) => Json(ServiceActionResult { success: false, message: msg }),
    }
}

// ─── Account Validation ──────────────────────────────────────────

#[derive(Deserialize)]
struct ValidateAccountRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct ValidateAccountResponse {
    success: bool,
    sid: String,
    display_name: String,
    message: String,
}

async fn validate_account(Json(req): Json<ValidateAccountRequest>) -> Json<ValidateAccountResponse> {
    let username = req.username.clone();
    let password = req.password.clone();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        tokio::task::spawn_blocking(move || {
            crate::wincred::validate_and_resolve(&username, &password)
        }),
    ).await;

    match result {
        Ok(Ok(Ok((sid, display_name)))) => Json(ValidateAccountResponse {
            success: true,
            sid,
            display_name: display_name.clone(),
            message: format!("验证成功：{}", display_name),
        }),
        Ok(Ok(Err(msg))) => Json(ValidateAccountResponse {
            success: false,
            sid: String::new(),
            display_name: String::new(),
            message: msg,
        }),
        Ok(Err(_)) => Json(ValidateAccountResponse {
            success: false,
            sid: String::new(),
            display_name: String::new(),
            message: "验证线程异常".to_string(),
        }),
        Err(_) => Json(ValidateAccountResponse {
            success: false,
            sid: String::new(),
            display_name: String::new(),
            message: "验证超时：无法连接域控制器（30 秒）".to_string(),
        }),
    }
}

// ─── Static File Serving ────────────────────────────────────────

async fn serve_frontend(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    match crate::frontend::Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(content.data.to_vec().into())
                .unwrap()
        }
        None => {
            // SPA fallback: serve index.html for client-side routing
            match crate::frontend::Assets::get("index.html") {
                Some(content) => Html(String::from_utf8_lossy(&content.data).to_string()).into_response(),
                None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
            }
        }
    }
}
