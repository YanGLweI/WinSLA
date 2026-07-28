//! Dual Authentication Credential Provider - Core Implementation
//!
//! This module implements the core logic for dual-account authentication.
//! The actual COM vtable integration requires unsafe FFI and will be added
//! once the framework compiles cleanly.

use windows::core::GUID;

/// Credential Provider CLSID matching registry registration
/// {E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}
pub const CLSID_DUAL_AUTH_PROVIDER: GUID = GUID {
    data1: 0xE4D9F6E7,
    data2: 0x8A2B,
    data3: 0x4C3D,
    data4: [0x9E, 0x5F, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F],
};

/// Credential Provider Filter CLSID (separate registration)
/// {E4D9F6E8-8A2B-4C3D-9E5F-1A2B3C4D5E6F}
pub const CLSID_WINSLA_FILTER: GUID = GUID {
    data1: 0xE4D9F6E8,
    data2: 0x8A2B,
    data3: 0x4C3D,
    data4: [0x9E, 0x5F, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F],
};

/// Number of credentials this provider supplies (always 1 for dual auth)
pub const CREDENTIAL_COUNT: u32 = 1;

/// Field indices for the dual-auth credential UI
pub const FIELD_USER_A_NAME: usize = 0;
pub const FIELD_USER_A_PASS: usize = 1;
pub const FIELD_USER_B_NAME: usize = 2;
pub const FIELD_USER_B_PASS: usize = 3;
pub const FIELD_SUBMIT_BUTTON: usize = 4;
pub const FIELD_STATUS_TEXT: usize = 5;
pub const FIELD_COUNT: usize = 6;

// ============================================================================
// Core Credential Provider Structures
// ============================================================================

/// Credential state tracking
#[derive(Debug, Clone, PartialEq)]
pub enum CredentialState {
    /// No input yet
    Empty,
    /// Only User A has entered username
    UserAOnly,
    /// Both usernames entered, waiting for passwords
    BothUsernames,
    /// All fields filled, ready to submit
    ReadyToSubmit,
    /// Currently verifying with service
    Verifying,
    /// Verification succeeded
    Verified,
    /// Verification failed
    Failed(String),
}

/// Main dual-authentication credential structure
pub struct DualAuthCredential {
    /// Account information - User A
    pub user_a_username: String,
    pub user_a_password: String,

    /// Account information - User B
    pub user_b_username: String,
    pub user_b_password: String,

    /// Current state
    pub state: CredentialState,

    /// Status message displayed to user
    pub status_message: String,
}

impl Default for DualAuthCredential {
    fn default() -> Self {
        Self::new()
    }
}

impl DualAuthCredential {
    /// Create new dual-auth credential with default configuration
    pub fn new() -> Self {
        Self {
            user_a_username: String::new(),
            user_a_password: String::new(),
            user_b_username: String::new(),
            user_b_password: String::new(),
            state: CredentialState::Empty,
            status_message: "请输入主账号和审批人的凭据以完成双因素认证".to_string(),
        }
    }

    /// Set field value by index
    pub fn set_field_value(&mut self, field_index: usize, value: &str) {
        match field_index {
            FIELD_USER_A_NAME => {
                self.user_a_username = value.to_string();
                self.update_state();
            }
            FIELD_USER_A_PASS => {
                self.user_a_password = value.to_string();
                self.update_state();
            }
            FIELD_USER_B_NAME => {
                self.user_b_username = value.to_string();
                self.update_state();
            }
            FIELD_USER_B_PASS => {
                self.user_b_password = value.to_string();
                self.update_state();
            }
            _ => {}
        }
    }

    /// Get current field value as string
    pub fn get_field_value(&self, field_index: usize) -> Option<&str> {
        match field_index {
            FIELD_USER_A_NAME => Some(&self.user_a_username),
            FIELD_USER_A_PASS => Some(&self.user_a_password),
            FIELD_USER_B_NAME => Some(&self.user_b_username),
            FIELD_USER_B_PASS => Some(&self.user_b_password),
            FIELD_STATUS_TEXT => Some(&self.status_message),
            _ => None,
        }
    }

    /// Update internal state based on field values
    fn update_state(&mut self) {
        if self.user_a_username.is_empty() && self.user_b_username.is_empty() {
            self.state = CredentialState::Empty;
        } else if !self.user_a_username.is_empty() && self.user_b_username.is_empty() {
            self.state = CredentialState::UserAOnly;
        } else if !self.user_a_username.is_empty() && !self.user_b_username.is_empty() {
            if !self.user_a_password.is_empty() && !self.user_b_password.is_empty() {
                self.state = CredentialState::ReadyToSubmit;
            } else {
                self.state = CredentialState::BothUsernames;
            }
        }
    }

