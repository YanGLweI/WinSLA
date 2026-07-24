//! Axum REST API server for WinSLA Management

use std::sync::{Arc, Mutex};

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Json, Response},
    routing::{delete, get},
    Router,
};
use serde::Deserialize;

use crate::database::Database;
use crate::commands;

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
