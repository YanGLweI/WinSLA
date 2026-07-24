//! Dual Authentication Credential Provider - Service Communication
//!
//! Handles the interaction between the CP UI and the Windows Service
//! via Named Pipe for credential verification.

use crate::com_types::{AuthRequest, AuthResponse};
use crate::dual_auth_credential::{DualAuthCredential, CredentialState};

/// Submit credentials to the Windows Service for verification
///
/// This function collects both users' credentials and sends them
/// to the service via named pipe for AD verification.
pub fn submit_credentials(credential: &mut DualAuthCredential) -> Result<AuthResponse, String> {
    // Validate all fields are filled
    credential.validate_fields().map_err(|e| e)?;

    // Update state to verifying
    credential.state = CredentialState::Verifying;
    credential.status_message = "Verifying credentials...".to_string();

    // Build auth request
    let request = AuthRequest::new(
        credential.user_a_username.clone(),
        &credential.user_a_password,
        credential.user_b_username.clone(),
        &credential.user_b_password,
    );

    // Send to service via named pipe
    let response = crate::pipe_client::send_auth_request(&request)
        .map_err(|e| format!("Service communication error: {}", e))?;

    // Update credential state based on response
    match &response {
        AuthResponse::Success => {
            credential.mark_verified();
        }
        AuthResponse::FailUserA(msg) => {
            credential.mark_failed(&format!("User A: {}", msg));
        }
        AuthResponse::FailUserB(msg) => {
            credential.mark_failed(&format!("User B: {}", msg));
        }
        AuthResponse::BothFailed(msg_a, msg_b) => {
            credential.mark_failed(&format!("User A: {} | User B: {}", msg_a, msg_b));
        }
        AuthResponse::Timeout => {
            credential.mark_failed("Verification timed out");
        }
        AuthResponse::NetworkUnavailable => {
            credential.mark_failed("Domain controller unreachable");
        }
    }

    // Clear passwords from memory after use
    credential.user_a_password.clear();
    credential.user_b_password.clear();

    Ok(response)
}

/// Check if the credential is ready for submission
pub fn is_ready_to_submit(credential: &DualAuthCredential) -> bool {
    credential.state == CredentialState::ReadyToSubmit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_auth_credential::*;

    #[test]
    fn test_is_ready_to_submit() {
        let mut cred = DualAuthCredential::new();
        assert!(!is_ready_to_submit(&cred));

        cred.set_field_value(FIELD_USER_A_NAME, "user1");
        cred.set_field_value(FIELD_USER_A_PASS, "pass1");
        cred.set_field_value(FIELD_USER_B_NAME, "user2");
        cred.set_field_value(FIELD_USER_B_PASS, "pass2");
        assert!(is_ready_to_submit(&cred));
    }
}
