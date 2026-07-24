//! # WinSLA Management Library
//!
//! Core logic for the WinSLA management application.
//! Provides dual-account pairing configuration, service control,
//! and audit log querying.

pub mod commands;
pub mod database;

/// Application configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub service_name: String,
    pub pipe_path: String,
    pub db_path: String,
    pub max_retry_count: u32,
    pub auth_timeout_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            service_name: "WinSLA Service".to_string(),
            pipe_path: r"\\.\pipe\winsla-auth-pipe".to_string(),
            db_path: "winsla.db".to_string(),
            max_retry_count: 3,
            auth_timeout_secs: 30,
        }
    }
}

/// Run the management application (placeholder for Tauri integration)
pub fn run_app() {
    log::info!("WinSLA Management App starting...");
    let config = AppConfig::default();
    log::info!("Config: {:?}", config);
}
