//! Named Pipe Client for Credential Provider
//!
//! Sends authentication requests from the CP (running in LogonUI/Secure Desktop)
//! to the WinSLA Windows Service for AD credential verification.

use std::io::{Read, Write};
use std::time::Duration;

use crate::com_types::{AuthRequest, AuthResponse};

const PIPE_PATH: &str = r"\\.\pipe\winsla-auth-pipe";

/// Send an authentication request to the WinSLA service via named pipe
///
/// Protocol:
/// 1. Connect to pipe
/// 2. Send: [4 bytes length LE][JSON payload]
/// 3. Receive: [4 bytes length LE][JSON payload]
/// 4. Disconnect
pub fn send_auth_request(request: &AuthRequest) -> Result<AuthResponse, PipeClientError> {
    // Serialize request
    let request_bytes = serde_json::to_vec(request)
        .map_err(|e| PipeClientError::SerializationError(e.to_string()))?;

    // Connect to named pipe with retry
    let mut pipe = connect_with_retry()?;

    // Send length prefix + payload
    let len_prefix = (request_bytes.len() as u32).to_le_bytes();
    pipe.write_all(&len_prefix)
        .map_err(|e| PipeClientError::IoError(e.to_string()))?;
    pipe.write_all(&request_bytes)
        .map_err(|e| PipeClientError::IoError(e.to_string()))?;
    pipe.flush()
        .map_err(|e| PipeClientError::IoError(e.to_string()))?;

    // Read response length prefix
    let mut resp_len_bytes = [0u8; 4];
    pipe.read_exact(&mut resp_len_bytes)
        .map_err(|e| PipeClientError::IoError(e.to_string()))?;
    let resp_len = u32::from_le_bytes(resp_len_bytes) as usize;

    if resp_len > 1_000_000 {
        return Err(PipeClientError::InvalidResponse("Response too large".to_string()));
    }

    // Read response body
    let mut resp_buf = vec![0u8; resp_len];
    pipe.read_exact(&mut resp_buf)
        .map_err(|e| PipeClientError::IoError(e.to_string()))?;

    // Deserialize response
    let response: AuthResponse = serde_json::from_slice(&resp_buf)
        .map_err(|e| PipeClientError::SerializationError(e.to_string()))?;

    Ok(response)
}

/// Connect to the named pipe with retry logic
fn connect_with_retry() -> Result<std::fs::File, PipeClientError> {
    use std::os::windows::io::FromRawHandle;

    let max_retries = 3;
    let retry_delay = Duration::from_millis(500);

    for attempt in 0..max_retries {
        match open_pipe_handle() {
            Ok(handle) => {
                // Safety: We own the handle and it's a valid pipe handle
                let file = unsafe { std::fs::File::from_raw_handle(handle) };
                return Ok(file);
            }
            Err(e) => {
                if attempt < max_retries - 1 {
                    log::debug!("Pipe connect attempt {} failed: {}, retrying...", attempt + 1, e);
                    std::thread::sleep(retry_delay);
                } else {
                    return Err(PipeClientError::ConnectionError(format!(
                        "Failed to connect after {} attempts: {}",
                        max_retries, e
                    )));
                }
            }
        }
    }

    unreachable!()
}

/// Open a handle to the named pipe using Windows API
fn open_pipe_handle() -> Result<std::os::windows::io::RawHandle, String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = OsStr::new(PIPE_PATH)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    unsafe {
        let handle = windows::Win32::Storage::FileSystem::CreateFileW(
            windows::core::PCWSTR::from_raw(wide_path.as_ptr()),
            0x80000000 | 0x40000000, // GENERIC_READ | GENERIC_WRITE
            windows::Win32::Storage::FileSystem::FILE_SHARE_READ
                | windows::Win32::Storage::FileSystem::FILE_SHARE_WRITE,
            None,
            windows::Win32::Storage::FileSystem::OPEN_EXISTING,
            windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        );

        match handle {
            Ok(h) => Ok(h.0 as std::os::windows::io::RawHandle),
            Err(e) => Err(format!("CreateFileW failed: {}", e)),
        }
    }
}

/// Pipe client error types
#[derive(Debug, thiserror::Error)]
pub enum PipeClientError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Timeout")]
    Timeout,
}
