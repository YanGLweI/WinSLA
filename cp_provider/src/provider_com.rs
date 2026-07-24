//! ICredentialProvider COM implementation
//!
//! This is the main COM object that LogonUI interacts with to enumerate
//! and manage credential tiles on the login screen.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::GUID;

use crate::credential_com::DualAuthCredentialCom;

// ICredentialProvider IID: {87387110-4B45-4B18-9E46-93B1E4B0E4B4}
pub const IID_ICREDENTIAL_PROVIDER: GUID = GUID {
    data1: 0x87387110,
    data2: 0x4B45,
    data3: 0x4B18,
    data4: [0x9E, 0x46, 0x93, 0xB1, 0xE4, 0xB0, 0xE4, 0xB4],
};

/// Usage scenarios
const CPUS_LOGON: u32 = 1;
const CPUS_UNLOCK_WORKSTATION: u32 = 2;

/// ICredentialProvider vtable (IUnknown + 6 methods)
#[repr(C)]
pub struct ProviderVTable {
    // IUnknown
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // ICredentialProvider
    pub set_usage_scenario: unsafe extern "system" fn(*mut c_void, u32, u32) -> i32,
    pub set_serialization: unsafe extern "system" fn(*mut c_void, *const c_void) -> i32,
    pub advise: unsafe extern "system" fn(*mut c_void, *mut c_void, usize) -> i32,
    pub unadvise: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_credential_count: unsafe extern "system" fn(*mut c_void, *mut u32, *mut u32, *mut i32) -> i32,
    pub get_credential_at: unsafe extern "system" fn(*mut c_void, u32, *const GUID, *mut *mut c_void) -> i32,
}

/// The COM object for our Credential Provider
#[repr(C)]
pub struct DualAuthProviderCom {
    pub vtable: *const ProviderVTable,
    pub ref_count: AtomicU32,
    pub usage_scenario: u32,
    pub events: *mut c_void,       // ICredentialProviderEvents callback
    pub advise_context: usize,
    pub credential: *mut c_void,   // The single DualAuthCredentialCom instance
}

static PROVIDER_VTABLE: ProviderVTable = ProviderVTable {
    query_interface: provider_query_interface,
    add_ref: provider_add_ref,
    release: provider_release,
    set_usage_scenario: provider_set_usage_scenario,
    set_serialization: provider_set_serialization,
    advise: provider_advise,
    unadvise: provider_unadvise,
    get_credential_count: provider_get_credential_count,
    get_credential_at: provider_get_credential_at,
};

impl DualAuthProviderCom {
    /// Create a new provider instance, returns raw pointer with refcount=1
    pub fn create_instance() -> *mut c_void {
        let provider = Box::new(DualAuthProviderCom {
            vtable: &PROVIDER_VTABLE,
            ref_count: AtomicU32::new(1),
            usage_scenario: 0,
            events: std::ptr::null_mut(),
            advise_context: 0,
            credential: std::ptr::null_mut(),
        });
        Box::into_raw(provider) as *mut c_void
    }
}

unsafe extern "system" fn provider_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    let iid = &*riid;
    let iid_iunknown = GUID::from_u128(0x00000000_0000_0000_C000_000000000046);

    if *iid == iid_iunknown || *iid == IID_ICREDENTIAL_PROVIDER {
        *ppv = this;
        provider_add_ref(this);
        0 // S_OK
    } else {
        *ppv = std::ptr::null_mut();
        -2147467262i32 // E_NOINTERFACE
    }
}

unsafe extern "system" fn provider_add_ref(this: *mut c_void) -> u32 {
    let provider = &*(this as *const DualAuthProviderCom);
    provider.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn provider_release(this: *mut c_void) -> u32 {
    let provider = &*(this as *const DualAuthProviderCom);
    let count = provider.ref_count.fetch_sub(1, Ordering::Release) - 1;
    if count == 0 {
        let p = Box::from_raw(this as *mut DualAuthProviderCom);
        // Release the credential if we hold one
        if !p.credential.is_null() {
            let vtable = *(p.credential as *const *const c_void);
            let rel = (*(vtable as *const ProviderVTable)).release;
            rel(p.credential);
        }
    }
    count
}

unsafe extern "system" fn provider_set_usage_scenario(
    this: *mut c_void,
    cpus: u32,
    _flags: u32,
) -> i32 {
    // Only support logon and unlock scenarios
    if cpus == CPUS_LOGON || cpus == CPUS_UNLOCK_WORKSTATION {
        let provider = &mut *(this as *mut DualAuthProviderCom);
        provider.usage_scenario = cpus;
        0 // S_OK
    } else {
        -2147467263i32 // E_NOTIMPL - reject change password, credui, etc.
    }
}

unsafe extern "system" fn provider_set_serialization(
    _this: *mut c_void,
    _pcpcs: *const c_void,
) -> i32 {
    // We don't support pre-serialized credentials
    -2147467263i32 // E_NOTIMPL
}

unsafe extern "system" fn provider_advise(
    this: *mut c_void,
    events: *mut c_void,
    advise_context: usize,
) -> i32 {
    let provider = &mut *(this as *mut DualAuthProviderCom);
    provider.events = events;
    provider.advise_context = advise_context;
    // AddRef on the events callback
    if !events.is_null() {
        let vtable = *(events as *const *const c_void);
        let add_ref = (*(vtable as *const ProviderVTable)).add_ref;
        add_ref(events);
    }
    0 // S_OK
}

unsafe extern "system" fn provider_unadvise(this: *mut c_void) -> i32 {
    let provider = &mut *(this as *mut DualAuthProviderCom);
    if !provider.events.is_null() {
        let vtable = *(provider.events as *const *const c_void);
        let rel = (*(vtable as *const ProviderVTable)).release;
        rel(provider.events);
        provider.events = std::ptr::null_mut();
    }
    provider.advise_context = 0;
    0 // S_OK
}

unsafe extern "system" fn provider_get_credential_count(
    _this: *mut c_void,
    count: *mut u32,
    default_index: *mut u32,
    auto_logon: *mut i32,
) -> i32 {
    *count = 1;           // We have exactly 1 credential tile (dual-auth)
    *default_index = 0;   // First (only) tile is default
    *auto_logon = 0;      // FALSE - don't auto-logon
    0 // S_OK
}

unsafe extern "system" fn provider_get_credential_at(
    this: *mut c_void,
    index: u32,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    if index != 0 {
        return -2147024809i32; // E_INVALIDARG
    }

    let provider = &mut *(this as *mut DualAuthProviderCom);

    // Create credential on first request
    if provider.credential.is_null() {
        provider.credential = DualAuthCredentialCom::create_instance();
        if provider.credential.is_null() {
            return -2147467259i32; // E_OUTOFMEMORY
        }
    }

    // QueryInterface on the credential for the requested IID
    let vtable = *(provider.credential as *const *const c_void);
    let qi = (*(vtable as *const ProviderVTable)).query_interface;
    qi(provider.credential, riid, ppv)
}
