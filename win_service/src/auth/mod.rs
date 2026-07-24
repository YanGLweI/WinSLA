//! Authentication module for AD domain credential verification

pub mod dual_validator;
pub mod sspi_verifier;
pub mod ldap_verifier;
pub mod emergency;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Authentication error types
#[derive(Error, Debug, Serialize, Deserialize)]
pub enum AuthError {
    #[error("LDAP connection failed: {0}")]
    LdapConnection(String),
    
    #[error("SSPI authentication failed: {0}")]
    SspiFailure(String),
    
    #[error("Network unavailable: DC unreachable")]
    NetworkUnavailable,
    
    #[error("Invalid credentials for user: {0}")]
    InvalidCredentials(String),
    
    #[error("Timeout waiting for response")]
    Timeout,
    
    #[error("Service communication error: {0}")]
    CommunicationError(String),
    
    #[error("Both users failed: {0} and {1}")]
    BothFailed(String, String),
    
    #[error("Internal service error: {0}")]
    ServiceError(String),
}

/// Result type for authentication operations
pub type AuthResult = Result<(), AuthError>;

/// Dual account verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DualVerificationResult {
    Success,
    UserAFailed(String),  // Error message
    UserBFailed(String),
    BothFailed(String, String),
    Timeout,
}
