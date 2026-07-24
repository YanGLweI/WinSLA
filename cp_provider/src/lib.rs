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
pub mod class_factory;
pub mod provider_com;
pub mod credential_com;

use std::ffi::c_void;
use windows::Win32::Foundation::BOOL;
use windows::core::GUID;

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
            // Disable thread library calls for performance
        }
        0 /* DLL_PROCESS_DETACH */ => {}
        _ => {}
    }
    BOOL::from(true)
}

/// COM Class Factory export - DllGetClassObject
/// Called by COM runtime to get the class factory for our CP
#[no_mangle]
pub extern "system" fn DllGetClassObject(
    rclsid: *const c_void,
    riid: *const c_void,
    ppv: *mut *mut c_void,
) -> i32 {
    unsafe {
        if rclsid.is_null() || riid.is_null() || ppv.is_null() {
            return -2147024809i32; // E_INVALIDARG
        }

        let clsid = &*(rclsid as *const GUID);

        // Check if the requested CLSID matches our provider
        if *clsid != CLSID_DUAL_AUTH_PROVIDER {
            *ppv = std::ptr::null_mut();
            return -2147467262i32; // CLASS_E_CLASSNOTAVAILABLE
        }

        // Create the class factory
        let factory = class_factory::create_class_factory();
        if factory.is_null() {
            *ppv = std::ptr::null_mut();
            return -2147467259i32; // E_OUTOFMEMORY
        }

        // QueryInterface on the factory for the requested IID
        let vtable = *(factory as *const *const c_void);
        let qi_fn = (*(vtable as *const class_factory::ClassFactoryVTable)).query_interface;
        let hr = qi_fn(factory, riid as *const GUID, ppv);

        // Release our initial reference
        let rel_fn = (*(vtable as *const class_factory::ClassFactoryVTable)).release;
        rel_fn(factory);

        hr
    }
}

/// COM registration export - DllCanUnloadNow
#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> i32 {
    if class_factory::can_unload_now() {
        0 // S_OK - can unload
    } else {
        1 // S_FALSE - cannot unload yet
    }
}

/// COM self-registration export
#[no_mangle]
pub extern "system" fn DllRegisterServer() -> i32 {
    // Registration is handled by NSIS installer / install.ps1 script
    0 // S_OK
}

/// COM self-unregistration export
#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> i32 {
    // Unregistration is handled by NSIS uninstaller / unregister.ps1 script
    0 // S_OK
}
