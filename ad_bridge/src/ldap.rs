//! LDAP authentication implementation for AD domain integration
//!
//! Uses ldap3 crate's synchronous API (LdapConn) for reliable AD binding.

use ldap3::LdapConn;

/// LDAP authentication error types
#[derive(Debug, thiserror::Error)]
pub enum LdapAuthError {
    #[error("LDAP connection failed: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

/// Configuration for LDAP connection
#[derive(Clone)]
pub struct LdapConfig {
    pub dc_addresses: Vec<String>,
    pub base_dn: String,
}

impl LdapConfig {
    pub fn new(dc_addresses: Vec<String>, base_dn: String) -> Self {
        Self { dc_addresses, base_dn }
    }
}

/// LDAP authentication client that connects to Active Directory
pub struct LdapAuthClient {
    config: LdapConfig,
}

impl LdapAuthClient {
    /// Create new LDAP auth client
    pub fn new(config: LdapConfig) -> Self {
        Self { config }
    }

    /// Verify credentials using LDAP Simple Bind
    ///
    /// Tries each configured DC until one succeeds or all fail.
    pub fn verify_credentials(&self, username: &str, password: &str) -> Result<(), LdapAuthError> {
        if self.config.dc_addresses.is_empty() {
            return Err(LdapAuthError::Config("No domain controllers configured".to_string()));
        }

        if username.is_empty() || password.is_empty() {
            return Err(LdapAuthError::InvalidCredentials(
                "Username and password must not be empty".to_string(),
            ));
        }

        log::debug!("Verifying credentials via LDAP for user: {}", username);

        // Try each DC until one succeeds
        let mut last_error = None;
        for dc in &self.config.dc_addresses {
            match self.verify_single_dc(dc, username, password) {
                Ok(()) => {
                    log::info!("LDAP authentication successful for {} against {}", username, dc);
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("LDAP auth failed for {} on {}: {}", username, dc, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            LdapAuthError::InvalidCredentials(format!(
                "Failed to authenticate {} against any domain controller",
                username
            ))
        }))
    }

    /// Verify credentials against a single DC using LDAP Simple Bind
    fn verify_single_dc(
        &self,
        dc_address: &str,
        username: &str,
        password: &str,
    ) -> Result<(), LdapAuthError> {
        // Construct LDAP URL (use ldap:// for port 389, ldaps:// for 636)
        let ldap_url = format!("ldap://{}", dc_address);
        log::trace!("Connecting to LDAP: {}", ldap_url);

        // Establish LDAP connection
        let mut ldap = LdapConn::new(&ldap_url)
            .map_err(|e| LdapAuthError::Connection(format!("Cannot connect to {}: {}", dc_address, e)))?;

        // Build user's Distinguished Name
        let user_dn = self.build_user_dn(username);
        log::trace!("Attempting bind with DN: {}", user_dn);

        // Perform simple bind authentication
        let bind_result = ldap.simple_bind(&user_dn, password);

        // Ensure connection is properly closed
        let _ = ldap.unbind();

        match bind_result {
            Ok(bind_res) => {
                // Check the result code
                if bind_res.rc == 0 {
                    // LDAP_SUCCESS
                    log::trace!("LDAP simple bind successful for {}", username);
                    Ok(())
                } else if bind_res.rc == 49 {
                    // LDAP_INVALID_CREDENTIALS
                    Err(LdapAuthError::InvalidCredentials(format!(
                        "Invalid credentials for user: {}",
                        username
                    )))
                } else {
                    Err(LdapAuthError::Authentication(format!(
                        "LDAP bind failed with code {}: {:?}",
                        bind_res.rc, bind_res.text
                    )))
                }
            }
            Err(e) => Err(LdapAuthError::Connection(format!(
                "LDAP bind operation failed: {}",
                e
            ))),
        }
    }

    /// Build distinguished name for a user account
    ///
    /// Supports formats:
    /// - "DOMAIN\username" -> CN=username,{base_dn}
    /// - "user@domain.com" -> CN=user,{base_dn}
    /// - "username" -> CN=username,{base_dn}
    fn build_user_dn(&self, username: &str) -> String {
        let user_part = if username.contains('\\') {
            // Format: DOMAIN\username
            username.split('\\').last().unwrap_or(username)
        } else if username.contains('@') {
            // Format: user@domain.com - extract just the username part
            username.split('@').next().unwrap_or(username)
        } else {
            username
        };

        // Build CN=username,{base_dn}
        format!("CN={},{}", user_part, self.config.base_dn)
    }

    /// Get user's distinguished name (public helper)
    pub fn get_user_dn(&self, username: &str) -> String {
        self.build_user_dn(username)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_user_dn_with_domain_backslash() {
        let config = LdapConfig::new(vec![], "DC=example,DC=com".to_string());
        let client = LdapAuthClient::new(config);

        let dn = client.get_user_dn("DOMAIN\\admin");
        assert_eq!(dn, "CN=admin,DC=example,DC=com");
    }

    #[test]
    fn test_build_user_dn_with_email_format() {
        let config = LdapConfig::new(vec![], "DC=example,DC=com".to_string());
        let client = LdapAuthClient::new(config);

        let dn = client.get_user_dn("admin@example.com");
        assert_eq!(dn, "CN=admin,DC=example,DC=com");
    }

    #[test]
    fn test_build_user_dn_simple_username() {
        let config = LdapConfig::new(vec![], "DC=test,DC=local".to_string());
        let client = LdapAuthClient::new(config);

        let dn = client.get_user_dn("johndoe");
        assert_eq!(dn, "CN=johndoe,DC=test,DC=local");
    }

    #[test]
    fn test_empty_credentials_rejected() {
        let config = LdapConfig::new(
            vec!["dc1.example.com".to_string()],
            "DC=example,DC=com".to_string(),
        );
        let client = LdapAuthClient::new(config);

        let result = client.verify_credentials("", "password");
        assert!(result.is_err());

        let result = client.verify_credentials("user", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_no_dc_configured() {
        let config = LdapConfig::new(vec![], "DC=example,DC=com".to_string());
        let client = LdapAuthClient::new(config);

        let result = client.verify_credentials("user", "pass");
        assert!(matches!(result, Err(LdapAuthError::Config(_))));
    }
}
