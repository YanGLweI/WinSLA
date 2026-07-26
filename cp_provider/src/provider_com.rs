//! ICredentialProvider COM implementation
//!
//! This is the main COM object that LogonUI interacts with to enumerate
//! and manage credential tiles on the login screen.
//!
//! IMPORTANT (COM multi-interface correctness):
//! `ICredentialProvider` and `ICredentialProviderSetUserArray` are two distinct
//! COM interfaces and MUST be exposed through two distinct vtable pointers.
//! LogonUI (Windows 8+) calls `QueryInterface(IID_ICredentialProviderSetUserArray)`
//! and then invokes `SetUserArray`, which lives at vtable slot 3 of that interface
//! (`[QI, AddRef, Release, SetUserArray]`). Returning the ICredentialProvider
//! pointer for that QI would make LogonUI hit `SetUsageScenario` (slot 3 of the
//! provider vtable) instead, so the provider gets discarded and no tile shows.
//!
//! We therefore implement a nested `SetUserArrayStub` with its own vtable and
//! return its pointer for the SetUserArray QI. The stub forwards IUnknown to the
//! owner so both interfaces share a single reference count and lifetime.

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

// ─── Diagnostics ────────────────────────────────────────────────
// LogonUI runs on the secure desktop; a file trace is the only practical way
// to observe how far the credential-provider call chain gets during testing.

pub(crate) fn trace(msg: &str) {
    use std::io::Write;

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = format!("[{}] pid={} {}\n", ts, std::process::id(), msg);

    // Candidate log locations, in priority order. If the secure-desktop/SYSTEM
    // context cannot write under ProgramData, fall back to the TEMP directory
    // (for SYSTEM this is C:\Windows\Temp, which is always writable).
    let mut paths: Vec<String> = Vec::new();
    paths.push("C:\\ProgramData\\WinSLA\\cp_trace.log".to_string());
    if let Ok(tmp) = std::env::var("TEMP") {
        paths.push(format!("{}\\WinSLA_cp_trace.log", tmp));
    }
    paths.push("C:\\Windows\\Temp\\WinSLA_cp_trace.log".to_string());

    for path in &paths {
        // Best-effort ensure the parent directory exists.
        if let Some(parent) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = f.write_all(line.as_bytes());
            return;
        }
    }
}

/// ICredentialProvider vtable (IUnknown + 8 methods).
/// NOTE: SetUserArray is intentionally NOT part of this vtable; it belongs to a
/// separate ICredentialProviderSetUserArray interface exposed via a stub pointer.
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
    pub get_credential_at: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32,
}

/// ICredentialProviderSetUserArray vtable (IUnknown + 1 method).
/// SetUserArray MUST be at slot 3 for LogonUI to call it correctly.
#[repr(C)]
pub struct SetUserArrayVTable {
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
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
    pub sua_stub: *mut SetUserArrayStub, // nested interface stub (not ref-counted separately)
}

/// Nested stub exposing ICredentialProviderSetUserArray with its own vtable.
/// `owner` points back to the containing DualAuthProviderCom; IUnknown calls are
/// forwarded to the owner so both interfaces share one refcount and lifetime.
#[repr(C)]
pub struct SetUserArrayStub {
    pub vtable: *const SetUserArrayVTable,
    pub owner: *mut DualAuthProviderCom,
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
};

static SUA_VTABLE: SetUserArrayVTable = SetUserArrayVTable {
    query_interface: sua_query_interface,
    add_ref: sua_add_ref,
    release: sua_release,
    set_user_array: sua_set_user_array,
};

impl DualAuthProviderCom {
    /// Create a new provider instance, returns raw pointer with refcount=1
    pub fn create_instance() -> *mut c_void {
        trace("Provider::CreateInstance");

        let provider = Box::new(DualAuthProviderCom {
            vtable: &PROVIDER_VTABLE,
            ref_count: AtomicU32::new(1),
            usage_scenario: 0,
            events: std::ptr::null_mut(),
            advise_context: 0,
            credential: std::ptr::null_mut(),
            user_array: std::ptr::null_mut(),
            sua_stub: std::ptr::null_mut(),
        });
        let raw = Box::into_raw(provider);

        // Create the nested SetUserArray stub pointing back at the provider.
        let stub = Box::new(SetUserArrayStub {
            vtable: &SUA_VTABLE,
            owner: raw,
        });
        unsafe {
            (*raw).sua_stub = Box::into_raw(stub);
        }

        raw as *mut c_void
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
        trace("Provider::QI -> ICredentialProvider");
        0 // S_OK
    } else if *iid == IID_ICREDENTIAL_PROVIDER_SET_USER_ARRAY {
        // Return the nested stub pointer (its vtable has SetUserArray at slot 3).
        let provider = &*(this as *const DualAuthProviderCom);
        *ppv = provider.sua_stub as *mut c_void;
        provider_add_ref(this); // shared refcount on the owner
        trace("Provider::QI -> ICredentialProviderSetUserArray (stub)");
        0 // S_OK
    } else {
        *ppv = std::ptr::null_mut();
        trace(&format!("Provider::QI riid={:08X}-{:04X}-{:04X} -> E_NOINTERFACE",
            iid.data1, iid.data2, iid.data3));
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
            let cred_vtable = *(p.credential as *const *const c_void);
            use crate::credential_com::CredentialVTable;
            let rel = (*(cred_vtable as *const CredentialVTable)).release;
            rel(p.credential);
        }
        // Release user array
        if !p.user_array.is_null() {
            let vtable = *(p.user_array as *const *const c_void);
            let rel = (*(vtable as *const SetUserArrayVTable)).release;
            rel(p.user_array);
        }
        // Free the nested stub (it is not independently ref-counted)
        if !p.sua_stub.is_null() {
            drop(Box::from_raw(p.sua_stub));
        }
        trace("Provider::Release -> destroyed");
    }
    count
}

