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

use crate::credential_com::{DualAuthCredentialCom, TileType};

// Registry policy key path for reading default_tile_enabled setting
const REGISTRY_POLICY_KEY: &str = r"SOFTWARE\WinSLA\Policy";

#[link(name = "user32")]
unsafe extern "system" {
    fn GetSystemMetrics(nindex: i32) -> i32;
}
const SM_REMOTESESSION_VALUE: i32 = 0x1000;

/// Returns true when the current session is a remote (RDP) session.
/// LogonUI runs inside the session it displays UI for, so this reflects
/// the session being logged on to / unlocked.
fn is_remote_session() -> bool {
    unsafe { GetSystemMetrics(SM_REMOTESESSION_VALUE) != 0 }
}

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

// WinSLA private GUID - used to identify our own CP in Filter() implementation
pub const IID_WINSLA_PRIVATE_ID: GUID = GUID {
    data1: 0x8DAE8B3E,
    data2: 0x5C7A,
    data3: 0x4F9B,
    data4: [0xB2, 0xE1, 0x3A, 0xC4, 0xD5, 0xF6, 0xA7, 0xB8],
};

// ICredentialProviderFilter IID: {a5da53f9-d475-4080-a120-910c4a739880}
pub const IID_ICREDENTIAL_PROVIDER_FILTER: GUID = GUID {
    data1: 0xa5da53f9,
    data2: 0xd475,
    data3: 0x4080,
    data4: [0xa1, 0x20, 0x91, 0x0c, 0x4a, 0x73, 0x98, 0x80],
};

/// Usage scenarios
const CPUS_LOGON: u32 = 1;
const CPUS_UNLOCK_WORKSTATION: u32 = 2;

/// Field types from CREDENTIAL_PROVIDER_FIELD_TYPE
const CPFT_SMALL_TEXT: u32 = 2;
const CPFT_EDIT_TEXT: u32 = 4;
const CPFT_PASSWORD_TEXT: u32 = 5;
const CPFT_SUBMIT_BUTTON: u32 = 9;

/// Number of fields in the shared field descriptor set (union of both tiles)
const FIELD_COUNT: u32 = 7;

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

/// ICredentialProviderFilter vtable (IUnknown + 2 methods).
/// Based on CREDPROVIDERFILTER_INTERFACE_DEFINITION from SDK.
#[repr(C)]
pub struct FilterVTable {
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub filter: unsafe extern "system" fn(*mut c_void, u32, u32, *const GUID, *mut i32, u32) -> i32,
    pub update_remote_credential: unsafe extern "system" fn(*mut c_void, u32) -> i32,
}

/// The COM object for our Credential Provider
#[repr(C)]
pub struct DualAuthProviderCom {
    pub vtable: *const ProviderVTable,
    pub ref_count: AtomicU32,
    pub usage_scenario: u32,
    pub remote_session: bool,     // true when LogonUI runs in an RDP session
    pub events: *mut c_void,       // ICredentialProviderEvents callback
    pub advise_context: usize,
    pub credential_dual: *mut c_void,       // Dual-control tile (index 0)
    pub credential_emergency: *mut c_void,  // Emergency override tile (index 1)
    pub user_array: *mut c_void,   // ICredentialProviderUserArray
    pub sua_stub: *mut SetUserArrayStub, // nested interface stub (not ref-counted separately)
    pub filter_stub: *mut FilterStub,      // nested interface stub for ICredentialProviderFilter
}

/// Nested stub exposing ICredentialProviderSetUserArray with its own vtable.
/// `owner` points back to the containing DualAuthProviderCom; IUnknown calls are
/// forwarded to the owner so both interfaces share one refcount and lifetime.
#[repr(C)]
pub struct SetUserArrayStub {
    pub vtable: *const SetUserArrayVTable,
    pub owner: *mut DualAuthProviderCom,
}