    /// Validate all required fields are filled
    pub fn validate_fields(&self) -> Result<(), String> {
        if self.user_a_username.is_empty() {
            return Err("User A username is required".to_string());
        }
        if self.user_a_password.is_empty() {
            return Err("User A password is required".to_string());
        }
        if self.user_b_username.is_empty() {
            return Err("User B username is required".to_string());
        }
        if self.user_b_password.is_empty() {
            return Err("User B password is required".to_string());
        }
        Ok(())
    }

    /// Reset all fields to initial state
    pub fn reset(&mut self) {
        self.user_a_username.clear();
        self.user_a_password.clear();
        self.user_b_username.clear();
        self.user_b_password.clear();
        self.state = CredentialState::Empty;
        self.status_message = "Enter both user credentials".to_string();
    }

    /// Mark as verified
    pub fn mark_verified(&mut self) {
        self.state = CredentialState::Verified;
        self.status_message = "Authentication successful".to_string();
    }

    /// Mark as failed with reason
    pub fn mark_failed(&mut self, reason: &str) {
        self.state = CredentialState::Failed(reason.to_string());
        // Display Chinese error message
        self.status_message = format!("身份验证失败：{}", reason);
    }
}

// ============================================================================
// Credential Provider (manages credentials)
// ============================================================================

/// Credential Provider interface implementation
pub struct DualAuthProvider {
    pub credentials: Vec<DualAuthCredential>,
    pub provider_name: String,
}

impl DualAuthProvider {
    /// Create new credential provider instance
    pub fn new() -> Self {
        Self {
            credentials: vec![DualAuthCredential::new()],
            provider_name: "WinSLA Dual-Account Authentication".to_string(),
        }
    }

    /// Get credential count
    pub fn get_credential_count(&self) -> u32 {
        self.credentials.len() as u32
    }

    /// Get mutable reference to credential at index
    pub fn get_credential_mut(&mut self, index: u32) -> Option<&mut DualAuthCredential> {
        self.credentials.get_mut(index as usize)
    }

    /// Get provider name (displayed in LogonUI)
    pub fn get_provider_name(&self) -> &str {
        &self.provider_name
    }
}

impl Default for DualAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert UTF-16 wide string pointer to Rust String
pub fn wstr_to_string(wstr: *const u16) -> Option<String> {
    if wstr.is_null() {
        return None;
    }

    let mut len = 0;
    unsafe {
        while *wstr.add(len) != 0 {
            len += 1;
        }
        String::from_utf16(std::slice::from_raw_parts(wstr, len)).ok()
    }
}

/// Convert Rust String to null-terminated UTF-16 Vec
pub fn string_to_wstring(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_creation() {
        let cred = DualAuthCredential::new();
        assert_eq!(cred.state, CredentialState::Empty);
    }

    #[test]
    fn test_field_setting() {
        let mut cred = DualAuthCredential::new();
        cred.set_field_value(FIELD_USER_A_NAME, "user1@domain.com");
        assert_eq!(cred.state, CredentialState::UserAOnly);
        assert_eq!(cred.user_a_username, "user1@domain.com");
    }

    #[test]
    fn test_validation() {
        let mut cred = DualAuthCredential::new();
        assert!(cred.validate_fields().is_err());

        cred.set_field_value(FIELD_USER_A_NAME, "user1");
        cred.set_field_value(FIELD_USER_A_PASS, "pass1");
        cred.set_field_value(FIELD_USER_B_NAME, "user2");
        cred.set_field_value(FIELD_USER_B_PASS, "pass2");
        assert!(cred.validate_fields().is_ok());
        assert_eq!(cred.state, CredentialState::ReadyToSubmit);
    }

    #[test]
    fn test_reset() {
        let mut cred = DualAuthCredential::new();
        cred.set_field_value(FIELD_USER_A_NAME, "user1");
        cred.reset();
        assert_eq!(cred.state, CredentialState::Empty);
        assert!(cred.user_a_username.is_empty());
    }

    #[test]
    fn test_provider() {
        let provider = DualAuthProvider::new();
        assert_eq!(provider.get_credential_count(), 1);
        assert_eq!(provider.get_provider_name(), "WinSLA Dual-Account Authentication");
    }
}
