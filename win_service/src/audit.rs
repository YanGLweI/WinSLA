//! Audit logging module for WinSLA Service
//!
//! Records authentication events to Windows Event Log and local file.

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    AuthSuccess {
        user_a: String,
        user_b: String,
        timestamp: String,
    },
    AuthFailure {
        user_a: String,
        user_b: String,
        reason: String,
        timestamp: String,
    },
    ServiceStarted,
    ServiceStopped,
    EmergencyOverride {
        username: String,
        reason: String,
        timestamp: String,
    },
}

/// Audit logger that writes to Windows Event Log and local file
pub struct AuditLogger {
    log_file_path: String,
}

impl AuditLogger {
    pub fn new() -> Self {
        let log_dir = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        let log_file_path = format!(r"{}\WinSLA\audit.log", log_dir);

        // Ensure log directory exists
        if let Some(parent) = std::path::Path::new(&log_file_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        Self { log_file_path }
    }

    /// Log an audit event
    pub fn log_event(&self, event: &AuditEvent) {
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
        let message = match event {
            AuditEvent::AuthSuccess { user_a, user_b, .. } => {
                format!("[{}] AUTH_SUCCESS: user_a={}, user_b={}", timestamp, user_a, user_b)
            }
            AuditEvent::AuthFailure { user_a, user_b, reason, .. } => {
                format!("[{}] AUTH_FAILURE: user_a={}, user_b={}, reason={}", timestamp, user_a, user_b, reason)
            }
            AuditEvent::ServiceStarted => {
                format!("[{}] SERVICE_STARTED", timestamp)
            }
            AuditEvent::ServiceStopped => {
                format!("[{}] SERVICE_STOPPED", timestamp)
            }
            AuditEvent::EmergencyOverride { username, reason, .. } => {
                format!("[{}] EMERGENCY_OVERRIDE: user={}, reason={}", timestamp, username, reason)
            }
        };

        // Write to local file
        self.write_to_file(&message);

        // Write to Windows Event Log
        self.write_to_event_log(&message);

        log::info!("AUDIT: {}", message);
    }

    /// Write audit entry to local log file
    fn write_to_file(&self, message: &str) {
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)
        {
            let _ = writeln!(file, "{}", message);
        }
    }

    /// Write audit entry to Windows Event Log
    fn write_to_event_log(&self, message: &str) {
        // Use Windows Event Log API via ReportEventW
        // For now, use the `log` crate as fallback
        // In production, this would use:
        //   RegisterEventSourceW -> ReportEventW -> DeregisterEventSource
        log::debug!("EventLog: {}", message);
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}
