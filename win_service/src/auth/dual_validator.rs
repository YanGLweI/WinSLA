//! Dual account verification logic

use tokio::join;
use crate::auth::{AuthError, AuthResult};
use crate::audit::AuditDb;
use log::{debug, error, info, warn};

/// Validate two accounts' credentials concurrently using real Windows logon.
/// On success returns the canonical logon name ("DOMAIN\user") of user A,
/// which the Credential Provider must serialize verbatim for LSA.
pub async fn validate_dual_accounts(
    user_a_username: &str,
    user_a_password: &str,
    user_b_username: &str,
    user_b_password: &str,
) -> Result<String, AuthError> {
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
        (Ok(canonical_a), Ok(_)) => {
            info!("Both users authenticated successfully");
            Ok(canonical_a)
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
/// On success returns the canonical logon name ("DOMAIN\user") of the account.
///
/// Supported username formats:
///   "DOMAIN\\user"        -> domain=DOMAIN, user=user
///   "user@domain.suffix"  -> UPN, passed as-is with NULL domain
///   "user"                -> local account database (NULL domain)
pub fn verify_password_windows(username: &str, password: &str) -> Result<String, AuthError> {
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
            let canonical = canonical_logon_name(username);
            debug!("Canonical logon name for '{}': '{}'", username, canonical);
            Ok(canonical)
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

// ─── Canonical logon name resolution (RDP serialization fix) ──────────────

#[link(name = "netapi32")]
extern "system" {
    fn NetGetJoinInformation(
        server: *const u16,
        name_buffer: *mut *mut u16,
        buffer_type: *mut u32,
    ) -> u32;
    fn NetApiBufferFree(buffer: *mut core::ffi::c_void) -> u32;
}

/// Resolve the authoritative "DOMAIN\user" logon name for a successfully
/// verified credential, so the Credential Provider can serialize exactly the
/// account that was validated (the CP must never rebuild the domain prefix -
/// that was the root cause of RDP logons being rejected by LSA):
///   "DOMAIN\user"  -> returned as-is (explicit domain was used for verification)
///   "user@domain"  -> resolved to DOMAIN\sAMAccountName via account lookup
///   "user"         -> local SAM hit: "MACHINE\user"; otherwise "JOINED_DOMAIN\user"
fn canonical_logon_name(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.contains('\\') {
        return trimmed.to_string();
    }
    if trimmed.contains('@') {
        if let Some((name, domain)) = resolve_account_name(None, trimmed) {
            return format!("{}\\{}", domain, name);
        }
        return trimmed.to_string();
    }
    // Bare name: LogonUserW with NULL domain validates against the local SAM
    // first, so mirror that order here before falling back to the joined domain.
    if let Some((name, domain)) = resolve_account_name(Some("."), trimmed) {
        return format!("{}\\{}", domain, name);
    }
    if let Some(domain) = joined_domain_name() {
        return format!("{}\\{}", domain, trimmed);
    }
    trimmed.to_string()
}

/// Look up an account and return its canonical (sAMAccountName, domain) pair.
/// `system`: Some(".") restricts the lookup to the local account database;
/// None searches the local machine first, then trusted domains.
fn resolve_account_name(system: Option<&str>, account: &str) -> Option<(String, String)> {
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Security::{LookupAccountNameW, LookupAccountSidW, PSID, SID_NAME_USE};

    let account_w: Vec<u16> = account.encode_utf16().chain(std::iter::once(0)).collect();
    let system_w: Vec<u16> = system
        .map(|s| s.encode_utf16().chain(std::iter::once(0)).collect())
        .unwrap_or_default();
    let system_pcwstr = if system.is_some() {
        PCWSTR(system_w.as_ptr())
    } else {
        PCWSTR::null()
    };

    // Step 1: resolve the name to a SID (two-call sizing pattern)
    let mut cb_sid: u32 = 0;
    let mut cch_domain: u32 = 0;
    let mut sid_use = SID_NAME_USE(0);
    let _ = unsafe {
        LookupAccountNameW(
            system_pcwstr,
            PCWSTR(account_w.as_ptr()),
            PSID::default(),
            &mut cb_sid,
            PWSTR::null(),
            &mut cch_domain,
            &mut sid_use,
        )
    };
    if cb_sid == 0 {
        return None;
    }

    let mut sid_buf = vec![0u8; cb_sid as usize];
    let mut domain_buf = vec![0u16; cch_domain as usize];
    let found = unsafe {
        LookupAccountNameW(
            system_pcwstr,
            PCWSTR(account_w.as_ptr()),
            PSID(sid_buf.as_mut_ptr() as *mut core::ffi::c_void),
            &mut cb_sid,
            PWSTR(domain_buf.as_mut_ptr()),
            &mut cch_domain,
            &mut sid_use,
        )
    };
    if found.is_err() {
        return None;
    }

    // Step 2: resolve the SID back to the canonical (name, domain) pair
    let mut cch_name: u32 = 0;
    let mut cch_domain2: u32 = 0;
    let _ = unsafe {
        LookupAccountSidW(
            system_pcwstr,
            PSID(sid_buf.as_mut_ptr() as *mut core::ffi::c_void),
            PWSTR::null(),
            &mut cch_name,
            PWSTR::null(),
            &mut cch_domain2,
            &mut sid_use,
        )
    };
    if cch_name == 0 {
        return None;
    }
    let mut name_buf = vec![0u16; cch_name as usize];
    let mut domain_buf2 = vec![0u16; cch_domain2 as usize];
    let resolved = unsafe {
        LookupAccountSidW(
            system_pcwstr,
            PSID(sid_buf.as_mut_ptr() as *mut core::ffi::c_void),
            PWSTR(name_buf.as_mut_ptr()),
            &mut cch_name,
            PWSTR(domain_buf2.as_mut_ptr()),
            &mut cch_domain2,
            &mut sid_use,
        )
    };
    if resolved.is_err() {
        return None;
    }

    let name_end = name_buf.iter().position(|&c| c == 0).unwrap_or(name_buf.len());
    let domain_end = domain_buf2.iter().position(|&c| c == 0).unwrap_or(domain_buf2.len());
    let name = String::from_utf16_lossy(&name_buf[..name_end]);
    let domain = String::from_utf16_lossy(&domain_buf2[..domain_end]);
    if name.is_empty() || domain.is_empty() {
        return None;
    }
    Some((name, domain))
}

/// NetBIOS name of the domain this machine is joined to, or None for
/// workgroup machines. The NetGetJoinInformation buffer type must be
/// NetSetupDomainName(3) - a workgroup name is NOT a valid logon domain and
/// must never be prefixed onto the username (RDP "WORKGROUP\user" bug).
fn joined_domain_name() -> Option<String> {
    let mut name_buf: *mut u16 = std::ptr::null_mut();
    let mut buf_type: u32 = 0;
    let status = unsafe { NetGetJoinInformation(std::ptr::null(), &mut name_buf, &mut buf_type) };
    if status != 0 || name_buf.is_null() {
        return None;
    }
    let domain = unsafe {
        let mut len = 0usize;
        while *name_buf.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(name_buf, len))
    };
    unsafe { NetApiBufferFree(name_buf as *mut core::ffi::c_void) };
    // NETSETUP_JOIN_STATUS: 0=Unknown 1=Unjoined 2=WorkgroupName 3=DomainName
    if buf_type != 3 || domain.is_empty() {
        return None;
    }
    Some(domain)
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
