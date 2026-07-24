//! COM type definitions for Credential Provider

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request sent from CP to Service via Named Pipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub request_id: Uuid,
    pub user_a_username: String,
    pub user_a_password_hash: Vec<u8>, // Encrypted or secured
    pub user_b_username: String,
    pub user_b_password_hash: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Response sent from Service back to CP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthResponse {
    Success,
    FailUserA(String),      // Error message for User A failure
    FailUserB(String),      // Error message for User B failure
    BothFailed(String, String),
    Timeout,
    NetworkUnavailable,
}

impl AuthRequest {
    pub fn new(
        user_a_username: String,
        user_a_password: &str,
        user_b_username: String,
        user_b_password: &str,
    ) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hash;
        
        let mut hasher = DefaultHasher::new();
        format!("{}-{}", user_a_username, user_b_username).hash(&mut hasher);
        
        // In production, encrypt passwords before transmission
        let password_encryption_key = b"dev-salt-for-demo-only"; // REMOVE IN PRODUCTION
        
        AuthRequest {
            request_id: Uuid::new_v4(),
            user_a_username,
            user_a_password_hash: sha256_password(user_a_password, password_encryption_key),
            user_b_username,
            user_b_password_hash: sha256_password(user_b_password, password_encryption_key),
            timestamp: chrono::Utc::now(),
        }
    }
}

fn sha256_password(password: &str, salt: &[u8]) -> Vec<u8> {
    use sha2::Sha256;
    use hmac::{Hmac, Mac};
    
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(salt).expect("Failed to create HMAC");
    mac.update(password.as_bytes());
    mac.finalize().into_bytes().to_vec()
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
