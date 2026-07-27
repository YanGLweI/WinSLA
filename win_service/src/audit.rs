//! Audit logging module for WinSLA Service
//!
//! Records authentication events to Windows Event Log, local file, and shared SQLite DB.

use chrono::Local;
use rusqlite::{Connection, params};
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
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
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

// ─── SQLite Audit Database ────────────────────────────────────────

/// Shared database path accessible by both service (SYSTEM) and management app
const SHARED_DB_DIR: &str = r"C:\ProgramData\WinSLA";
const SHARED_DB_FILE: &str = r"C:\ProgramData\WinSLA\winsla.db";

/// SQLite-backed audit recorder shared with the management application.
///
/// Writes authentication results to the same database the management app reads,
/// enabling real-time audit log and statistics on the dashboard.
pub struct AuditDb {
    conn: Connection,
}

impl AuditDb {
    /// Open (or create) the shared audit database.
    /// Enables WAL mode for safe multi-process concurrent access.
    pub fn open() -> Result<Self, rusqlite::Error> {
        // Ensure directory exists
        let _ = std::fs::create_dir_all(SHARED_DB_DIR);

        let conn = Connection::open(SHARED_DB_FILE)?;

        // Enable WAL for concurrent read/write from service + management app
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;

        // Create audit_log table if it doesn't exist (same schema as management app)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                account_sid TEXT NOT NULL,
                approver_sid TEXT NOT NULL,
                result TEXT NOT NULL,
                error_message TEXT,
                client_hostname TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);",
        )?;

        // Migration: rename old columns if upgrading from v2.0.4
        Self::migrate_old_columns(&conn)?;

        Ok(Self { conn })
    }

    /// Migrate tables from old column names (user_a_sid/user_b_sid) to new ones (account_sid/approver_sid)
    fn migrate_old_columns(conn: &Connection) -> Result<(), rusqlite::Error> {
        // Check audit_log
        let audit_has_old: bool = {
            let mut stmt = conn.prepare("PRAGMA table_info(audit_log)")?;
            let columns: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .collect();
            columns.contains(&"user_a_sid".to_string())
        };
        if audit_has_old {
            conn.execute_batch(
                "ALTER TABLE audit_log RENAME COLUMN user_a_sid TO account_sid;
                 ALTER TABLE audit_log RENAME COLUMN user_b_sid TO approver_sid;",
            )?;
        }

        // Check dual_pairs
        let pairs_has_old: bool = {
            let mut stmt = conn.prepare("PRAGMA table_info(dual_pairs)")?;
            let columns: Vec<String> = stmt.query_map([], |row| row.get::<_, String>(1))?
                .filter_map(|r| r.ok())
                .collect();
            columns.contains(&"user_a_sid".to_string())
        };
        if pairs_has_old {
            conn.execute_batch(
                "ALTER TABLE dual_pairs RENAME COLUMN user_a_sid TO account_sid;
                 ALTER TABLE dual_pairs RENAME COLUMN user_b_sid TO approver_sid;
                 ALTER TABLE dual_pairs RENAME COLUMN user_a_name TO account_username;
                 ALTER TABLE dual_pairs RENAME COLUMN user_b_name TO approver_username;",
            )?;
        }

        Ok(())
    }

    /// Record an authentication attempt into the shared database.
    pub fn record_auth(
        &self,
        account_sid: &str,      // 主账号 SID
        approver_sid: &str,     // 审批人 SID
        result: &str,
        error_message: Option<&str>,
        client_hostname: Option<&str>,
    ) -> Result<(), rusqlite::Error> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        self.conn.execute(
            "INSERT INTO audit_log (timestamp, account_sid, approver_sid, result, error_message, client_hostname)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![timestamp, account_sid, approver_sid, result, error_message, client_hostname],
        )?;
        Ok(())
    }

    /// Get all enabled pairing rules from the shared database
    pub fn get_enabled_pairs(&self) -> Result<Vec<(String, String, String, String)>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT account_sid, approver_sid, account_username, approver_username FROM dual_pairs WHERE enabled = 1",
        )?;

        let pairs = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        pairs.collect()
    }
}
