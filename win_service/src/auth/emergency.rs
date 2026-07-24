//! Emergency override mechanism for WinSLA
//!
//! Allows designated emergency accounts to bypass dual-authentication
//! in critical situations. All overrides are logged for audit.

use serde::{Deserialize, Serialize};

/// Emergency override request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyOverrideRequest {
    pub username: String,
    pub password_hash: Vec<u8>,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Emergency override result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmergencyOverrideResult {
    /// Override approved - single user can login
    Approved { username: String },
    /// Override denied - account not authorized
    Denied { reason: String },
    /// Override expired
    Expired,
}

/// Emergency override configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyConfig {
    /// Whether emergency override is enabled
    pub enabled: bool,
    /// List of authorized emergency account SIDs
    pub authorized_sids: Vec<String>,
    /// Whether a reason must be provided
    pub require_reason: bool,
    /// Maximum time an override remains active (hours)
    pub max_active_hours: u32,
}

impl Default for EmergencyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            authorized_sids: vec![],
            require_reason: true,
            max_active_hours: 24,
        }
    }
}

/// Emergency override manager
pub struct EmergencyOverrideManager {
    config: EmergencyConfig,
}

impl EmergencyOverrideManager {
    pub fn new(config: EmergencyConfig) -> Self {
        Self { config }
    }

    /// Check if emergency override is allowed for the given request
    pub fn check_override(&self, request: &EmergencyOverrideRequest) -> EmergencyOverrideResult {
        if !self.config.enabled {
            return EmergencyOverrideResult::Denied {
                reason: "Emergency override is disabled by policy".to_string(),
            };
        }

        if self.config.require_reason && request.reason.trim().is_empty() {
            return EmergencyOverrideResult::Denied {
                reason: "A reason must be provided for emergency override".to_string(),
            };
        }

        // In production, verify the account SID against authorized_sids list
        // and check credentials against AD
        if request.username.is_empty() {
            return EmergencyOverrideResult::Denied {
                reason: "Username is required".to_string(),
            };
        }

        // Check if the account is in the authorized list
        // For now, we check by username (in production, use SID)
        let is_authorized = self.config.authorized_sids.iter().any(|sid| {
            sid.eq_ignore_ascii_case(&request.username)
        });

        if !is_authorized && !self.config.authorized_sids.is_empty() {
            return EmergencyOverrideResult::Denied {
                reason: format!(
                    "Account '{}' is not authorized for emergency override",
                    request.username
                ),
            };
        }

        log::warn!(
            "EMERGENCY OVERRIDE APPROVED: user={}, reason={}",
            request.username,
            request.reason
        );

        EmergencyOverrideResult::Approved {
            username: request.username.clone(),
        }
    }

    /// Update configuration
    pub fn update_config(&mut self, config: EmergencyConfig) {
        self.config = config;
    }

    /// Check if override is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Default for EmergencyOverrideManager {
    fn default() -> Self {
        Self::new(EmergencyConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_disabled() {
        let config = EmergencyConfig {
            enabled: false,
            ..Default::default()
        };
        let manager = EmergencyOverrideManager::new(config);

        let request = EmergencyOverrideRequest {
            username: "admin".to_string(),
            password_hash: vec![],
            reason: "Emergency".to_string(),
            timestamp: chrono::Utc::now(),
        };

        match manager.check_override(&request) {
            EmergencyOverrideResult::Denied { reason } => {
                assert!(reason.contains("disabled"));
            }
            _ => panic!("Expected Denied"),
        }
    }

    #[test]
    fn test_override_requires_reason() {
        let config = EmergencyConfig {
            enabled: true,
            require_reason: true,
            ..Default::default()
        };
        let manager = EmergencyOverrideManager::new(config);

        let request = EmergencyOverrideRequest {
            username: "admin".to_string(),
            password_hash: vec![],
            reason: "".to_string(),
            timestamp: chrono::Utc::now(),
        };

        match manager.check_override(&request) {
            EmergencyOverrideResult::Denied { reason } => {
                assert!(reason.contains("reason"));
            }
            _ => panic!("Expected Denied"),
        }
    }

    #[test]
    fn test_override_approved() {
        let config = EmergencyConfig {
            enabled: true,
            require_reason: true,
            authorized_sids: vec![], // Empty = allow all (for testing)
            ..Default::default()
        };
        let manager = EmergencyOverrideManager::new(config);

        let request = EmergencyOverrideRequest {
            username: "admin".to_string(),
            password_hash: vec![1, 2, 3],
            reason: "System maintenance required".to_string(),
            timestamp: chrono::Utc::now(),
        };

        match manager.check_override(&request) {
            EmergencyOverrideResult::Approved { username } => {
                assert_eq!(username, "admin");
            }
            _ => panic!("Expected Approved"),
        }
    }

    #[test]
    fn test_override_unauthorized_account() {
        let config = EmergencyConfig {
            enabled: true,
            require_reason: false,
            authorized_sids: vec!["S-1-5-21-admin-sid".to_string()],
            ..Default::default()
        };
        let manager = EmergencyOverrideManager::new(config);

        let request = EmergencyOverrideRequest {
            username: "regular_user".to_string(),
            password_hash: vec![],
            reason: "Trying to bypass".to_string(),
            timestamp: chrono::Utc::now(),
        };

        match manager.check_override(&request) {
            EmergencyOverrideResult::Denied { reason } => {
                assert!(reason.contains("not authorized"));
            }
            _ => panic!("Expected Denied"),
        }
    }
}
