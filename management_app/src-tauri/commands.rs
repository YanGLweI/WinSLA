//! Management commands for WinSLA
//!
//! These functions provide the core logic that will be exposed
//! to the Vue frontend via Tauri commands.

use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Service status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceStatus {
    pub running: bool,
    pub version: String,
    pub connections_accepted: u64,
    pub successful_auths: u64,
    pub failed_auths: u64,
}

/// Dual-account pair configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPair {
    pub id: String,
    pub account_sid: String,           // 主账号 SID
    pub approver_sid: String,          // 审批人 SID
    pub account_username: String,      // 主账号用户名
    pub approver_username: String,     // 审批人用户名
    pub enabled: bool,
    pub created_at: String,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub id: u64,
    pub timestamp: String,
    pub user_a: String,
    pub user_b: String,
    pub result: String,
    pub error_message: Option<String>,
}

/// Check if the WinSLA service is currently running via Windows SCM
pub fn is_service_running() -> bool {
    let output = std::process::Command::new("sc.exe")
        .args(["query", "WinSLA Service"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains("RUNNING"),
        Err(_) => false,
    }
}

/// Start the WinSLA service
pub fn start_service() -> Result<bool, String> {
    // In production: sc.exe start "WinSLA Service"
    log::info!("Starting WinSLA service...");
    Ok(true)
}

/// Stop the WinSLA service
pub fn stop_service() -> Result<bool, String> {
    // In production: sc.exe stop "WinSLA Service"
    log::info!("Stopping WinSLA service...");
    Ok(true)
}

/// Get all configured dual-account pairs
pub fn get_dual_pairs() -> Result<Vec<DualPair>, String> {
    // In production: query SQLite database
    Ok(vec![])
}

/// Add a new dual-account pair
pub fn add_dual_pair(account_sid: &str, approver_sid: &str) -> Result<DualPair, String> {
    let pair = DualPair {
        id: uuid::Uuid::new_v4().to_string(),
        account_sid: account_sid.to_string(),
        approver_sid: approver_sid.to_string(),
        account_username: String::new(),
        approver_username: String::new(),
        enabled: true,
        created_at: chrono::Local::now().to_rfc3339(),
    };
    // In production: insert into SQLite database
    Ok(pair)
}

/// Remove a dual-account pair
pub fn remove_dual_pair(pair_id: &str) -> Result<bool, String> {
    log::info!("Removing dual pair: {}", pair_id);
    // In production: delete from SQLite database
    Ok(true)
}

/// Get audit log entries
pub fn get_audit_log(limit: u32) -> Result<Vec<AuditEntry>, String> {
    let _ = limit;
    // In production: query SQLite database
    Ok(vec![])
}
