//! Named Pipe Server for WinSLA Authentication Service
//!
//! Uses tokio's built-in Windows named pipe support for async I/O.

use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::windows::named_pipe::ServerOptions;

use crate::auth;
use crate::audit::AuditDb;
use crate::com_types::{AuthRequest, AuthResponse};
use crate::ServiceState;

const PIPE_PATH: &str = r"\\.\pipe\winsla-auth-pipe";

/// Main pipe server loop (blocking - spawns its own tokio runtime)
pub fn run_pipe_server(state: Arc<Mutex<ServiceState>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::info!("Starting pipe server on {}", PIPE_PATH);

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        run_async_pipe_server(state).await
    })
}

/// Async pipe server implementation
async fn run_async_pipe_server(state: Arc<Mutex<ServiceState>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        // Create a new pipe server instance
        let server = ServerOptions::new()
            .first_pipe_instance(false)
            .create(PIPE_PATH)?;

        log::info!("Waiting for client connection...");

        // Wait for a client to connect
        server.connect().await?;
        log::info!("Client connected via pipe");

        {
            let mut state_guard = state.lock().unwrap();
            state_guard.connections_accepted += 1;
            state_guard.update_activity();
        }

        let state_clone = Arc::clone(&state);

        // Handle the client in a spawned task
        tokio::spawn(async move {
            if let Err(e) = handle_client(server, state_clone).await {
                log::error!("Client handler error: {}", e);
            }
        });
    }
}

/// Handle a single client connection
async fn handle_client(
    mut pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    state: Arc<Mutex<ServiceState>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Read message length (u32 little-endian)
    let mut len_bytes = [0u8; 4];
    pipe.read_exact(&mut len_bytes).await?;
    let message_len = u32::from_le_bytes(len_bytes) as usize;

    if message_len > 10_000_000 {
        return Err("Invalid message size".into());
    }

    // Read message body
    let mut buf = vec![0u8; message_len];
    pipe.read_exact(&mut buf).await?;

    // Deserialize request
    let request: AuthRequest = serde_json::from_slice(&buf)?;

    log::info!(
        "Processing auth request: {} <-> {}",
        request.user_a_username,
        request.user_b_username
    );

    // Process authentication
    let response = process_auth_request(request, &state).await;

    // Serialize and send response with length prefix
    let response_bytes = serde_json::to_vec(&response)?;
    let len_prefix = (response_bytes.len() as u32).to_le_bytes();

    pipe.write_all(&len_prefix).await?;
    pipe.write_all(&response_bytes).await?;
    pipe.flush().await?;

    log::info!("Response sent successfully");

    Ok(())
}

/// Process authentication request using dual verification
async fn process_auth_request(request: AuthRequest, state: &Arc<Mutex<ServiceState>>) -> AuthResponse {
    log::info!(
        "Processing auth request: {} <-> {}",
        request.user_a_username,
        request.user_b_username
    );
    
    // Step 1: Check pairing rules first (strict order validation)
    match crate::auth::dual_validator::check_pairing_rule(
        &request.user_a_username,  // Treat user_a as account
        &request.user_b_username,  // Treat user_b as approver
    ).await {
        Ok(()) => {
            log::info!("Pairing rule validation passed");
        }
        Err(auth::AuthError::InvalidCredentials(msg)) => {
            log::warn!("Pairing rule rejected login: {}", msg);
            
            // Record audit log before returning
            if let Ok(audit_db) = AuditDb::open() {
                let _ = audit_db.record_auth(
                    &request.user_a_username,
                    &request.user_b_username,
                    "pairing_violation",
                    Some(&msg),
                    None,
                );
            }
            
            return AuthResponse::FailUserA(msg); // Show Chinese error message
        }
        Err(e) => {
            log::error!("Pairing rule check error: {}", e);
            return AuthResponse::NetworkUnavailable;
        }
    }
    
    // Step 2: Verify both accounts' credentials
    let result = auth::dual_validator::validate_dual_accounts(
        &request.user_a_username,
        &request.user_a_password_hash,
        &request.user_b_username,
        &request.user_b_password_hash,
    )
    .await;

    let response = match result {
        Ok(()) => {
            log::info!("Authentication successful for {} + {}", request.user_a_username, request.user_b_username);
            AuthResponse::Success
        }
        Err(auth::AuthError::InvalidCredentials(msg)) => {
            log::warn!("Auth failed (invalid credentials): {}", msg);
            AuthResponse::FailUserA(msg)
        }
        Err(auth::AuthError::BothFailed(msg_a, msg_b)) => {
            log::warn!("Both users failed: A={}, B={}", msg_a, msg_b);
            AuthResponse::BothFailed(msg_a, msg_b)
        }
        Err(auth::AuthError::NetworkUnavailable) => {
            log::error!("Network unavailable during auth");
            AuthResponse::NetworkUnavailable
        }
        Err(auth::AuthError::Timeout) => {
            log::error!("Auth timeout");
            AuthResponse::Timeout
        }
        Err(e) => {
            log::error!("Auth error: {}", e);
            AuthResponse::FailUserA(e.to_string())
        }
    };

    // Record result in state
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.update_activity();
        if response.is_success() {
            state_guard.record_success();
        } else {
            state_guard.record_failure();
        }
    }

    // Write audit record to shared SQLite database
    let result_str: &str;
    let error_msg: Option<String>;
    match &response {
        AuthResponse::Success => { result_str = "success"; error_msg = None; }
        AuthResponse::FailUserA(msg) => { result_str = "fail_user_a"; error_msg = Some(msg.clone()); }
        AuthResponse::FailUserB(msg) => { result_str = "fail_user_b"; error_msg = Some(msg.clone()); }
        AuthResponse::BothFailed(msg_a, msg_b) => { result_str = "fail_both"; error_msg = Some(format!("{}; {}", msg_a, msg_b)); }
        AuthResponse::NetworkUnavailable => { result_str = "network_unavailable"; error_msg = Some("Network unavailable".to_string()); }
        AuthResponse::Timeout => { result_str = "timeout"; error_msg = Some("Authentication timeout".to_string()); }
    }

    if let Ok(audit_db) = AuditDb::open() {
        // 使用用户名作为占位符（因为还没有 SID）
        // TODO: 在认证成功后使用真实的 SID
        if let Err(e) = audit_db.record_auth(
            &request.user_a_username,   // placeholder: account_sid
            &request.user_b_username,   // placeholder: approver_sid
            result_str,
            error_msg.as_deref(),
            None, // client_hostname not available in current protocol
        ) {
            log::warn!("Failed to write audit record to DB: {}", e);
        }
    } else {
        log::warn!("Failed to open audit database");
    }

    response
}
