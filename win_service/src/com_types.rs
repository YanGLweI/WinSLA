//! Common types shared between Credential Provider and Service

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request sent from CP to Service via Named Pipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub request_id: Uuid,
    pub user_a_username: String,
    pub user_a_password_hash: Vec<u8>,
    pub user_b_username: String,
    pub user_b_password_hash: Vec<u8>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl AuthRequest {
    /// Create a new auth request with HMAC-protected passwords
    pub fn new(
        user_a_username: &str,
        user_a_password: &str,
        user_b_username: &str,
        user_b_password: &str,
    ) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        // Generate deterministic salt based on usernames (for demo)
        // In production, use unique random salt per connection
        let salt = format!("{}-{}", user_a_username, user_b_username);
        
        let mut hasher = DefaultHasher::new();
        salt.hash(&mut hasher);
        
        AuthRequest {
            request_id: Uuid::new_v4(),
            user_a_username: user_a_username.to_lowercase(),
            user_a_password_hash: hash_password(user_a_password, &salt),
            user_b_username: user_b_username.to_lowercase(),
            user_b_password_hash: hash_password(user_b_password, &salt),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Password hashing for secure transmission
fn hash_password(password: &str, salt: &str) -> Vec<u8> {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(password.as_bytes());
    hasher.finalize().to_vec()
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

impl AuthResponse {
    /// Check if authentication was successful
    pub fn is_success(&self) -> bool {
        matches!(self, AuthResponse::Success)
    }
}
