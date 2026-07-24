//! SQLite database storage for WinSLA management
//!
//! Stores dual-account pairing rules, emergency accounts, and audit logs.

use rusqlite::{Connection, Result as SqliteResult, params};
use serde::{Deserialize, Serialize};

/// Database manager for WinSLA policy storage
pub struct Database {
    conn: Connection,
}

/// Dual-account pair record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualPairRecord {
    pub id: String,
    pub user_a_sid: String,
    pub user_b_sid: String,
    pub user_a_name: String,
    pub user_b_name: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Emergency override account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyAccount {
    pub id: String,
    pub sid: String,
    pub username: String,
    pub reason: String,
    pub approved_by: String,
    pub activated_at: String,
    pub expires_at: Option<String>,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub timestamp: String,
    pub user_a_sid: String,
    pub user_b_sid: String,
    pub result: String,
    pub error_message: Option<String>,
    pub client_hostname: Option<String>,
}

/// Policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub max_retry_count: u32,
    pub auth_timeout_secs: u64,
    pub allow_emergency_override: bool,
    pub emergency_requires_reason: bool,
    pub offline_cache_enabled: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            max_retry_count: 3,
            auth_timeout_secs: 30,
            allow_emergency_override: true,
            emergency_requires_reason: true,
            offline_cache_enabled: true,
        }
    }
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(path: &str) -> SqliteResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Open in-memory database (for testing)
    pub fn open_in_memory() -> SqliteResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.initialize_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn initialize_schema(&self) -> SqliteResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS dual_pairs (
                id TEXT PRIMARY KEY,
                user_a_sid TEXT NOT NULL,
                user_b_sid TEXT NOT NULL,
                user_a_name TEXT NOT NULL DEFAULT '',
                user_b_name TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(user_a_sid, user_b_sid)
            );

            CREATE TABLE IF NOT EXISTS emergency_accounts (
                id TEXT PRIMARY KEY,
                sid TEXT NOT NULL,
                username TEXT NOT NULL,
                reason TEXT NOT NULL DEFAULT '',
                approved_by TEXT NOT NULL DEFAULT '',
                activated_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT
            );

            CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                user_a_sid TEXT NOT NULL,
                user_b_sid TEXT NOT NULL,
                result TEXT NOT NULL,
                error_message TEXT,
                client_hostname TEXT
            );

            CREATE TABLE IF NOT EXISTS policy_config (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp);
            CREATE INDEX IF NOT EXISTS idx_dual_pairs_enabled ON dual_pairs(enabled);
            ",
        )?;
        Ok(())
    }

    // ========================================================================
    // Dual Pairs CRUD
    // ========================================================================

    /// Add a new dual-account pair
    pub fn add_dual_pair(
        &self,
        user_a_sid: &str,
        user_b_sid: &str,
        user_a_name: &str,
        user_b_name: &str,
    ) -> SqliteResult<DualPairRecord> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO dual_pairs (id, user_a_sid, user_b_sid, user_a_name, user_b_name, enabled, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6)",
            params![id, user_a_sid, user_b_sid, user_a_name, user_b_name, now],
        )?;

        Ok(DualPairRecord {
            id,
            user_a_sid: user_a_sid.to_string(),
            user_b_sid: user_b_sid.to_string(),
            user_a_name: user_a_name.to_string(),
            user_b_name: user_b_name.to_string(),
            enabled: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// Get all dual pairs
    pub fn get_all_dual_pairs(&self) -> SqliteResult<Vec<DualPairRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, user_a_sid, user_b_sid, user_a_name, user_b_name, enabled, created_at, updated_at
             FROM dual_pairs ORDER BY created_at DESC",
        )?;

        let pairs = stmt.query_map([], |row| {
            Ok(DualPairRecord {
                id: row.get(0)?,
                user_a_sid: row.get(1)?,
                user_b_sid: row.get(2)?,
                user_a_name: row.get(3)?,
                user_b_name: row.get(4)?,
                enabled: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        })?;

        pairs.collect()
    }

    /// Remove a dual pair by ID
    pub fn remove_dual_pair(&self, pair_id: &str) -> SqliteResult<bool> {
        let affected = self.conn.execute(
            "DELETE FROM dual_pairs WHERE id = ?1",
            params![pair_id],
        )?;
        Ok(affected > 0)
    }

    /// Enable or disable a dual pair
    pub fn set_dual_pair_enabled(&self, pair_id: &str, enabled: bool) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE dual_pairs SET enabled = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![enabled as i32, pair_id],
        )?;
        Ok(())
    }

    // ========================================================================
    // Emergency Accounts
    // ========================================================================

    /// Add an emergency override account
    pub fn add_emergency_account(
        &self,
        sid: &str,
        username: &str,
        reason: &str,
        approved_by: &str,
    ) -> SqliteResult<EmergencyAccount> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO emergency_accounts (id, sid, username, reason, approved_by, activated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, sid, username, reason, approved_by, now],
        )?;

        Ok(EmergencyAccount {
            id,
            sid: sid.to_string(),
            username: username.to_string(),
            reason: reason.to_string(),
            approved_by: approved_by.to_string(),
            activated_at: now,
            expires_at: None,
        })
    }

    /// Get all emergency accounts
    pub fn get_emergency_accounts(&self) -> SqliteResult<Vec<EmergencyAccount>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, sid, username, reason, approved_by, activated_at, expires_at
             FROM emergency_accounts ORDER BY activated_at DESC",
        )?;

        let accounts = stmt.query_map([], |row| {
            Ok(EmergencyAccount {
                id: row.get(0)?,
                sid: row.get(1)?,
                username: row.get(2)?,
                reason: row.get(3)?,
                approved_by: row.get(4)?,
                activated_at: row.get(5)?,
                expires_at: row.get(6)?,
            })
        })?;

        accounts.collect()
    }

    // ========================================================================
    // Audit Log
    // ========================================================================

    /// Add an audit log entry
    pub fn add_audit_entry(
        &self,
        user_a_sid: &str,
        user_b_sid: &str,
        result: &str,
        error_message: Option<&str>,
        client_hostname: Option<&str>,
    ) -> SqliteResult<i64> {
        self.conn.execute(
            "INSERT INTO audit_log (user_a_sid, user_b_sid, result, error_message, client_hostname)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user_a_sid, user_b_sid, result, error_message, client_hostname],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get recent audit log entries
    pub fn get_audit_log(&self, limit: u32) -> SqliteResult<Vec<AuditLogEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, timestamp, user_a_sid, user_b_sid, result, error_message, client_hostname
             FROM audit_log ORDER BY timestamp DESC LIMIT ?1",
        )?;

        let entries = stmt.query_map(params![limit], |row| {
            Ok(AuditLogEntry {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                user_a_sid: row.get(2)?,
                user_b_sid: row.get(3)?,
                result: row.get(4)?,
                error_message: row.get(5)?,
                client_hostname: row.get(6)?,
            })
        })?;

        entries.collect()
    }

    // ========================================================================
    // Policy Configuration
    // ========================================================================

    /// Get policy configuration
    pub fn get_policy(&self) -> SqliteResult<PolicyConfig> {
        let mut config = PolicyConfig::default();

        let mut stmt = self.conn.prepare("SELECT key, value FROM policy_config")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (key, value) = row?;
            match key.as_str() {
                "max_retry_count" => config.max_retry_count = value.parse().unwrap_or(3),
                "auth_timeout_secs" => config.auth_timeout_secs = value.parse().unwrap_or(30),
                "allow_emergency_override" => config.allow_emergency_override = value == "true",
                "emergency_requires_reason" => config.emergency_requires_reason = value == "true",
                "offline_cache_enabled" => config.offline_cache_enabled = value == "true",
                _ => {}
            }
        }

        Ok(config)
    }

    /// Save policy configuration
    pub fn save_policy(&self, config: &PolicyConfig) -> SqliteResult<()> {
        let entries = vec![
            ("max_retry_count", config.max_retry_count.to_string()),
            ("auth_timeout_secs", config.auth_timeout_secs.to_string()),
            ("allow_emergency_override", config.allow_emergency_override.to_string()),
            ("emergency_requires_reason", config.emergency_requires_reason.to_string()),
            ("offline_cache_enabled", config.offline_cache_enabled.to_string()),
        ];

        for (key, value) in entries {
            self.conn.execute(
                "INSERT OR REPLACE INTO policy_config (key, value) VALUES (?1, ?2)",
                params![key, value],
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_creation() {
        let db = Database::open_in_memory().unwrap();
        let pairs = db.get_all_dual_pairs().unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_add_and_get_dual_pair() {
        let db = Database::open_in_memory().unwrap();

        let pair = db
            .add_dual_pair("S-1-5-21-user-a", "S-1-5-21-user-b", "Alice", "Bob")
            .unwrap();
        assert_eq!(pair.user_a_name, "Alice");
        assert_eq!(pair.user_b_name, "Bob");
        assert!(pair.enabled);

        let pairs = db.get_all_dual_pairs().unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].user_a_sid, "S-1-5-21-user-a");
    }

    #[test]
    fn test_remove_dual_pair() {
        let db = Database::open_in_memory().unwrap();

        let pair = db
            .add_dual_pair("S-1-5-21-a", "S-1-5-21-b", "A", "B")
            .unwrap();

        assert!(db.remove_dual_pair(&pair.id).unwrap());
        assert!(!db.remove_dual_pair("nonexistent").unwrap());

        let pairs = db.get_all_dual_pairs().unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_audit_log() {
        let db = Database::open_in_memory().unwrap();

        db.add_audit_entry("S-1-5-a", "S-1-5-b", "success", None, Some("PC01"))
            .unwrap();
        db.add_audit_entry("S-1-5-a", "S-1-5-b", "fail_user_a", Some("Wrong password"), Some("PC01"))
            .unwrap();

        let entries = db.get_audit_log(10).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].result, "fail_user_a"); // Most recent first
    }

    #[test]
    fn test_policy_config() {
        let db = Database::open_in_memory().unwrap();

        let config = PolicyConfig {
            max_retry_count: 5,
            auth_timeout_secs: 60,
            allow_emergency_override: false,
            emergency_requires_reason: true,
            offline_cache_enabled: false,
        };
        db.save_policy(&config).unwrap();

        let loaded = db.get_policy().unwrap();
        assert_eq!(loaded.max_retry_count, 5);
        assert_eq!(loaded.auth_timeout_secs, 60);
        assert!(!loaded.allow_emergency_override);
        assert!(!loaded.offline_cache_enabled);
    }

    #[test]
    fn test_emergency_accounts() {
        let db = Database::open_in_memory().unwrap();

        let account = db
            .add_emergency_account("S-1-5-admin", "admin", "System maintenance", "CTO")
            .unwrap();
        assert_eq!(account.username, "admin");

        let accounts = db.get_emergency_accounts().unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].reason, "System maintenance");
    }
}
