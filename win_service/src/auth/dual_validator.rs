//! Dual account verification logic

use tokio::join;
use crate::auth::{AuthError, AuthResult};
use log::{debug, error, info};

/// Validate two accounts' credentials concurrently
pub async fn validate_dual_accounts(
    user_a_username: &str,
    user_a_password: &[u8],
    user_b_username: &str,
    user_b_password: &[u8],
) -> Result<(), AuthError> {
    debug!("Starting dual account verification for {} and {}", 
           user_a_username, user_b_username);
    
    // Perform both verifications in parallel
    let (user_a_result, user_b_result) = join!(
        verify_single_account(user_a_username, user_a_password),
        verify_single_account(user_b_username, user_b_password)
    );
    
    match (user_a_result, user_b_result) {
        (Ok(_), Ok(_)) => {
            info!("Both users authenticated successfully");
            Ok(())
        }
        (Err(ea), Ok(_)) => {
            error!("User A failed: {}", ea);
            Err(AuthError::InvalidCredentials(ea.to_string()))
        }
        (Ok(_), Err(eb)) => {
            error!("User B failed: {}", eb);
            Err(AuthError::InvalidCredentials(eb.to_string()))
        }
        (Err(ea), Err(eb)) => {
            error!("Both users failed: A={}, B={}", ea, eb);
            Err(AuthError::BothFailed(ea.to_string(), eb.to_string()))
        }
    }
}

/// Verify a single account's credentials using LDAP
async fn verify_single_account(username: &str, password: &[u8]) -> Result<(), AuthError> {
    debug!("Verifying credentials for user: {}", username);
    
    // Try LDAP authentication first
    match try_ldap_auth(username, password).await {
        Ok(_) => {
            info!("LDAP auth succeeded for {}", username);
            Ok(())
        }
        Err(e) => {
            debug!("LDAP auth failed, trying fallback: {}", e);
            
            // Try SSPI as fallback
            match try_sspi_auth(username, password).await {
                Ok(_) => {
                    info!("SSPI auth succeeded for {}", username);
                    Ok(())
                }
                Err(_) => {
                    Err(e) // Return LDAP error as primary
                }
            }
        }
    }
}

/// LDAP Simple Bind authentication
async fn try_ldap_auth(username: &str, password: &[u8]) -> Result<(), AuthError> {
    // This is a placeholder - production code needs proper LDAP implementation
    // We'll need to link against OpenLDAP or use a Rust LDAP library
    
    // Simulated check for development
    if password.is_empty() || username.is_empty() {
        return Err(AuthError::InvalidCredentials("Empty credentials".to_string()));
    }
    
    // In production, this would:
    // 1. Connect to domain controller via LDAP/LDAPS
    // 2. Perform simple bind with the DN of the user
    // 3. Check if the bind succeeds
    
    // For dev, we accept any non-empty credentials
    // TODO: Implement real LDAP binding
    debug!("LDAP auth simulated for {}", username);
    Ok(())
}

/// SSPI (NTLM/Kerberos) authentication
async fn try_sspi_auth(username: &str, password: &[u8]) -> Result<(), AuthError> {
    // Placeholder for SSPI implementation
    // Would use windows-rs to call InitializeSecurityContext
    
    if password.is_empty() || username.is_empty() {
        return Err(AuthError::InvalidCredentials("Empty credentials".to_string()));
    }
    
    debug!("SSPI auth simulated for {}", username);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_both_success() {
        let result = validate_dual_accounts(
            "user1@DOMAIN.COM", b"pass1",
            "user2@DOMAIN.COM", b"pass2",
        ).await;
        
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_user_a_fail() {
        let result = validate_dual_accounts(
            "invalid", b"",
            "user2@DOMAIN.COM", b"pass2",
        ).await;
        
        assert!(result.is_err());
    }
}
