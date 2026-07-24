//! ICredentialProvider COM implementation
//!
//! This is the main COM object that LogonUI interacts with to enumerate
//! and manage credential tiles on the login screen.
//! Also implements ICredentialProviderSetUserArray for Windows 8+.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::GUID;

use crate::credential_com::DualAuthCredentialCom;

// ICredentialProvider IID: {d27c3481-5a1c-45b2-8aaa-c20ebbe8229e}
pub const IID_ICREDENTIAL_PROVIDER: GUID = GUID {
    data1: 0xd27c3481,
    data2: 0x5a1c,
    data3: 0x45b2,
    data4: [0x8a, 0xaa, 0xc2, 0x0e, 0xbb, 0xe8, 0x22, 0x9e],
};

// ICredentialProviderSetUserArray IID: {095c1484-1c0c-4388-9c6d-500e61bf84bd}
pub const IID_ICREDENTIAL_PROVIDER_SET_USER_ARRAY: GUID = GUID {
    data1: 0x095c1484,
    data2: 0x1c0c,
    data3: 0x4388,
    data4: [0x9c, 0x6d, 0x50, 0x0e, 0x61, 0xbf, 0x84, 0xbd],
};

/// Usage scenarios
const CPUS_LOGON: u32 = 1;
const CPUS_UNLOCK_WORKSTATION: u32 = 2;

/// Field types from CREDENTIAL_PROVIDER_FIELD_TYPE
const CPFT_SMALL_TEXT: u32 = 2;
const CPFT_EDIT_TEXT: u32 = 4;
const CPFT_PASSWORD_TEXT: u32 = 5;
const CPFT_SUBMIT_BUTTON: u32 = 9;

/// Number of fields in our credential tile
const FIELD_COUNT: u32 = 6;

/// ICredentialProvider vtable (IUnknown + 8 methods)
/// Plus ICredentialProviderSetUserArray shares the same object (QI returns same pointer)
#[repr(C)]
pub struct ProviderVTable {
    // IUnknown (3)
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // ICredentialProvider (8)
    pub set_usage_scenario: unsafe extern "system" fn(*mut c_void, u32, u32) -> i32,
    pub set_serialization: unsafe extern "system" fn(*mut c_void, *const c_void) -> i32,
    pub advise: unsafe extern "system" fn(*mut c_void, *mut c_void, usize) -> i32,
    pub unadvise: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_field_descriptor_count: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
    pub get_field_descriptor_at: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32,
    pub get_credential_count: unsafe extern "system" fn(*mut c_void, *mut u32, *mut u32, *mut i32) -> i32,
    pub get_credential_at: unsafe extern "system" fn(*mut c_void, u32, *const GUID, *mut *mut c_void) -> i32,
    // ICredentialProviderSetUserArray (1) - appended after ICredentialProvider
    pub set_user_array: unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32,
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
    pub user_array: *mut c_void,   // ICredentialProviderUserArray
}

