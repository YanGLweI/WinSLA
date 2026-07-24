//! LDAP-based authentication verifier

use crate::auth::AuthError;

/// LDAP Authentication helper
pub struct LdapVerifier {
    dc_address: Option<String>,
    admin_dn: Option<String>,
}

impl Default for LdapVerifier {
    fn default() -> Self {
        Self {
            dc_address: None,
            admin_dn: None,
        }
    }
}

impl LdapVerifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure with domain controller address and admin credentials
    #[allow(dead_code)]
    pub fn with_config(mut self, dc_address: &str, admin_dn: &str) -> Self {
        self.dc_address = Some(dc_address.to_string());
        self.admin_dn = Some(admin_dn.to_string());
        self
    }

    /// Simple LDAP bind authentication
    #[allow(dead_code)]
    pub async fn simple_bind(
        &self,
        username: &str,
        password: &str,
    ) -> Result<(), AuthError> {
        log::debug!("Performing LDAP simple bind for user: {}", username);

        // In production, this would use an LDAP library like ldap3 or rusty-ldap
        // For now, we simulate based on credential format
        
        if !username.contains('@') && !username.contains('\\') {
            return Err(AuthError::InvalidCredentials("Invalid username format".to_string()));
        }

        if password.is_empty() {
            return Err(AuthError::InvalidCredentials("Empty password".to_string()));
        }

        // Simulate successful binding for valid-looking credentials
        log::trace!("LDAP bind would succeed for {}", username);
        Ok(())
    }

    /// Get LDAP distinguished name from username
    pub fn get_user_dn(&self, username: &str, domain: &str) -> String {
        // Common formats: CN=username,OU=Users,DC=domain,DC=com
        // Or simply: username@domain.com
        let domain_parts: Vec<&str> = domain.split('.').collect();
        
        if domain_parts.len() > 1 {
            let dc_components: String = domain_parts
                .iter()
                .map(|part| format!("DC={}", part))
                .collect::<Vec<_>>()
                .join(",");
            
            format!("CN={},{}", username.replace('@', ","), dc_components)
        } else {
            // Single-part domain
            format!("CN={}@{}", username, domain)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_dn() {
        let verifier = LdapVerifier::new();
        let dn = verifier.get_user_dn("admin", "EXAMPLE.COM");
        assert!(dn.contains("CN=admin"));
        assert!(dn.contains("DC=EXAMPLE"));
        assert!(dn.contains("DC=COM"));
    }

    #[tokio::test]
    async fn test_empty_password() {
        let verifier = LdapVerifier::new();
        let result = verifier.simple_bind("admin", "").await;
        assert!(result.is_err());
    }
}