unsafe extern "system" fn provider_set_usage_scenario(
    this: *mut c_void,
    cpus: u32,
    _flags: u32,
) -> i32 {
    trace(&format!("Provider::SetUsageScenario cpus={} flags={}", cpus, _flags));
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
    trace("Provider::Advise");
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
    trace("Provider::GetFieldDescriptorCount");
    *count = FIELD_COUNT;
    0 // S_OK
}

unsafe extern "system" fn provider_get_field_descriptor_at(
    _this: *mut c_void,
    index: u32,
    ppcpfd: *mut *mut c_void,
) -> i32 {
    trace(&format!("Provider::GetFieldDescriptorAt index={}", index));
    if index >= FIELD_COUNT || ppcpfd.is_null() {
        return -2147024809i32; // E_INVALIDARG
    }

    // Field definitions: (type, label)
    let (cpft, label): (u32, &str) = match index {
        0 => (CPFT_EDIT_TEXT, "用户名 A"),
        1 => (CPFT_PASSWORD_TEXT, "密码 A"),
        2 => (CPFT_EDIT_TEXT, "用户名 B"),
        3 => (CPFT_PASSWORD_TEXT, "密码 B"),
        4 => (CPFT_SUBMIT_BUTTON, "验证并登录"),
        5 => (CPFT_SMALL_TEXT, "状态"),
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
        return -2147467259i32; // E_OUTOFMEMORY
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
    trace("Provider::GetCredentialCount -> 1");
    *count = 1;           // We have exactly 1 credential tile (dual-auth)
    *default_index = u32::MAX; // CREDENTIAL_PROVIDER_NO_DEFAULT
    *auto_logon = 0;      // FALSE - don't auto-logon
    0 // S_OK
}

unsafe extern "system" fn provider_get_credential_at(
    this: *mut c_void,
    index: u32,
    ppcpc: *mut *mut c_void,
) -> i32 {
    trace(&format!("Provider::GetCredentialAt index={} ppcpc={:p}", index, ppcpc));
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

    // Return the credential pointer. AddRef for the caller.
    *ppcpc = provider.credential;
    let cred_vtable = *(provider.credential as *const *const c_void);
    use crate::credential_com::CredentialVTable;
    let addref_fn = (*(cred_vtable as *const CredentialVTable)).add_ref;
    addref_fn(provider.credential);
    trace(&format!("Provider::GetCredentialAt -> S_OK credential={:p}", provider.credential));
    0 // S_OK
}

// ─── ICredentialProviderSetUserArray (nested stub) ──────────────

unsafe extern "system" fn sua_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    // Forward to the owner's QueryInterface so all interfaces stay consistent.
    let stub = &*(this as *const SetUserArrayStub);
    provider_query_interface(stub.owner as *mut c_void, riid, ppv)
}

unsafe extern "system" fn sua_add_ref(this: *mut c_void) -> u32 {
    let stub = &*(this as *const SetUserArrayStub);
    provider_add_ref(stub.owner as *mut c_void)
}

unsafe extern "system" fn sua_release(this: *mut c_void) -> u32 {
    let stub = &*(this as *const SetUserArrayStub);
    provider_release(stub.owner as *mut c_void)
}

unsafe extern "system" fn sua_set_user_array(
    this: *mut c_void,
    users: *mut c_void,
) -> i32 {
    trace("SetUserArray::SetUserArray");
    let stub = &*(this as *const SetUserArrayStub);
    let provider = &mut *(stub.owner as *mut DualAuthProviderCom);

    // Release old array if any
    if !provider.user_array.is_null() {
        let vtable = *(provider.user_array as *const *const c_void);
        let rel = (*(vtable as *const SetUserArrayVTable)).release;
        rel(provider.user_array);
    }

    // Store and AddRef new array
    provider.user_array = users;
    if !users.is_null() {
        let vtable = *(users as *const *const c_void);
        let add_ref = (*(vtable as *const SetUserArrayVTable)).add_ref;
        add_ref(users);
    }

    0 // S_OK
}
