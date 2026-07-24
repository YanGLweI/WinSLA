//! # AD Bridge Library
//!
//! Provides Active Directory authentication operations via LDAP.
//! This crate can be used by both the Service and Management App.

pub mod ldap;

pub use ldap::{LdapAuthClient, LdapAuthError, LdapConfig};

use std::sync::Arc;

/// Configuration for domain connectivity
#[derive(Clone)]
pub struct DomainConfig {
    pub dc_addresses: Vec<String>,
    pub base_dn: String,
}

impl Default for DomainConfig {
    fn default() -> Self {
        Self {
            dc_addresses: vec![],
            base_dn: String::new(),
        }
    }
}

/// High-level domain authentication client
pub struct DomainAuthClient {
    config: Arc<DomainConfig>,
    ldap_client: LdapAuthClient,
}

impl DomainAuthClient {
    pub fn new(config: DomainConfig) -> Self {
        let ldap_config = LdapConfig::new(
            config.dc_addresses.clone(),
            config.base_dn.clone(),
        );
        let ldap_client = LdapAuthClient::new(ldap_config);

        Self {
            config: Arc::new(config),
            ldap_client,
        }
    }

    /// Verify credentials using LDAP Simple Bind
    pub fn verify_credentials(&self, username: &str, password: &str) -> Result<bool, LdapAuthError> {
        log::debug!("Verifying credentials for user: {}", username);

        match self.ldap_client.verify_credentials(username, password) {
            Ok(()) => {
                log::info!("Authentication successful for {}", username);
                Ok(true)
            }
            Err(LdapAuthError::InvalidCredentials(_)) => {
                log::info!("Invalid credentials for {}", username);
                Ok(false)
            }
            Err(e) => {
                log::error!("Authentication error for {}: {}", username, e);
                Err(e)
            }
        }
    }

    /// Get the base DN
    pub fn get_base_dn(&self) -> &str {
        &self.config.base_dn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_config_creation() {
        let config = DomainConfig {
            dc_addresses: vec!["dc1.example.com".to_string()],
            base_dn: "DC=example,DC=com".to_string(),
        };
        assert_eq!(config.dc_addresses.len(), 1);
    }

    #[test]
    fn test_ldap_config_creation() {
        let config = LdapConfig::new(
            vec!["dc1.example.com".to_string()],
            "DC=example,DC=com".to_string(),
        );
        assert_eq!(config.dc_addresses.len(), 1);
        assert_eq!(config.base_dn, "DC=example,DC=com");
    }
}
