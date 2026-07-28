//! Dual account verification logic

use tokio::join;
use crate::auth::{AuthError, AuthResult};
use crate::audit::AuditDb;
use log::{debug, error, info, warn};

/// Validate two accounts' credentials concurrently using real Windows logon
pub async fn validate_dual_accounts(
    user_a_username: &str,
    user_a_password: &str,
    user_b_username: &str,
    user_b_password: &str,
) -> Result<(), AuthError> {
    debug!("Starting dual account verification for {} and {}",
           user_a_username, user_b_username);

    // LogonUserW may block on DC communication, so run both verifications on
    // the blocking thread pool in parallel.
    let (a_user, a_pass) = (user_a_username.to_string(), user_a_password.to_string());
    let (b_user, b_pass) = (user_b_username.to_string(), user_b_password.to_string());

    let (handle_a, handle_b) = join!(
        tokio::task::spawn_blocking(move || verify_password_windows(&a_user, &a_pass)),
        tokio::task::spawn_blocking(move || verify_password_windows(&b_user, &b_pass)),
    );

    let user_a_result = handle_a.map_err(|e| AuthError::ServiceError(format!("verify task failed: {}", e)))?;
    let user_b_result = handle_b.map_err(|e| AuthError::ServiceError(format!("verify task failed: {}", e)))?;

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

/// Verify a Windows account password with LogonUserW (network logon type).
///
/// Supported username formats:
///   "DOMAIN\\user"        -> domain=DOMAIN, user=user
///   "user@domain.suffix"  -> UPN, passed as-is with NULL domain
///   "user"                -> local account database (NULL domain)
pub fn verify_password_windows(username: &str, password: &str) -> Result<(), AuthError> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::Security::{
        LogonUserW, LOGON32_LOGON_NETWORK, LOGON32_PROVIDER_DEFAULT,
    };

    if username.trim().is_empty() || password.is_empty() {
        return Err(AuthError::InvalidCredentials("用户名或密码为空".to_string()));
    }

    let (user_part, domain_part): (String, Option<String>) = if let Some(pos) = username.find('\\') {
        let domain = username[..pos].to_string();
        let user = username[pos + 1..].to_string();
        (user, if domain.is_empty() { None } else { Some(domain) })
    } else {
        (username.to_string(), None)
    };

    let user_w: Vec<u16> = user_part.encode_utf16().chain(std::iter::once(0)).collect();
    let pass_w: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();
    let domain_w: Vec<u16> = domain_part
        .as_deref()
        .unwrap_or("")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let domain_pcwstr = if domain_part.is_some() {
        PCWSTR(domain_w.as_ptr())
    } else {
        PCWSTR::null()
    };

    let mut handle = HANDLE::default();
    let result = unsafe {
        LogonUserW(
            PCWSTR(user_w.as_ptr()),
            domain_pcwstr,
            PCWSTR(pass_w.as_ptr()),
            LOGON32_LOGON_NETWORK,
            LOGON32_PROVIDER_DEFAULT,
            &mut handle,
        )
    };

    match result {
        Ok(()) => {
            unsafe {
                let _ = CloseHandle(handle);
            }
            Ok(())
        }
        Err(e) => {
            // HRESULT_FROM_WIN32: low 16 bits carry the Win32 error code
            let hr = e.code().0 as u32;
            let win32_code = hr & 0xFFFF;
            warn!("LogonUserW failed for {}: HRESULT=0x{:08X} win32={}", username, hr, win32_code);
            Err(AuthError::InvalidCredentials(map_logon_error(win32_code)))
        }
    }
}

/// Map common Win32 logon error codes to user-facing Chinese messages
fn map_logon_error(win32_code: u32) -> String {
    match win32_code {
        1326 => "用户名或密码错误".to_string(),
        1327 => "账号受限：不允许空密码登录".to_string(),
        1331 => "账号已被禁用".to_string(),
        1907 => "密码已过期，必须先更改密码".to_string(),
        1909 => "账号已被域策略锁定".to_string(),
        1385 => "该账号没有网络登录权限".to_string(),
        1355 => "域控制器不可用或域名无效".to_string(),
        _ => format!("凭据验证失败 (错误码 {})", win32_code),
    }
}

/// Extract bare username from various formats: "HOT\ylw" / "ylw@hot.local" / "ylw" → "ylw"
pub(crate) fn extract_bare_username(input: &str) -> String {
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
    
    // If no pairing rules are configured, reject dual-auth login and direct
    // users to the Windows default tile (which stays enabled until the first
    // pairing rule is created). This prevents arbitrary domain-account pairs
    // from bypassing the dual-control policy on unconfigured machines.
    if pairs.is_empty() {
        warn!("No pairing rules configured, rejecting dual-auth login");
        return Err(AuthError::InvalidCredentials(
            "系统未配置配对规则，请使用 Windows 默认登录方式".to_string()
        ));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_credentials_rejected() {
        assert!(verify_password_windows("", "pass").is_err());
        assert!(verify_password_windows("user", "").is_err());
    }

    #[test]
    fn test_error_mapping() {
        assert!(map_logon_error(1326).contains("密码错误"));
        assert!(map_logon_error(1909).contains("锁定"));
    }

    #[tokio::test]
    async fn test_empty_password_fails() {
        let result = validate_dual_accounts(
            "user1", "",
            "user2", "pass2",
        ).await;

        assert!(result.is_err());
    }
}
