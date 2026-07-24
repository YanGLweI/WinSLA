//! UI Control handling for Credential Provider
//! This module manages the display of input fields in LogonUI

use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;

/// Represents an editable field in the credential UI
pub enum TextFieldType {
    Username,
    Password,
}

pub struct TextField {
    pub field_type: TextFieldType,
    pub value: String,
    pub is_required: bool,
    pub label: String,
}

impl TextField {
    pub fn new(field_type: TextFieldType, label: &str) -> Self {
        Self {
            field_type,
            value: String::new(),
            is_required: true,
            label: label.to_string(),
        }
    }
    
    pub fn set_value(&mut self, text: &str) {
        self.value = text.to_string();
    }
    
    pub fn is_empty(&self) -> bool {
        self.value.trim().is_empty()
    }
}

/// Dual auth UI consists of two sets of credentials
pub struct DualAuthUi {
    pub user_a_username: TextField,
    pub user_a_password: TextField,
    pub user_b_username: TextField,
    pub user_b_password: TextField,
    pub submit_button_label: String,
    pub status_message: String,
}

impl DualAuthUi {
    pub fn new() -> Self {
        Self {
            user_a_username: TextField::new(TextFieldType::Username, "First User"),
            user_a_password: TextField::new(TextFieldType::Password, "First Password"),
            user_b_username: TextField::new(TextFieldType::Username, "Second User"),
            user_b_password: TextField::new(TextFieldType::Password, "Second Password"),
            submit_button_label: "Submit\0".to_string(),
            status_message: "Enter both users' credentials\0".to_string(),
        }
    }
    
    /// Check if all required fields are filled
    pub fn has_all_credentials(&self) -> bool {
        !self.user_a_username.is_empty() &&
        !self.user_a_password.value.is_empty() && // Password field has different validation
        !self.user_b_username.is_empty() &&
        !self.user_b_password.value.is_empty()
    }
    
    /// Get status message based on current state
    pub fn get_status(&self) -> &str {
        if self.user_a_username.is_empty() && self.user_b_username.is_empty() {
            &self.status_message
        } else if !self.user_a_username.is_empty() && self.user_b_username.is_empty() {
            "Waiting for second user"
        } else {
            "Ready to verify"
        }
    }
    
    /// Reset all fields
    pub fn reset(&mut self) {
        self.user_a_username.set_value("");
        self.user_a_password.set_value("");
        self.user_b_username.set_value("");
        self.user_b_password.set_value("");
        self.status_message = "Enter both users' credentials\0".to_string();
    }
}

/// Convert UTF-16 WStr to Rust String
pub fn wstr_to_string(wstr: *const u16) -> Option<String> {
    if wstr.is_null() {
        return None;
    }
    
    let mut len = 0;
    unsafe {
        while *wstr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(wstr, len);
        OsString::from_wide(slice)
            .into_string()
            .ok()
    }
}

/// Convert Rust String to WStr (owned)
pub fn string_to_wstring(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dual_auth_ui_creation() {
        let ui = DualAuthUi::new();
        assert!(!ui.has_all_credentials());
    }

    #[test]
    fn test_field_validation() {
        let mut ui = DualAuthUi::new();
        ui.user_a_username.set_value("user1");
        ui.user_b_username.set_value("user2");
        
        assert!(!ui.has_all_credentials()); // Missing passwords
        
        ui.user_a_password.set_value("pass1");
        ui.user_b_password.set_value("pass2");
        
        assert!(ui.has_all_credentials());
    }

    #[test]
    fn test_reset() {
        let mut ui = DualAuthUi::new();
        ui.user_a_username.set_value("user1");
        ui.reset();
        assert!(ui.user_a_username.is_empty());
    }
}
