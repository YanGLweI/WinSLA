//! Axum REST API server for WinSLA Management

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};

use crate::database::Database;
use crate::commands;

const SERVICE_NAME: &str = "WinSLA Service";

pub type AppState = Arc<Mutex<Database>>;

/// Create the axum router with all API routes
pub fn create_router(db: AppState) -> Router {
    Router::new()
        .route("/api/status", get(get_status))
        .route("/api/pairs", get(list_pairs).post(add_pair))
        .route("/api/pairs/{id}", delete(delete_pair))
        .route("/api/emergency", get(list_emergency).post(add_emergency))
        .route("/api/emergency/{id}", delete(delete_emergency))
        .route("/api/audit", get(list_audit))
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

async fn get_status() -> Json<commands::ServiceStatus> {
    Json(commands::get_service_status().unwrap_or(commands::ServiceStatus {
        running: false,
        version: env!("CARGO_PKG_VERSION").to_string(),
        connections_accepted: 0,
        successful_auths: 0,
        failed_auths: 0,
    }))
}

async fn list_pairs(State(db): State<AppState>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.get_all_dual_pairs() {
        Ok(pairs) => Json(pairs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct AddPairRequest {
    user_a_name: String,
    user_b_name: String,
    #[serde(default)]
    user_a_sid: String,
    #[serde(default)]
    user_b_sid: String,
}

async fn add_pair(State(db): State<AppState>, Json(req): Json<AddPairRequest>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.add_dual_pair(&req.user_a_sid, &req.user_b_sid, &req.user_a_name, &req.user_b_name) {
        Ok(pair) => (StatusCode::CREATED, Json(pair)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_pair(State(db): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let db = db.lock().unwrap();
    match db.remove_dual_pair(&id) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
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
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
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
    match crate::wincred::validate_and_resolve(&req.username, &req.password) {
        Ok((sid, display_name)) => Json(ValidateAccountResponse {
            success: true,
            sid,
            display_name: display_name.clone(),
            message: format!("验证成功: {}", display_name),
        }),
        Err(msg) => Json(ValidateAccountResponse {
            success: false,
            sid: String::new(),
            display_name: String::new(),
            message: msg,
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
