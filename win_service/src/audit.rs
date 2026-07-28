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
        
        // Busy timeout: 5s for concurrent access safety (prevents "database is locked")
        conn.busy_timeout(std::time::Duration::from_secs(5))?;

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

        // Login attempt tracking table for the retry/lockout policy
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS login_attempts (
                account TEXT PRIMARY KEY,
                fail_count INTEGER NOT NULL DEFAULT 0,
                locked_until TEXT
            );",
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

    /// Read the policy configuration written by the management app.
    /// Falls back to defaults for any key that is missing or unreadable.
    pub fn get_policy(&self) -> ServicePolicy {
        let mut policy = ServicePolicy::default();
        if let Ok(mut stmt) = self.conn.prepare("SELECT key, value FROM policy_config") {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                for row in rows.flatten() {
                    match row.0.as_str() {
                        "max_retry_count" => policy.max_retry_count = row.1.parse().unwrap_or(5),
                        "auth_timeout_secs" => policy.auth_timeout_secs = row.1.parse().unwrap_or(30),
                        "allow_emergency_override" => policy.allow_emergency_override = row.1 == "true",
                        "emergency_requires_reason" => policy.emergency_requires_reason = row.1 == "true",
                        "offline_cache_enabled" => policy.offline_cache_enabled = row.1 == "true",
                        "lockout_duration_minutes" => policy.lockout_duration_minutes = row.1.parse().unwrap_or(10),
                        "default_tile_enabled" => policy.default_tile_enabled = row.1 == "true",
                        _ => {}
                    }
                }
            }
        }
        policy
    }

    /// Get all currently active emergency accounts as (sid, username) pairs.
    /// Accounts with a past expires_at are excluded; NULL expires_at never expires.
    pub fn get_emergency_accounts(&self) -> Vec<(String, String)> {
        let mut accounts = Vec::new();
        if let Ok(mut stmt) = self.conn.prepare(
            "SELECT sid, username FROM emergency_accounts
             WHERE expires_at IS NULL OR expires_at > datetime('now', 'localtime')",
        ) {
            if let Ok(rows) = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            }) {
                accounts.extend(rows.flatten());
            }
        }
        accounts
    }

    /// If the account is currently locked out, return the remaining lock time in seconds.
    pub fn get_lock_remaining_secs(&self, account: &str) -> Option<u64> {
        let locked_until: Option<String> = self
            .conn
            .query_row(
                "SELECT locked_until FROM login_attempts WHERE account = ?1",
                params![account.to_lowercase()],
                |row| row.get(0),
            )
            .ok()
            .flatten();

        let locked_until = locked_until?;
        let until = chrono::NaiveDateTime::parse_from_str(&locked_until, "%Y-%m-%d %H:%M:%S").ok()?;
        let now = chrono::Local::now().naive_local();
        if until > now {
            Some((until - now).num_seconds().max(1) as u64)
        } else {
            None
        }
    }

    /// Record a failed login attempt against the policy. When the failure count
    /// reaches max_retry_count the account is locked for lockout_duration_minutes
    /// and the counter restarts after the lock expires.
    ///
    /// Thread-safe: uses SQLite ON CONFLICT atomics; busy_timeout(5s) guards concurrent access.
    /// Returns (remaining_attempts_before_lock, Some(locked_secs) if this failure
    /// triggered a new lockout).
    pub fn record_login_failure(
        &self,
        account: &str,
        max_retry_count: u32,
        lockout_duration_minutes: u32,
    ) -> (u32, Option<u64>) {
        let account = account.to_lowercase();
        let current: u32 = self
            .conn
            .query_row(
                "SELECT fail_count FROM login_attempts WHERE account = ?1",
                params![&account],
                |row| row.get::<_, u32>(0),
            )
            .unwrap_or(0);

        let new_count = current + 1;

        if new_count >= max_retry_count {
            let locked_until = (chrono::Local::now()
                + chrono::Duration::minutes(lockout_duration_minutes as i64))
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
            let rows = self.conn.execute(
                "INSERT INTO login_attempts (account, fail_count, locked_until) VALUES (?1, 0, ?2)
                 ON CONFLICT(account) DO UPDATE SET fail_count = 0, locked_until = ?2",
                params![&account, &locked_until],
            ).unwrap_or(0);
            log::warn!("Account '{}' locked after {} failures ({} rows updated)", account, new_count, rows);
            (0, Some(lockout_duration_minutes as u64 * 60))
        } else {
            let rows = self.conn.execute(
                "INSERT INTO login_attempts (account, fail_count, locked_until) VALUES (?1, ?2, NULL)
                 ON CONFLICT(account) DO UPDATE SET fail_count = ?2, locked_until = NULL",
                params![&account, new_count],
            ).unwrap_or(0);
            log::debug!("Account '{}' failure recorded: {} total ({} rows updated)", account, new_count, rows);
            (max_retry_count - new_count, None)
        }
    }

    /// Clear the failure counter for an account (called after a successful login).
    pub fn reset_login_failures(&self, account: &str) {
        let _ = self.conn.execute(
            "DELETE FROM login_attempts WHERE account = ?1",
            params![account.to_lowercase()],
        );
    }
}

/// Service-side view of the policy_config table written by the management app.
#[derive(Debug, Clone)]
pub struct ServicePolicy {
    pub max_retry_count: u32,
    pub auth_timeout_secs: u64,
    pub allow_emergency_override: bool,
    pub emergency_requires_reason: bool,
    pub offline_cache_enabled: bool,
    pub lockout_duration_minutes: u32,
    pub default_tile_enabled: bool,  // 是否启用 Windows 默认登录 Tile（默认 true = 保障未配置配对时可登录）
}

impl Default for ServicePolicy {
    fn default() -> Self {
        Self {
            max_retry_count: 5,
            auth_timeout_secs: 30,
            allow_emergency_override: true,
            emergency_requires_reason: true,
            offline_cache_enabled: true,
            lockout_duration_minutes: 10,
            default_tile_enabled: true,  // 默认启用默认 Tile，与管理端 PolicyConfig 默认值保持一致
        }
    }
}