/// Nested stub exposing ICredentialProviderFilter with its own vtable.
/// Similar design to SUAStub: IUnknown methods forward to owner, Filter methods implement policy.
#[repr(C)]
pub struct FilterStub {
    pub vtable: *const FilterVTable,
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

static FILTER_VTABLE: FilterVTable = FilterVTable {
    query_interface: filter_query_interface,
    add_ref: filter_add_ref,
    release: filter_release,
    filter: filter_filter,
    update_remote_credential: filter_update_remote_credential,
};

impl DualAuthProviderCom {
    /// Create a new provider instance, returns raw pointer with refcount=1
    pub fn create_instance() -> *mut c_void {
        trace("Provider::CreateInstance");

        let provider = Box::new(DualAuthProviderCom {
            vtable: &PROVIDER_VTABLE,
            ref_count: AtomicU32::new(1),
            usage_scenario: 0,
            remote_session: false,
            events: std::ptr::null_mut(),
            advise_context: 0,
            credential_dual: std::ptr::null_mut(),
            credential_emergency: std::ptr::null_mut(),
            user_array: std::ptr::null_mut(),
            sua_stub: std::ptr::null_mut(),
            filter_stub: std::ptr::null_mut(),
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

        // Create the nested Filter stub pointing back at the provider.
        let stub = Box::new(FilterStub {
            vtable: &FILTER_VTABLE,
            owner: raw,
        });
        unsafe {
            (*raw).filter_stub = Box::into_raw(stub);
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
    } else if *iid == IID_ICREDENTIAL_PROVIDER_FILTER {
        // Return the Filter stub pointer
        let provider = &*(this as *const DualAuthProviderCom);
        *ppv = provider.filter_stub as *mut c_void;
        provider_add_ref(this); // shared refcount on the owner
        trace("Provider::QI -> ICredentialProviderFilter (stub)");
        0 // S_OK
    } else if *iid == IID_WINSLA_PRIVATE_ID {
        // Private ID - our own marker for Filter() self-identification
        *ppv = this;
        provider_add_ref(this);
        trace("Provider::QI -> IID_WINSLA_PRIVATE_ID (self-id)");
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
        // Release both credential tiles if we hold them
        use crate::credential_com::CredentialVTable;
        for cred_ptr in [p.credential_dual, p.credential_emergency] {
            if !cred_ptr.is_null() {
                let cred_vtable = *(cred_ptr as *const *const c_void);
                let rel = (*(cred_vtable as *const CredentialVTable)).release;
                rel(cred_ptr);
            }
        }
        // Release user array
        if !p.user_array.is_null() {
            let vtable = *(p.user_array as *const *const c_void);
            let rel = (*(vtable as *const SetUserArrayVTable)).release;
            rel(p.user_array);
        }
        // Free the nested SUA stub (it is not independently ref-counted)
        if !p.sua_stub.is_null() {
            drop(Box::from_raw(p.sua_stub));
        }
        // Free the nested Filter stub
        if !p.filter_stub.is_null() {
            drop(Box::from_raw(p.filter_stub));
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
        provider.remote_session = is_remote_session();
        trace(&format!("Provider::SetUsageScenario remote_session={}", provider.remote_session));
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

    // Field definitions: (type, label) — union of both tiles; each credential
    // hides the fields that do not apply to it via GetFieldState.
    let (cpft, label): (u32, &str) = match index {
        0 => (CPFT_EDIT_TEXT, "用户名"),
        1 => (CPFT_PASSWORD_TEXT, "密码"),
        2 => (CPFT_EDIT_TEXT, "审批人"),
        3 => (CPFT_PASSWORD_TEXT, "审批人密码"),
        4 => (CPFT_EDIT_TEXT, "应急原因"),
        5 => (CPFT_SUBMIT_BUTTON, "验证并登录"),
        6 => (CPFT_SMALL_TEXT, "状态"),
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
    this: *mut c_void,
    count: *mut u32,
    default_index: *mut u32,
    auto_logon: *mut i32,
) -> i32 {
    // RDP-session unlock degradation (verified on TEST-WIN, 6 serialization
    // variants): winlogon submits third-party CP unlock credentials for a
    // REMOTE session through a RemoteInteractive path that fails
    // 0xC000006D/0xC00000E5 before any LSA audit, while built-in providers
    // are submitted with LogonType=7 and succeed. Our tiles can never work
    // in this scenario, so contribute zero tiles; the filter allows all
    // providers in the same scenario, leaving the built-in password tile.
    // Fresh logon (cpus=1, including RDP) and LOCAL unlock are unaffected
    // and remain dual-control.
    let provider = &*(this as *const DualAuthProviderCom);
    if provider.usage_scenario == CPUS_UNLOCK_WORKSTATION && provider.remote_session {
        trace("Provider::GetCredentialCount -> 0 (RDP-session unlock degraded to built-in)");
        *count = 0;
        *default_index = u32::MAX;
        *auto_logon = 0;
        return 0; // S_OK
    }
    trace("Provider::GetCredentialCount -> 2");
    *count = 2;           // Tile 0: dual-control login; Tile 1: emergency override
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
    if index > 1 {
        return -2147024809i32; // E_INVALIDARG
    }

    let provider = &mut *(this as *mut DualAuthProviderCom);

    // Select the slot for the requested tile and create it on first request
    let slot = if index == 0 {
        &mut provider.credential_dual
    } else {
        &mut provider.credential_emergency
    };

    if slot.is_null() {
        let tile_type = if index == 0 { TileType::Dual } else { TileType::Emergency };
        *slot = DualAuthCredentialCom::create_instance(tile_type);
        if slot.is_null() {
            return -2147467259i32; // E_OUTOFMEMORY
        }
    }

    // Keep the credential's view of the usage scenario in sync: GetUserSid
    // (ICredentialProviderCredential2) must return the locked user's SID only
    // in CPUS_UNLOCK_WORKSTATION and S_FALSE otherwise.
    (*(*slot as *mut DualAuthCredentialCom)).usage_scenario = provider.usage_scenario;

    // Return the credential pointer. AddRef for the caller.
    *ppcpc = *slot;
    let cred_vtable = *(*slot as *const *const c_void);
    use crate::credential_com::CredentialVTable;
    let addref_fn = (*(cred_vtable as *const CredentialVTable)).add_ref;
    addref_fn(*slot);
    trace(&format!("Provider::GetCredentialAt -> S_OK credential={:p}", *slot));
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

// ─── ICredentialProviderFilter (nested stub) ─────────────────────

unsafe extern "system" fn filter_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    // Forward to the owner's QueryInterface
    let stub = &*(this as *const FilterStub);
    provider_query_interface(stub.owner as *mut c_void, riid, ppv)
}

unsafe extern "system" fn filter_add_ref(this: *mut c_void) -> u32 {
    let stub = &*(this as *const FilterStub);
    provider_add_ref(stub.owner as *mut c_void)
}

unsafe extern "system" fn filter_release(this: *mut c_void) -> u32 {
    let stub = &*(this as *const FilterStub);
    provider_release(stub.owner as *mut c_void)
}

unsafe extern "system" fn filter_filter(
    this: *mut c_void,
    cpus: u32,
    _dwflags: u32,
    rgclsid_providers: *const GUID,
    rgballow: *mut i32,
    cproviders: u32,
) -> i32 {
    trace(&format!("Filter::Filter cpus={} cProviders={}", cpus, cproviders));
    
    // Only handle LOGON and UNLOCK_WORKSTATION scenarios
    if cpus != CPUS_LOGON && cpus != CPUS_UNLOCK_WORKSTATION {
        trace("Filter::Filter -> E_NOTIMPL (not logon/unlock scenario)");
        return -2147467263i32; // E_NOTIMPL
    }

    // RDP-session unlock degradation: our provider contributes zero tiles in
    // this scenario (see provider_get_credential_count), so built-in
    // providers MUST be allowed through or the session could never be
    // unlocked at all.
    if cpus == CPUS_UNLOCK_WORKSTATION && is_remote_session() {
        trace("Filter::Filter RDP-session unlock -> allow all (degraded)");
        for i in 0..cproviders {
            *rgballow.add(i as usize) = 1; // TRUE
        }
        return 0; // S_OK
    }

    // Read registry policy: if default_tile_enabled=true, don't filter anything
    let hide_others = match read_registry_default_tile_enabled() {
        Ok(Some(true)) => {
            trace("Filter::Filter policy=enabled, allow all");
            false
        },
        Ok(Some(false)) => {
            trace("Filter::Filter policy=disabled, hide others");
            true
        },
        _ => {
            trace("Filter::Filter policy read failed, allow all (fail-open)");
            false
        }
    };
    
    if !hide_others {
        // No filtering: allow all providers
        for i in 0..cproviders {
            *rgballow.add(i as usize) = 1; // TRUE
        }
        return 0; // S_OK
    }
    
    // Filtering is enabled - only allow our own CLSID (E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F)
    const CLSID_DUAL_AUTH_PROVIDER: GUID = GUID {
        data1: 0xE4D9F6E7,
        data2: 0x8A2B,
        data3: 0x4C3D,
        data4: [0x9E, 0x5F, 0x1A, 0x2B, 0x3C, 0x4D, 0x5E, 0x6F],
    };
    
    for i in 0..cproviders {
        let clsid = &*rgclsid_providers.add(i as usize);
        let is_self = clsid.data1 == CLSID_DUAL_AUTH_PROVIDER.data1
            && clsid.data2 == CLSID_DUAL_AUTH_PROVIDER.data2
            && clsid.data3 == CLSID_DUAL_AUTH_PROVIDER.data3
            && clsid.data4 == CLSID_DUAL_AUTH_PROVIDER.data4;
        
        *rgballow.add(i as usize) = if is_self { 1 } else { 0 };
        trace(&format!("Filter::Filter provider[{}] clsid={:08X} allow={}", 
            i, clsid.data1, if is_self { 1 } else { 0 }));
    }
    
    0 // S_OK
}

unsafe extern "system" fn filter_update_remote_credential(
    _this: *mut c_void,
    _dwflags: u32,
) -> i32 {
    // RDP credential update - not supported (return E_NOTIMPL)
    trace("Filter::UpdateRemoteCredential -> E_NOTIMPL");
    -2147467263i32 // E_NOTIMPL
}

// ─── Helper functions ───────────────────────────────────────────

/// Read DefaultTileEnabled from Windows Registry
/// Returns None if key doesn't exist or read fails
fn read_registry_default_tile_enabled() -> Result<Option<bool>, String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, HKEY, REG_SAM_FLAGS};
    
    // Windows API constants
    const KEY_READ: u32 = 0x20019; // KEY_READ access mask
    
    let wide_path: Vec<u16> = OsStr::new(REGISTRY_POLICY_KEY)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let mut hkey = HKEY(std::ptr::null_mut());
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(wide_path.as_ptr()),
            0,
            REG_SAM_FLAGS(KEY_READ),
            &mut hkey,
        )
    };
    
    if result != ERROR_SUCCESS {
        return Ok(None); // Key doesn't exist yet
    }
    
    let mut value: u32 = 0;
    let mut vsize: u32 = 4;
    let value_name = OsStr::new("DefaultTileEnabled").encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let result2 = unsafe {
        RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(value_name.as_ptr()),
            None,
            None,
            Some(&mut value as *mut _ as *mut u8),
            Some(&mut vsize),
        )
    };
    
    unsafe { RegCloseKey(hkey); }
    
    if result2 != ERROR_SUCCESS {
        Ok(None)
    } else {
        Ok(Some(value == 1))
    }
}

/// Read EmergencyRequiresReason from Windows Registry.
/// Returns true if the key doesn't exist or read fails (fail-safe: require reason).
pub(crate) fn read_registry_emergency_requires_reason() -> bool {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY_LOCAL_MACHINE, HKEY, REG_SAM_FLAGS};
    
    const KEY_READ: u32 = 0x20019;
    
    let wide_path: Vec<u16> = OsStr::new(REGISTRY_POLICY_KEY)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    
    let mut hkey = HKEY(std::ptr::null_mut());
    let result = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(wide_path.as_ptr()),
            0,
            REG_SAM_FLAGS(KEY_READ),
            &mut hkey,
        )
    };
    
    if result != ERROR_SUCCESS {
        return true; // Key doesn't exist -> fail-safe: require reason
    }
    
    let mut value: u32 = 0;
    let mut vsize: u32 = 4;
    let value_name = OsStr::new("EmergencyRequiresReason").encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
    let result2 = unsafe {
        RegQueryValueExW(
            hkey,
            windows::core::PCWSTR(value_name.as_ptr()),
            None,
            None,
            Some(&mut value as *mut _ as *mut u8),
            Some(&mut vsize),
        )
    };
    
    unsafe { RegCloseKey(hkey); }
    
    if result2 != ERROR_SUCCESS {
        true // Read failed -> fail-safe: require reason
    } else {
        value == 1
    }
}
