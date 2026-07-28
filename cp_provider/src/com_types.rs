//! COM type definitions for Credential Provider

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Authentication mode requested by the Credential Provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthMode {
    /// Dual control login: primary account + approver
    Dual,
    /// Emergency override login: single authorized account with a reason
    Emergency,
}

/// Request sent from CP to Service via Named Pipe
///
/// Passwords travel as plaintext over the local named pipe. Both endpoints run
/// as SYSTEM on the same machine (LogonUI hosts the CP; the service runs as
/// LocalSystem), so this does not expose credentials beyond the trust boundary
/// Windows already uses for interactive logon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub request_id: Uuid,
    pub mode: AuthMode,
    /// Dual: primary account username; Emergency: emergency account username
    pub user_a_username: String,
    /// Plaintext password for user_a
    pub user_a_password: String,
    /// Dual: approver username; Emergency: empty
    pub user_b_username: String,
    /// Dual: approver plaintext password; Emergency: empty
    pub user_b_password: String,
    /// Emergency: reason for override; Dual: empty
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Response sent from Service back to CP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthResponse {
    /// Dual auth succeeded OR emergency override approved
    Success,
    FailUserA(String),      // Error message for primary account failure
    FailUserB(String),      // Error message for approver failure
    BothFailed(String, String),
    /// Account locked out after too many failed attempts
    Locked { remaining_secs: u64 },
    /// Emergency override denied (policy disabled / not authorized / missing reason)
    EmergencyDenied(String),
    Timeout,
    NetworkUnavailable,
}

impl AuthRequest {
    /// Create a dual-control authentication request
    pub fn new_dual(
        user_a_username: &str,
        user_a_password: &str,
        user_b_username: &str,
        user_b_password: &str,
    ) -> Self {
        AuthRequest {
            request_id: Uuid::new_v4(),
            mode: AuthMode::Dual,
            user_a_username: user_a_username.to_lowercase(),
            user_a_password: user_a_password.to_string(),
            user_b_username: user_b_username.to_lowercase(),
            user_b_password: user_b_password.to_string(),
            reason: String::new(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create an emergency override authentication request
    pub fn new_emergency(username: &str, password: &str, reason: &str) -> Self {
        AuthRequest {
            request_id: Uuid::new_v4(),
            mode: AuthMode::Emergency,
            user_a_username: username.to_lowercase(),
            user_a_password: password.to_string(),
            user_b_username: String::new(),
            user_b_password: String::new(),
            reason: reason.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub request_id: Uuid,
    pub success: bool,
    pub error_message: Option<String>,
    pub audit_data: Option<AuditData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditData {
    pub user_a_sid: Option<String>,
    pub user_b_sid: Option<String>,
    pub verification_timestamp: chrono::DateTime<chrono::Utc>,
    pub result: VerificationResult,
    pub client_hostname: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationResult {
    Success,
    FailUserA,
    FailUserB,
    BothFailed,
    Timeout,
    NetworkError,
}
