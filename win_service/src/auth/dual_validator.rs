//! Dual account verification logic

use tokio::join;
use crate::auth::{AuthError, AuthResult};
use crate::audit::AuditDb;
use log::{debug, error, info, warn};

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

/// Extract bare username from various formats: "HOT\ylw" / "ylw@hot.local" / "ylw" → "ylw"
fn extract_bare_username(input: &str) -> String {
    if let Some(pos) = input.find('\\') {
        input[pos + 1..].to_lowercase()
    } else if let Some(pos) = input.find('@') {
        input[..pos].to_lowercase()
    } else {
        input.to_lowercase()
    }
}

/// Check pairing rules (strict order validation)
pub async fn check_pairing_rule(account_username: &str, approver_username: &str) -> Result<(), AuthError> {
    warn!("Checking pairing rules: account={}, approver={}", account_username, approver_username);
    
    // Read all enabled pairing rules from shared database
    let audit_db = match AuditDb::open() {
        Ok(db) => db,
        Err(e) => {
            warn!("Failed to open audit database: {}. Allowing login anyway.", e);
            return Ok(());
        }
    };
    
    let pairs = match audit_db.get_enabled_pairs() {
        Ok(pairs) => pairs,
        Err(e) => {
            warn!("Failed to read pairing rules: {}. Allowing login anyway.", e);
            return Ok(());
        }
    };
    
    // If no pairing rules are configured, allow any domain accounts to login
    if pairs.is_empty() {
        info!("No pairing rules configured, allowing any domain accounts");
        return Ok(());
    }
    
    // Normalize input usernames (strip domain, lowercase)
    let account_bare = extract_bare_username(account_username);
    let approver_bare = extract_bare_username(approver_username);
    
    // Check if current username combination is in valid pairs (strict order: account + approver)
    for (_account_sid, _approver_sid, pair_account_name, pair_approver_name) in &pairs {
        if extract_bare_username(pair_account_name) == account_bare
            && extract_bare_username(pair_approver_name) == approver_bare {
            info!("Pairing rule matched: {} (account) + {} (approver)", pair_account_name, pair_approver_name);
            return Ok(());
        }
    }
    
    // Pair not found - reject with Chinese error message
    let msg = "主账号与审批人不匹配：该组合不在有效配对列表中".to_string();
    warn!("Pairing rule rejected: {} + {} not in valid pairs", account_username, approver_username);
    Err(AuthError::InvalidCredentials(msg))
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
