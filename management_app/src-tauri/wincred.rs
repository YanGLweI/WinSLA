//! Windows credential validation and SID resolution
//!
//! Uses LogonUserW to validate credentials against AD,
//! then LookupAccountNameW to resolve the SID.

use windows::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL};
use windows::Win32::Security::{
    LogonUserW, LookupAccountNameW, PSID, LOGON32_LOGON_NETWORK, LOGON32_PROVIDER_DEFAULT,
    SID_NAME_USE,
};
use windows::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows::core::{PCWSTR, PWSTR};

/// Validate credentials via LogonUserW and resolve SID via LookupAccountNameW
/// Returns (sid_string, display_name) on success
pub fn validate_and_resolve(username: &str, password: &str) -> Result<(String, String), String> {
    unsafe {
        // Parse domain\user or user@domain format
        let (domain, user) = parse_username(username);

        let domain_wide: Vec<u16> = domain.encode_utf16().chain(std::iter::once(0)).collect();
        let user_wide: Vec<u16> = user.encode_utf16().chain(std::iter::once(0)).collect();
        let pass_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();

        let domain_pcwstr = if domain.is_empty() { PCWSTR::null() } else { PCWSTR(domain_wide.as_ptr()) };
        let user_pcwstr = PCWSTR(user_wide.as_ptr());
        let pass_pcwstr = PCWSTR(pass_wide.as_ptr());

        // Step 1: Validate credentials with LogonUserW
        let mut token = HANDLE::default();
        let result = LogonUserW(
            user_pcwstr,
            domain_pcwstr,
            pass_pcwstr,
            LOGON32_LOGON_NETWORK,
            LOGON32_PROVIDER_DEFAULT,
            &mut token,
        );

        if result.is_err() {
            let err = result.unwrap_err().code().0 as u32;
            return Err(match err {
                1326 => "Logon failure: unknown user name or bad password".to_string(),
                1331 => "Account is disabled".to_string(),
                1909 => "Account is locked out".to_string(),
                1327 => "Account restriction prevents logon".to_string(),
                _ => format!("Logon failed (error {})", err),
            });
        }
        let _ = CloseHandle(token);

        // Step 2: Resolve SID via LookupAccountNameW
        let full_name = if domain.is_empty() {
            user.to_string()
        } else {
            format!("{}\\{}", domain, user)
        };
        let full_name_wide: Vec<u16> = full_name.encode_utf16().chain(std::iter::once(0)).collect();

        let mut sid_size: u32 = 0;
        let mut domain_size: u32 = 0;
        let mut sid_name_use = SID_NAME_USE::default();

        // First call to get buffer sizes (will fail with ERROR_INSUFFICIENT_BUFFER)
        let _ = LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(full_name_wide.as_ptr()),
            PSID(std::ptr::null_mut()),
            &mut sid_size,
            PWSTR::null(),
            &mut domain_size,
            &mut sid_name_use,
        );

        if sid_size == 0 {
            return Err("Failed to resolve SID: buffer size query failed".to_string());
        }

        // Allocate buffers
        let mut sid_buf: Vec<u8> = vec![0; sid_size as usize];
        let mut domain_buf: Vec<u16> = vec![0; domain_size as usize];

        let sid_ptr = sid_buf.as_mut_ptr() as *mut std::ffi::c_void;

        let result = LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(full_name_wide.as_ptr()),
            PSID(sid_ptr),
            &mut sid_size,
            PWSTR(domain_buf.as_mut_ptr()),
            &mut domain_size,
            &mut sid_name_use,
        );

        if result.is_err() {
            return Err("Failed to resolve SID from account name".to_string());
        }

        // Step 3: Convert SID to string
        let mut sid_string_ptr: PWSTR = PWSTR::null();
        let result = ConvertSidToStringSidW(PSID(sid_ptr), &mut sid_string_ptr);

        if result.is_err() {
            return Err("Failed to convert SID to string".to_string());
        }

        // Read the SID string
        let sid_string = wide_ptr_to_string(sid_string_ptr.0);

        // Free the SID string buffer
        if !sid_string_ptr.0.is_null() {
            LocalFree(HLOCAL(sid_string_ptr.0 as *mut std::ffi::c_void));
        }

        // Build display name from resolved domain
        let resolved_domain = if domain_size > 1 {
            String::from_utf16_lossy(&domain_buf[..(domain_size - 1) as usize])
        } else {
            domain.clone()
        };

        let display_name = if resolved_domain.is_empty() {
            user.to_string()
        } else {
            format!("{}\\{}", resolved_domain, user)
        };

        Ok((sid_string, display_name))
    }
}

/// Parse "DOMAIN\user" or "user@domain.com" into (domain, user)
fn parse_username(input: &str) -> (String, String) {
    if let Some(pos) = input.find('\\') {
        let domain = input[..pos].to_string();
        let user = input[pos + 1..].to_string();
        (domain, user)
    } else if let Some(pos) = input.find('@') {
        let user = input[..pos].to_string();
        let domain = input[pos + 1..].to_string();
        (domain, user)
    } else {
        (String::new(), input.to_string())
    }
}

/// Convert a null-terminated wide string pointer to a Rust String
unsafe fn wide_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}