static PROVIDER_VTABLE: ProviderVTable = ProviderVTable {
    query_interface: provider_query_interface,
    add_ref: provider_add_ref,
    release: provider_release,
    set_usage_scenario: provider_set_usage_scenario,
    set_serialization: provider_set_serialization,
    advise: provider_advise,
    unadvise: provider_unadvise,
    get_field_descriptor_count: provider_get_field_descriptor_count,
    get_field_descriptor_at: provider_get_field_descriptor_at,
    get_credential_count: provider_get_credential_count,
    get_credential_at: provider_get_credential_at,
    set_user_array: provider_set_user_array,
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
            user_array: std::ptr::null_mut(),
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

    if *iid == iid_iunknown || *iid == IID_ICREDENTIAL_PROVIDER || *iid == IID_ICREDENTIAL_PROVIDER_SET_USER_ARRAY {
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
        // Release user array
        if !p.user_array.is_null() {
            let vtable = *(p.user_array as *const *const c_void);
            let rel = (*(vtable as *const ProviderVTable)).release;
            rel(p.user_array);
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

/// CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR layout (x64, 32 bytes):
///   DWORD dwFieldID       (offset 0)
///   DWORD cpft            (offset 4)
///   LPWSTR pszLabel       (offset 8)
///   GUID guidFieldType    (offset 16, 16 bytes)
unsafe extern "system" fn provider_get_field_descriptor_count(
    _this: *mut c_void,
    count: *mut u32,
) -> i32 {
    *count = FIELD_COUNT;
    0 // S_OK
}

unsafe extern "system" fn provider_get_field_descriptor_at(
    _this: *mut c_void,
    index: u32,
    ppcpfd: *mut *mut c_void,
) -> i32 {
    if index >= FIELD_COUNT || ppcpfd.is_null() {
        return -2147024809i32; // E_INVALIDARG
    }

    // Field definitions: (type, label)
    let (cpft, label): (u32, &str) = match index {
        0 => (CPFT_EDIT_TEXT, "User A"),
        1 => (CPFT_PASSWORD_TEXT, "Password A"),
        2 => (CPFT_EDIT_TEXT, "User B"),
        3 => (CPFT_PASSWORD_TEXT, "Password B"),
        4 => (CPFT_SUBMIT_BUTTON, "Verify & Login"),
        5 => (CPFT_SMALL_TEXT, "Status"),
        _ => unreachable!(),
    };

    // Allocate CREDENTIAL_PROVIDER_FIELD_DESCRIPTOR with CoTaskMemAlloc (32 bytes on x64)
    let desc = windows::Win32::System::Com::CoTaskMemAlloc(32) as *mut u8;
    if desc.is_null() {
        return -2147467259i32; // E_OUTOFMEMORY
    }

    // Zero-initialize
    std::ptr::write_bytes(desc, 0, 32);

    // dwFieldID (offset 0)
    *(desc as *mut u32) = index;
    // cpft (offset 4)
    *(desc.add(4) as *mut u32) = cpft;

    // pszLabel (offset 8) - allocate wide string with CoTaskMemAlloc
    let label_wide: Vec<u16> = label.encode_utf16().chain(std::iter::once(0)).collect();
    let label_bytes = label_wide.len() * 2;
    let label_ptr = windows::Win32::System::Com::CoTaskMemAlloc(label_bytes) as *mut u16;
    if label_ptr.is_null() {
        windows::Win32::System::Com::CoTaskMemFree(Some(desc as *const c_void));
        return -2147467259i32;
    }
    std::ptr::copy_nonoverlapping(label_wide.as_ptr(), label_ptr, label_wide.len());
    *(desc.add(8) as *mut *mut u16) = label_ptr;

    // guidFieldType (offset 16) - already zeroed (GUID_NULL)

    *ppcpfd = desc as *mut c_void;
    0 // S_OK
}

unsafe extern "system" fn provider_get_credential_count(
    _this: *mut c_void,
    count: *mut u32,
    default_index: *mut u32,
    auto_logon: *mut i32,
) -> i32 {
    *count = 1;           // We have exactly 1 credential tile (dual-auth)
    *default_index = u32::MAX; // CREDENTIAL_PROVIDER_NO_DEFAULT
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

/// ICredentialProviderSetUserArray::SetUserArray
unsafe extern "system" fn provider_set_user_array(
    this: *mut c_void,
    users: *mut c_void,
) -> i32 {
    let provider = &mut *(this as *mut DualAuthProviderCom);

    // Release old array if any
    if !provider.user_array.is_null() {
        let vtable = *(provider.user_array as *const *const c_void);
        let rel = (*(vtable as *const ProviderVTable)).release;
        rel(provider.user_array);
    }

    // Store and AddRef new array
    provider.user_array = users;
    if !users.is_null() {
        let vtable = *(users as *const *const c_void);
        let add_ref = (*(vtable as *const ProviderVTable)).add_ref;
        add_ref(users);
    }

    0 // S_OK
}
