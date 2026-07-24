//! # Credential Provider - Dual Authentication
//!
//! Windows Credential Provider implementation for dual-account authentication.
//! This DLL is loaded by LogonUI in the secure desktop environment.

#![cfg_attr(test, allow(dead_code))]

pub mod com_types;
pub mod credential_provider;
pub mod ui_controls;
pub mod dual_auth_credential;
pub mod pipe_client;

use std::ffi::c_void;
use windows::Win32::Foundation::BOOL;

// Re-export key types
pub use dual_auth_credential::CLSID_DUAL_AUTH_PROVIDER;
pub use dual_auth_credential::{DualAuthCredential, DualAuthProvider};

/// DLL Entry Point
#[no_mangle]
pub extern "system" fn DllMain(
    _hmodule: *mut c_void,
    dwreason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    match dwreason {
        1 /* DLL_PROCESS_ATTACH */ => {
            log::info!("DualAuthCP DLL loaded");
        }
        0 /* DLL_PROCESS_DETACH */ => {
            log::info!("DualAuthCP DLL unloaded");
        }
        _ => {}
    }
    BOOL::from(true)
}

/// COM Class Factory export - DllGetClassObject
/// Called by COM runtime to get the class factory for our CP
#[no_mangle]
pub extern "system" fn DllGetClassObject(
    _rclsid: *const c_void,
    _riid: *const c_void,
    _ppv: *mut *mut c_void,
) -> i32 {
    // TODO: Implement full COM class factory
    // For now return E_NOTIMPL
    log::debug!("DllGetClassObject called");
    -2147467263i32 // E_NOTIMPL
}

/// COM registration export - DllCanUnloadNow
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> i32 {
    0 // S_OK - can unload
}

/// COM self-registration export
#[no_mangle]
pub extern "system" fn DllRegisterServer() -> i32 {
    log::info!("DllRegisterServer called");
    // Registration is handled by install.ps1 script
    0 // S_OK
}

/// COM self-unregistration export
#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> i32 {
    log::info!("DllUnregisterServer called");
    0 // S_OK
}
