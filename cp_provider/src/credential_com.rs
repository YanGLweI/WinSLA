//! ICredentialProviderCredential COM implementation
//!
//! Handles the dual-account authentication UI tile on the Windows login screen.
//! Fields: User A name, User A password, User B name, User B password, Submit button, Status text.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::GUID;
use zeroize::Zeroizing;

// ICredentialProviderCredential IID: {63913a93-40c1-481a-818d-4072ff8c70cc}
pub const IID_ICREDENTIAL_PROVIDER_CREDENTIAL: GUID = GUID {
    data1: 0x63913a93,
    data2: 0x40c1,
    data3: 0x481a,
    data4: [0x81, 0x8d, 0x40, 0x72, 0xff, 0x8c, 0x70, 0xcc],
};

// ICredentialProviderCredentialWithFieldOptions IID: {DBC6FB30-C843-49E3-A645-573E6F39446A}
const IID_ICREDENTIAL_PROVIDER_CREDENTIAL_WITH_FIELD_OPTIONS: GUID = GUID {
    data1: 0xDBC6FB30,
    data2: 0xC843,
    data3: 0x49E3,
    data4: [0xA6, 0x45, 0x57, 0x3E, 0x6F, 0x39, 0x44, 0x6A],
};

// ICredentialProviderCredentialWithSubmissionOptions IID: {19844E8F-93E3-425A-9485-56A35726FE1C}
const IID_ICREDENTIAL_PROVIDER_CREDENTIAL_WITH_SUBMISSION_OPTIONS: GUID = GUID {
    data1: 0x19844E8F,
    data2: 0x93E3,
    data3: 0x425A,
    data4: [0x94, 0x85, 0x56, 0xA3, 0x57, 0x26, 0xFE, 0x1C],
};

/// IID_ICredentialProviderCredential2 = {FD672C54-40EA-4D6E-9B49-CFB1A7507BD7}
const IID_ICREDENTIAL_PROVIDER_CREDENTIAL2: GUID = GUID {
    data1: 0xFD672C54,
    data2: 0x40EA,
    data3: 0x4D6E,
    data4: [0x9B, 0x49, 0xCF, 0xB1, 0xA7, 0x50, 0x7B, 0xD7],
};

// Field indices (union of both tiles; visibility is controlled per-tile in GetFieldState)
const FIELD_USER_A_NAME: u32 = 0;   // Dual: primary account; Emergency: emergency account
const FIELD_USER_A_PASS: u32 = 1;
const FIELD_USER_B_NAME: u32 = 2;   // Dual only: approver
const FIELD_USER_B_PASS: u32 = 3;   // Dual only: approver password
const FIELD_REASON: u32 = 4;        // Emergency only: override reason
const FIELD_SUBMIT: u32 = 5;
const FIELD_STATUS: u32 = 6;
const FIELD_COUNT: u32 = 7;

/// Which login tile a credential instance represents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileType {
    /// Dual-control login (primary account + approver)
    Dual,
    /// Emergency override login (single authorized account + reason)
    Emergency,
}

// Field states
const CPFS_HIDDEN: u32 = 0;
const CPFS_DISPLAY_IN_SELECTED_TILE: u32 = 1;
const CPFS_DISPLAY_IN_BOTH: u32 = 3;

// Interactive states
const CPFIS_NONE: u32 = 0;
const CPFIS_FOCUSED: u32 = 3;

// GetSerialization responses (CREDENTIAL_PROVIDER_GET_SERIALIZATION_RESPONSE)
// MUST match the SDK enum exactly (credentialprovider.h, build 26100):
//   CPGSR_NO_CREDENTIAL_NOT_FINISHED      = 0
//   CPGSR_NO_CREDENTIAL_FINISHED          = 1
//   CPGSR_RETURN_CREDENTIAL_FINISHED      = 2  <- return a credential to LogonUI/LSA
//   CPGSR_RETURN_NO_CREDENTIAL_FINISHED   = 3  <- finished but NO credential (LogonUI ignores serialization!)
// Returning 3 when we mean 2 makes credprovhost skip LsaLogonUser entirely -> silent login loop.
const CPGSR_NO_CREDENTIAL_NOT_FINISHED: u32 = 0;
const CPGSR_NO_CREDENTIAL_FINISHED: u32 = 1;
const CPGSR_RETURN_CREDENTIAL_FINISHED: u32 = 2;
const CPGSR_RETURN_NO_CREDENTIAL_FINISHED: u32 = 3;

// Status icons
const CPSI_NONE: u32 = 0;
const CPSI_ERROR: u32 = 3;
const CPSI_SUCCESS: u32 = 2;

/// CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION (x64, 32 bytes)
/// MUST match the real Windows SDK layout (credentialprovider.h, build 26100):
///   ULONG  ulAuthenticationPackage;   // offset 0
///   GUID   clsidCredentialProvider;   // offset 4  (16 bytes, ends at 20)
///   ULONG  cbSerialization;           // offset 20
///   byte  *rgbSerialization;          // offset 28 (4 bytes padding at 24)
/// Total size = 32. NOTE: There is NO cbSize field. Adding one shifts every field
/// by 4 bytes and makes credprovhost.dll read the CLSID tail as cbSerialization
/// (crash: memmove with count = CLSID-tail 0xcda64ef7, src = NULL). Verified against
/// credprovhost disassembly which reads cbSerialization @ struct+20 ([rbp+0xd4])
/// and rgbSerialization @ struct+28 ([rbp+0xd8]).
#[repr(C)]
struct CredSerialization {
    ul_authentication_package: u32,  // offset 0
    clsid_credential_provider: GUID, // offset 4 (16 bytes, ends at 20)
    cb_serialization: u32,           // offset 20
    // 4 bytes implicit padding at offset 24 (aligns pointer to 8)
    rgb_serialization: *mut u8,      // offset 28
}

/// ICredentialProviderCredential2 vtable (IUnknown + 17 + GetUserSid).
/// The main credential object uses this 21-slot vtable directly, so a single
/// pointer satisfies both ICredentialProviderCredential (first 20 slots) and
/// ICredentialProviderCredential2 (all 21 slots) - classic C++ inheritance
/// layout, no stub objects, no this-pointer translation.
#[repr(C)]
pub struct CredentialVTable {
    // IUnknown (3)
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // ICredentialProviderCredential (17)
    pub advise: unsafe extern "system" fn(*mut c_void, *mut c_void, usize) -> i32,
    pub unadvise: unsafe extern "system" fn(*mut c_void) -> i32,
    pub set_selected: unsafe extern "system" fn(*mut c_void, *mut i32) -> i32,
    pub set_deselected: unsafe extern "system" fn(*mut c_void) -> i32,
    pub get_field_state: unsafe extern "system" fn(*mut c_void, u32, *mut u32, *mut u32) -> i32,
    pub get_string_value: unsafe extern "system" fn(*mut c_void, u32, *mut *mut u16) -> i32,
    pub get_bitmap_value: unsafe extern "system" fn(*mut c_void, u32, *mut isize) -> i32,
    pub get_checkbox_value: unsafe extern "system" fn(*mut c_void, u32, *mut i32, *mut *mut u16) -> i32,
    pub get_submit_button_value: unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> i32,
    pub get_combobox_value_count: unsafe extern "system" fn(*mut c_void, u32, *mut u32, *mut u32) -> i32,
    pub get_combobox_value_at: unsafe extern "system" fn(*mut c_void, u32, u32, *mut *mut u16) -> i32,
    pub set_string_value: unsafe extern "system" fn(*mut c_void, u32, *const u16) -> i32,
    pub set_checkbox_value: unsafe extern "system" fn(*mut c_void, u32, i32) -> i32,
    pub set_combobox_selected_value: unsafe extern "system" fn(*mut c_void, u32, u32) -> i32,
    pub command_link_clicked: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    pub get_serialization: unsafe extern "system" fn(*mut c_void, *mut u32, *mut c_void, *mut *mut u16, *mut u32) -> i32,
    pub report_result: unsafe extern "system" fn(*mut c_void, i32, i32, *mut *mut u16, *mut u32) -> i32,
    // ICredentialProviderCredential2 (1)
    pub get_user_sid: unsafe extern "system" fn(*mut c_void, *mut *mut u16) -> i32,
}

/// The credential COM object holding user input state
#[repr(C)]
pub struct DualAuthCredentialCom {
    pub vtable: *const CredentialVTable,
    pub ref_count: AtomicU32,
    pub events: *mut c_void,
    pub advise_context: usize,
    pub tile_type: TileType,
    // Field values (stored as heap-allocated wide strings; passwords zeroized on drop)
    pub user_a_name: Vec<u16>,
    pub user_a_pass: Zeroizing<Vec<u16>>,
    pub user_b_name: Vec<u16>,
    pub user_b_pass: Zeroizing<Vec<u16>>,
    pub emergency_reason: Zeroizing<Vec<u16>>,
    pub status_text: Vec<u16>,
    // Authentication result
    pub auth_success: bool,
    pub serialized_user: Vec<u16>,  // DOMAIN\user for logon
    pub serialized_pass: Zeroizing<Vec<u16>>,
    // Nested stub for ICredentialProviderCredentialWithFieldOptions
    pub field_options_stub: *mut FieldOptionsStub,
    // Nested stub for ICredentialProviderCredentialWithSubmissionOptions
    pub submission_options_stub: *mut SubmissionOptionsStub,
    // Synced from the provider's SetUsageScenario (CPUS_*); GetUserSid
    // behaviour differs between logon and unlock scenarios.
    pub usage_scenario: u32,
}

/// Nested stub exposing ICredentialProviderCredentialWithFieldOptions.
/// VTable: [QI, AddRef, Release, GetFieldOptions]
#[repr(C)]
pub struct FieldOptionsStub {
    pub vtable: *const FieldOptionsVTable,
    pub owner: *mut DualAuthCredentialCom,
}

#[repr(C)]
pub struct FieldOptionsVTable {
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_field_options: unsafe extern "system" fn(*mut c_void, u32, *mut u32) -> i32,
}

/// Nested stub exposing ICredentialProviderCredentialWithSubmissionOptions.
/// VTable: [QI, AddRef, Release, GetSubmissionOptions]
#[repr(C)]
pub struct SubmissionOptionsStub {
    pub vtable: *const SubmissionOptionsVTable,
    pub owner: *mut DualAuthCredentialCom,
}

#[repr(C)]
pub struct SubmissionOptionsVTable {
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_submission_options: unsafe extern "system" fn(*mut c_void, *mut u32) -> i32,
}

static CREDENTIAL_VTABLE: CredentialVTable = CredentialVTable {
    query_interface: cred_query_interface,
    add_ref: cred_add_ref,
    release: cred_release,
    advise: cred_advise,
    unadvise: cred_unadvise,
    set_selected: cred_set_selected,
    set_deselected: cred_set_deselected,
    get_field_state: cred_get_field_state,
    get_string_value: cred_get_string_value,
    get_bitmap_value: cred_get_bitmap_value,
    get_checkbox_value: cred_get_checkbox_value,
    get_submit_button_value: cred_get_submit_button_value,
    get_combobox_value_count: cred_get_combobox_value_count,
    get_combobox_value_at: cred_get_combobox_value_at,
    set_string_value: cred_set_string_value,
    set_checkbox_value: cred_set_checkbox_value,
    set_combobox_selected_value: cred_set_combobox_selected_value,
    command_link_clicked: cred_command_link_clicked,
    get_serialization: cred_get_serialization,
    report_result: cred_report_result,
    get_user_sid: cred_get_user_sid,
};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

static FIELD_OPTIONS_VTABLE: FieldOptionsVTable = FieldOptionsVTable {
    query_interface: fo_query_interface,
    add_ref: fo_add_ref,
    release: fo_release,
    get_field_options: fo_get_field_options,
};

static SUBMISSION_OPTIONS_VTABLE: SubmissionOptionsVTable = SubmissionOptionsVTable {
    query_interface: so_query_interface,
    add_ref: so_add_ref,
    release: so_release,
    get_submission_options: so_get_submission_options,
};

impl DualAuthCredentialCom {
    pub fn create_instance(tile_type: TileType) -> *mut c_void {
        let initial_status = match tile_type {
            TileType::Dual => "双控登录",
            TileType::Emergency => "应急登录（需授权账号）",
        };
        let cred = Box::new(DualAuthCredentialCom {
            vtable: &CREDENTIAL_VTABLE,
            ref_count: AtomicU32::new(1),
            events: std::ptr::null_mut(),
            advise_context: 0,
            tile_type,
            user_a_name: to_wide(""),
            user_a_pass: Zeroizing::new(to_wide("")),
            user_b_name: to_wide(""),
            user_b_pass: Zeroizing::new(to_wide("")),
            emergency_reason: Zeroizing::new(to_wide("")),
            status_text: to_wide(initial_status),
            auth_success: false,
            serialized_user: Vec::new(),
            serialized_pass: Zeroizing::new(Vec::new()),
            field_options_stub: std::ptr::null_mut(),
            submission_options_stub: std::ptr::null_mut(),
            usage_scenario: 0,
        });
        let ptr = Box::into_raw(cred) as *mut c_void;
        // Create the field options stub
        let stub = Box::new(FieldOptionsStub {
            vtable: &FIELD_OPTIONS_VTABLE,
            owner: ptr as *mut DualAuthCredentialCom,
        });
        unsafe { (*(ptr as *mut DualAuthCredentialCom)).field_options_stub = Box::into_raw(stub); }
        // Create the submission options stub
        let so_stub = Box::new(SubmissionOptionsStub {
            vtable: &SUBMISSION_OPTIONS_VTABLE,
            owner: ptr as *mut DualAuthCredentialCom,
        });
        unsafe { (*(ptr as *mut DualAuthCredentialCom)).submission_options_stub = Box::into_raw(so_stub); }
        // NOTE: Credential2Stub intentionally not created (see QI comment).
        ptr
    }
}

// ─── IUnknown ────────────────────────────────────────────────────

unsafe extern "system" fn cred_query_interface(
    this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void,
) -> i32 {
    let iid = &*riid;
    let iid_iunknown = GUID::from_u128(0x00000000_0000_0000_C000_000000000046);
    crate::provider_com::trace(&format!(
        "Credential::QI riid={:08X}-{:04X}-{:04X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
        iid.data1, iid.data2, iid.data3,
        iid.data4[0], iid.data4[1], iid.data4[2], iid.data4[3],
        iid.data4[4], iid.data4[5], iid.data4[6], iid.data4[7]
    ));
    if *iid == iid_iunknown || *iid == IID_ICREDENTIAL_PROVIDER_CREDENTIAL {
        *ppv = this;
        cred_add_ref(this);
        0
    } else if *iid == IID_ICREDENTIAL_PROVIDER_CREDENTIAL_WITH_FIELD_OPTIONS {
        // Return the nested FieldOptionsStub pointer
        let cred = &*(this as *const DualAuthCredentialCom);
        *ppv = cred.field_options_stub as *mut c_void;
        cred_add_ref(this); // share refcount with owner
        crate::provider_com::trace("Credential::QI -> ICredentialProviderCredentialWithFieldOptions");
        0
    } else if *iid == IID_ICREDENTIAL_PROVIDER_CREDENTIAL_WITH_SUBMISSION_OPTIONS {
        // Return the nested SubmissionOptionsStub pointer
        let cred = &*(this as *const DualAuthCredentialCom);
        *ppv = cred.submission_options_stub as *mut c_void;
        cred_add_ref(this); // share refcount with owner
        crate::provider_com::trace("Credential::QI -> ICredentialProviderCredentialWithSubmissionOptions");
        0
    } else if *iid == IID_ICREDENTIAL_PROVIDER_CREDENTIAL2 {
        // ICredentialProviderCredential2 INHERITS ICredentialProviderCredential.
        // Our main vtable is the full 21-slot vtable (20 base + GetUserSid),
        // so `this` satisfies both interfaces with classic C++ layout.
        // Never answer this IID with a short stub vtable: LogonUI would call
        // base methods through garbage slots and the tile vanishes (v1.0.21 bug).
        //
        // Only expose the interface in CPUS_UNLOCK_WORKSTATION: it exists
        // solely so winlogon can SID-match our tile to the locked user.
        // Answering it during fresh logon changes LogonUI's tile treatment
        // (custom bitmap ignored, default-tile selection altered) for no
        // benefit, so keep the v1.0.20 behavior (E_NOINTERFACE) there.
        let cred = &*(this as *const DualAuthCredentialCom);
        if cred.usage_scenario != 2 {
            *ppv = std::ptr::null_mut();
            crate::provider_com::trace("Credential::QI -> Credential2 withheld (logon scenario)");
            return -2147467262i32; // E_NOINTERFACE
        }
        *ppv = this;
        cred_add_ref(this);
        crate::provider_com::trace("Credential::QI -> ICredentialProviderCredential2");
        0
    } else {
        *ppv = std::ptr::null_mut();
        crate::provider_com::trace("Credential::QI -> E_NOINTERFACE");
        -2147467262i32
    }
}

unsafe extern "system" fn cred_add_ref(this: *mut c_void) -> u32 {
    let c = &*(this as *const DualAuthCredentialCom);
    c.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn cred_release(this: *mut c_void) -> u32 {
    let c = &*(this as *const DualAuthCredentialCom);
    let count = c.ref_count.fetch_sub(1, Ordering::Release) - 1;
    if count == 0 {
        drop(Box::from_raw(this as *mut DualAuthCredentialCom));
    }
    count
}

// ─── ICredentialProviderCredential ───────────────────────────────

unsafe extern "system" fn cred_advise(this: *mut c_void, events: *mut c_void, ctx: usize) -> i32 {
    let c = &mut *(this as *mut DualAuthCredentialCom);
    c.events = events;
    c.advise_context = ctx;
    0
}

unsafe extern "system" fn cred_unadvise(this: *mut c_void) -> i32 {
    let c = &mut *(this as *mut DualAuthCredentialCom);
    c.events = std::ptr::null_mut();
    0
}

unsafe extern "system" fn cred_set_selected(_this: *mut c_void, auto_logon: *mut i32) -> i32 {
    *auto_logon = 0; // Don't auto-logon
    0 // S_OK (not S_FALSE which would trigger auto-submit)
}

unsafe extern "system" fn cred_set_deselected(_this: *mut c_void) -> i32 {
    0
}

unsafe extern "system" fn cred_get_field_state(
    this: *mut c_void, field: u32, state: *mut u32, interactive: *mut u32,
) -> i32 {
    crate::provider_com::trace(&format!("Credential::GetFieldState field={}", field));
    if field >= FIELD_COUNT {
        return -2147024809i32; // E_INVALIDARG
    }
    let c = &*(this as *const DualAuthCredentialCom);
    // Per-tile field visibility:
    //   Dual tile      -> hides the emergency reason field
    //   Emergency tile -> hides the approver fields; hides reason if policy doesn't require it
    let visible = match c.tile_type {
        TileType::Dual => field != FIELD_REASON,
        TileType::Emergency => {
            if field == FIELD_USER_B_NAME || field == FIELD_USER_B_PASS {
                false
            } else if field == FIELD_REASON
                && !crate::provider_com::read_registry_emergency_requires_reason() {
                false // Policy doesn't require reason -> hide the field
            } else {
                true
            }
        }
    };
    *state = if visible { CPFS_DISPLAY_IN_BOTH } else { CPFS_HIDDEN };
    *interactive = if field == FIELD_USER_A_NAME { CPFIS_FOCUSED } else { CPFIS_NONE };
    0
}

unsafe extern "system" fn cred_get_string_value(
    this: *mut c_void, field: u32, value: *mut *mut u16,
) -> i32 {
    crate::provider_com::trace(&format!("Credential::GetStringValue field={}", field));
    let c = &*(this as *const DualAuthCredentialCom);
    let src = match field {
        FIELD_USER_A_NAME => &c.user_a_name,
        FIELD_USER_A_PASS => &c.user_a_pass,
        FIELD_USER_B_NAME => &c.user_b_name,
        FIELD_USER_B_PASS => &c.user_b_pass,
        FIELD_REASON => &c.emergency_reason,
        FIELD_STATUS => &c.status_text,
        _ => return -2147024809i32,
    };
    // Allocate with CoTaskMemAlloc
    let bytes = src.len() * 2;
    let ptr = windows::Win32::System::Com::CoTaskMemAlloc(bytes) as *mut u16;
    if ptr.is_null() {
        return -2147467259i32; // E_OUTOFMEMORY
    }
    std::ptr::copy_nonoverlapping(src.as_ptr(), ptr, src.len());
    *value = ptr;
    0
}

unsafe extern "system" fn cred_get_bitmap_value(
    _this: *mut c_void, _field: u32, _bitmap: *mut isize,
) -> i32 {
    -2147467263i32 // E_NOTIMPL - no custom bitmap
}

unsafe extern "system" fn cred_get_checkbox_value(
    _this: *mut c_void, _field: u32, _checked: *mut i32, _label: *mut *mut u16,
) -> i32 {
    -2147467263i32 // E_NOTIMPL - no checkboxes
}

unsafe extern "system" fn cred_get_submit_button_value(
    _this: *mut c_void, _field: u32, adjacent: *mut u32,
) -> i32 {
    *adjacent = FIELD_STATUS; // Status text is adjacent to submit button
    0
}

unsafe extern "system" fn cred_get_combobox_value_count(
    _this: *mut c_void, _field: u32, _count: *mut u32, _selected: *mut u32,
) -> i32 {
    -2147467263i32 // E_NOTIMPL
}

unsafe extern "system" fn cred_get_combobox_value_at(
    _this: *mut c_void, _field: u32, _index: u32, _value: *mut *mut u16,
) -> i32 {
    -2147467263i32 // E_NOTIMPL
}

unsafe extern "system" fn cred_set_string_value(
    this: *mut c_void, field: u32, value: *const u16,
) -> i32 {
    crate::provider_com::trace(&format!("Credential::SetStringValue field={}", field));
    let c = &mut *(this as *mut DualAuthCredentialCom);
    // Read the wide string
    let new_val = if value.is_null() {
        to_wide("")
    } else {
        let mut len = 0;
        while *value.add(len) != 0 { len += 1; }
        std::slice::from_raw_parts(value, len + 1).to_vec()
    };

    match field {
        FIELD_USER_A_NAME => c.user_a_name = new_val,
        FIELD_USER_A_PASS => c.user_a_pass = Zeroizing::new(new_val),
        FIELD_USER_B_NAME => c.user_b_name = new_val,
        FIELD_USER_B_PASS => c.user_b_pass = Zeroizing::new(new_val),
        FIELD_REASON => c.emergency_reason = Zeroizing::new(new_val),
        _ => return -2147024809i32,
    }
    0
}

unsafe extern "system" fn cred_set_checkbox_value(_this: *mut c_void, _field: u32, _val: i32) -> i32 {
    -2147467263i32
}

unsafe extern "system" fn cred_set_combobox_selected_value(_this: *mut c_void, _field: u32, _val: u32) -> i32 {
    -2147467263i32
}

unsafe extern "system" fn cred_command_link_clicked(_this: *mut c_void, _field: u32) -> i32 {
    -2147467263i32
}

unsafe extern "system" fn cred_get_serialization(
    this: *mut c_void,
    response: *mut u32,
    serialization: *mut c_void,
    status_text: *mut *mut u16,
    status_icon: *mut u32,
) -> i32 {
    let c = &mut *(this as *mut DualAuthCredentialCom);

    let (session_id, is_remote) = logon_environment();
    crate::provider_com::trace(&format!(
        "Credential::GetSerialization tile={:?} session={} remote={} user_a='{}' user_b='{}'",
        c.tile_type, session_id, is_remote,
        wide_to_string(&c.user_a_name), wide_to_string(&c.user_b_name)
    ));

    // If already authenticated, return the serialized credential
    if c.auth_success {
        let user_a = wide_to_string(&c.serialized_user);
        let pass_a = wide_to_string(&c.serialized_pass);
        crate::provider_com::trace(&format!(
            "GetSerialization: auth_success cached, re-serializing user='{}' pass_len={}",
            user_a, pass_a.len()
        ));
        fill_serialization(serialization, &user_a, &pass_a, c.usage_scenario);
        log_serialization_bytes("GetSerialization(cached) FINAL struct", serialization);
        set_empty_status(status_text);
        *status_icon = 0;
        *response = CPGSR_RETURN_CREDENTIAL_FINISHED;
        return 0;
    }

    match c.tile_type {
        TileType::Dual => serialize_dual(c, response, serialization, status_text, status_icon),
        TileType::Emergency => serialize_emergency(c, response, serialization, status_text, status_icon),
    }
}

/// Dual-control tile: verify both accounts with the service, then serialize the
/// primary account for the actual Windows logon.
unsafe fn serialize_dual(
    c: &mut DualAuthCredentialCom,
    response: *mut u32,
    serialization: *mut c_void,
    status_text: *mut *mut u16,
    status_icon: *mut u32,
) -> i32 {
    let user_a = wide_to_string(&c.user_a_name);
    let pass_a = wide_to_string(&c.user_a_pass);
    let user_b = wide_to_string(&c.user_b_name);
    let pass_b = wide_to_string(&c.user_b_pass);

    if user_a.is_empty() || pass_a.is_empty() || user_b.is_empty() || pass_b.is_empty() {
        set_status(c, status_text, status_icon, "请填写主账号和审批人的完整凭据", CPSI_ERROR);
        *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
        return 0;
    }

    // Send dual-auth request to service via pipe
    use crate::com_types::{AuthRequest, AuthResponse};
    let request = AuthRequest::new_dual(&user_a, &pass_a, &user_b, &pass_b, &logon_source_tag());

    crate::provider_com::trace("GetSerialization(dual): connecting to pipe...");
    let auth_result = crate::pipe_client::send_auth_request(&request);
    crate::provider_com::trace(&format!("GetSerialization(dual): pipe ok={:?}", auth_result.is_ok()));

    match auth_result {
        Ok(AuthResponse::Success { canonical_username }) => {
            // Use the service-canonicalized logon name verbatim (RDP fix): the
            // service validated this exact account; rebuilding the domain here
            // previously made LSA reject RDP logons (local accounts, ".\user"
            // input, UPNs and trusted domains all got the joined-domain prefix).
            let logon_name = if canonical_username.trim().is_empty() {
                user_a.clone()
            } else {
                canonical_username
            };
            c.auth_success = true;
            c.serialized_user = to_wide(&logon_name);
            c.serialized_pass = Zeroizing::new(to_wide(&pass_a));
            crate::provider_com::trace(&format!(
                "GetSerialization(dual): AUTH SUCCESS canonical='{}'", logon_name
            ));
            fill_serialization(serialization, &logon_name, &pass_a, c.usage_scenario);
            log_serialization_bytes("GetSerialization(dual success) FINAL struct", serialization);
            // IMPORTANT: When returning CREDENTIAL_FINISHED, status_text MUST be NULL
            // and status_icon MUST be CPSI_NONE (0) per Microsoft documentation.
            set_empty_status(status_text);
            *status_icon = 0; // CPSI_NONE
            *response = CPGSR_RETURN_CREDENTIAL_FINISHED;
            0
        }
        Ok(AuthResponse::Locked { remaining_secs }) => {
            let mins = (remaining_secs + 59) / 60;
            set_status(c, status_text, status_icon,
                &format!("失败次数过多，账号已锁定，请约 {} 分钟后重试", mins), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(AuthResponse::FailUserA(msg)) => {
            set_status(c, status_text, status_icon, &format!("验证失败：{}", msg), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(AuthResponse::FailUserB(msg)) => {
            set_status(c, status_text, status_icon, &format!("审批人验证失败：{}", msg), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(AuthResponse::BothFailed(a, b)) => {
            set_status(c, status_text, status_icon, &format!("验证失败：{} | {}", a, b), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(_) => {
            set_status(c, status_text, status_icon, "身份验证失败", CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Err(e) => {
            crate::provider_com::trace(&format!("GetSerialization(dual): PIPE ERROR: {}", e));
            set_status(c, status_text, status_icon, &format!("认证服务通信失败：{}", e), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
    }
}

/// Emergency tile: single authorized account + reason. The service checks the
/// emergency whitelist/policy and audits the override before we serialize.
unsafe fn serialize_emergency(
    c: &mut DualAuthCredentialCom,
    response: *mut u32,
    serialization: *mut c_void,
    status_text: *mut *mut u16,
    status_icon: *mut u32,
) -> i32 {
    let user = wide_to_string(&c.user_a_name);
    let pass = wide_to_string(&c.user_a_pass);
    let reason = wide_to_string(&c.emergency_reason);

    if user.is_empty() || pass.is_empty() {
        set_status(c, status_text, status_icon, "请输入应急账号和密码", CPSI_ERROR);
        *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
        return 0;
    }
    if crate::provider_com::read_registry_emergency_requires_reason() && reason.trim().is_empty() {
        set_status(c, status_text, status_icon, "请填写应急登录原因", CPSI_ERROR);
        *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
        return 0;
    }

    use crate::com_types::{AuthRequest, AuthResponse};
    let request = AuthRequest::new_emergency(&user, &pass, reason.trim(), &logon_source_tag());

    crate::provider_com::trace("GetSerialization(emergency): connecting to pipe...");
    let auth_result = crate::pipe_client::send_auth_request(&request);
    crate::provider_com::trace(&format!("GetSerialization(emergency): pipe ok={:?}", auth_result.is_ok()));

    match auth_result {
        Ok(AuthResponse::Success { canonical_username }) => {
            let logon_name = if canonical_username.trim().is_empty() {
                user.clone()
            } else {
                canonical_username
            };
            c.auth_success = true;
            c.serialized_user = to_wide(&logon_name);
            c.serialized_pass = Zeroizing::new(to_wide(&pass));
            crate::provider_com::trace(&format!(
                "GetSerialization(emergency): OVERRIDE APPROVED canonical='{}'", logon_name
            ));
            fill_serialization(serialization, &logon_name, &pass, c.usage_scenario);
            log_serialization_bytes("GetSerialization(emergency success) FINAL struct", serialization);
            set_empty_status(status_text);
            *status_icon = 0;
            *response = CPGSR_RETURN_CREDENTIAL_FINISHED;
            0
        }
        Ok(AuthResponse::Locked { remaining_secs }) => {
            let mins = (remaining_secs + 59) / 60;
            set_status(c, status_text, status_icon,
                &format!("失败次数过多，账号已锁定，请约 {} 分钟后重试", mins), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(AuthResponse::EmergencyDenied(msg)) => {
            set_status(c, status_text, status_icon, &format!("应急登录被拒绝：{}", msg), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(_) => {
            set_status(c, status_text, status_icon, "应急验证失败", CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Err(e) => {
            crate::provider_com::trace(&format!("GetSerialization(emergency): PIPE ERROR: {}", e));
            set_status(c, status_text, status_icon, &format!("认证服务通信失败：{}", e), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
    }
}

/// ICredentialProviderCredential2::GetUserSid.
///
/// Winlogon calls this during Post Initialization to associate the tile with
/// a known user. In CPUS_UNLOCK_WORKSTATION the association is MANDATORY:
/// submissions from tiles that do not SID-match the locked session's user are
/// rejected with 0xC000006D/0xC00000E5 before any Kerberos traffic - even
/// with a byte-correct KERB_INTERACTIVE_UNLOCK_LOGON including the right
/// LogonId (verified 2026-07-30: patched LogonId=0x84381, still rejected).
///
/// The unlock UI (LogonUI) runs inside the locked terminal session, so we
/// recover the interactive user's SID from that session's process tokens
/// (first process whose token user SID is S-1-5-21-*).
/// In fresh-logon scenarios there is no such user and we return S_FALSE,
/// keeping the tile an "Other User" tile exactly as before this interface
/// was implemented.
unsafe extern "system" fn cred_get_user_sid(this: *mut c_void, sid: *mut *mut u16) -> i32 {
    let c = &*(this as *const DualAuthCredentialCom);
    crate::provider_com::trace(&format!("Credential::GetUserSid scenario={}", c.usage_scenario));
    *sid = std::ptr::null_mut();
    // CPUS_UNLOCK_WORKSTATION = 2. Only the DUAL tile claims the locked user:
    // if the emergency tile also SID-matches, LogonUI treats it as a user tile
    // and picks it as the DEFAULT on the unlock screen (observed on TEST-WIN).
    if c.usage_scenario != 2 || !matches!(c.tile_type, TileType::Dual) {
        return 1; // S_FALSE - "Other User" tile (fresh logon UX unchanged)
    }
    if let Some(sid_str) = find_session_user_sid_string() {
        let wide = to_wide(&sid_str);
        let p = windows::Win32::System::Com::CoTaskMemAlloc(wide.len() * 2) as *mut u16;
        if !p.is_null() {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
            *sid = p;
            crate::provider_com::trace(&format!("Credential::GetUserSid -> {}", sid_str));
            return 0;
        }
    }
    crate::provider_com::trace("Credential::GetUserSid -> S_FALSE (no session user found)");
    1 // S_FALSE
}

/// Find the SID string ("S-1-5-21-...") of the interactive user logged on in
/// OUR terminal session. Used by GetUserSid for the unlock scenario.
unsafe fn find_session_user_sid_string() -> Option<String> {
    let mut our_session: u32 = 0;
    if ProcessIdToSessionId(std::process::id(), &mut our_session) == 0 {
        return None;
    }
    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snap == 0 || snap == -1isize as isize {
        return None;
    }
    let mut found: Option<String> = None;
    let mut pe: ProcessEntry32W = std::mem::zeroed();
    pe.size = std::mem::size_of::<ProcessEntry32W>() as u32;
    if Process32FirstW(snap, &mut pe) != 0 {
        loop {
            let mut proc_session: u32 = 0;
            if ProcessIdToSessionId(pe.process_id, &mut proc_session) != 0
                && proc_session == our_session
            {
                if let Some(s) = process_user_sid_string_if_interactive(pe.process_id) {
                    crate::provider_com::trace(&format!(
                        "find_session_user_sid_string: pid={} -> {}", pe.process_id, s
                    ));
                    found = Some(s);
                    break;
                }
            }
            if Process32NextW(snap, &mut pe) == 0 {
                break;
            }
        }
    }
    CloseHandle(snap);
    found
}

/// If process `pid` runs as an interactive account (user SID S-1-5-21-*),
/// return that SID as a string. Filters out SYSTEM (S-1-5-18), LOCAL/NETWORK
/// SERVICE (S-1-5-19/20), service SIDs (S-1-5-80-*), DWM (S-1-5-90-*), etc.
unsafe fn process_user_sid_string_if_interactive(pid: u32) -> Option<String> {
    let hproc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if hproc == 0 {
        return None;
    }
    let mut htok: isize = 0;
    let result = (|| {
        if OpenProcessToken(hproc, TOKEN_QUERY, &mut htok) == 0 {
            return None;
        }
        let mut need: u32 = 0;
        GetTokenInformation(htok, 1, std::ptr::null_mut(), 0, &mut need);
        if need == 0 || need > 4096 {
            return None;
        }
        let mut buf = vec![0u8; need as usize];
        if GetTokenInformation(htok, 1, buf.as_mut_ptr() as *mut c_void, need, &mut need) == 0 {
            return None;
        }
        let sid = *(buf.as_ptr() as *const *const u8);
        if sid.is_null() {
            return None;
        }
        // SID layout: Revision(1) SubAuthorityCount(1) IdentifierAuthority(6,
        // big-endian) SubAuthority[0](4, LE) ... Domain/local user accounts
        // have S-1-5-21-*: Rev=1, Auth=5, SubAuth[0]=21.
        if *sid != 1 || *sid.add(1) < 2 {
            return None;
        }
        if std::slice::from_raw_parts(sid.add(2), 6) != [0, 0, 0, 0, 0, 5] {
            return None;
        }
        let sub0 = u32::from_le_bytes([*sid.add(8), *sid.add(9), *sid.add(10), *sid.add(11)]);
        if sub0 != 21 {
            return None;
        }
        let mut str_ptr: *mut u16 = std::ptr::null_mut();
        if ConvertSidToStringSidW(sid as *const c_void, &mut str_ptr) == 0 || str_ptr.is_null() {
            return None;
        }
        let mut len = 0usize;
        while *str_ptr.add(len) != 0 {
            len += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(str_ptr, len));
        LocalFree(str_ptr as isize);
        Some(s)
    })();
    if htok != 0 {
        CloseHandle(htok);
    }
    CloseHandle(hproc);
    result
}

unsafe extern "system" fn cred_report_result(
    this: *mut c_void, ntstatus: i32, substatus: i32,
    status_text: *mut *mut u16, status_icon: *mut u32,
) -> i32 {
    crate::provider_com::trace(&format!(
        "ReportResult: ntstatus=0x{:08X} substatus=0x{:08X}",
        ntstatus as u32, substatus as u32
    ));
    // If the LSA logon failed (any NT_ERROR status, i.e. negative), invalidate the
    // cached authentication so the next GetSerialization re-reads the (possibly
    // corrected) input fields and re-validates against the service.
    //
    // Without this, a wrong password captured on the first attempt would be cached in
    // `serialized_pass` and re-sent on every retry forever -- because GetSerialization
    // short-circuits on `auth_success == true` and never re-reads the fields. This
    // manifested as a permanent "username or password is incorrect" loop that retyping
    // the correct credentials could not escape (especially at fresh boot, where a single
    // mistyped first attempt poisoned all subsequent attempts).
    if ntstatus < 0 {
        let c = &mut *(this as *mut DualAuthCredentialCom);
        c.auth_success = false;
        c.serialized_pass = Zeroizing::new(to_wide(""));
        crate::provider_com::trace(
            "ReportResult: logon failed, cleared cached auth_success so fields are re-read on next submit",
        );
        // Surface a specific reason on the tile instead of LogonUI's generic
        // "user name or password is incorrect" (crucial for RDP diagnostics).
        let msg = map_ntstatus_logon_error(ntstatus as u32);
        crate::provider_com::trace(&format!("ReportResult: {}", msg));
        set_status(c, status_text, status_icon, &msg, CPSI_ERROR);
    }
    0 // S_OK - acknowledge the result
}

/// Map common logon failure NTSTATUS codes to user-facing Chinese messages.
fn map_ntstatus_logon_error(ntstatus: u32) -> String {
    match ntstatus {
        0xC0000064 => "账号不存在，请检查用户名格式（域名\\用户名）".to_string(),
        0xC000006A => "Windows 拒绝了该密码".to_string(),
        0xC0000070 => "该账号不允许从这台计算机登录（工作站限制）".to_string(),
        0xC0000071 | 0xC0000224 => "密码已过期或必须更改密码".to_string(),
        0xC000015B => "未授予此登录类型的权限".to_string(),
        other => format!("Windows 登录失败 (0x{:08X})", other),
    }
}

// ─── ICredentialProviderCredentialWithFieldOptions (nested stub) ──

unsafe extern "system" fn fo_query_interface(
    this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void,
) -> i32 {
    let stub = &*(this as *const FieldOptionsStub);
    cred_query_interface(stub.owner as *mut c_void, riid, ppv)
}

unsafe extern "system" fn fo_add_ref(this: *mut c_void) -> u32 {
    let stub = &*(this as *const FieldOptionsStub);
    cred_add_ref(stub.owner as *mut c_void)
}

unsafe extern "system" fn fo_release(this: *mut c_void) -> u32 {
    let stub = &*(this as *const FieldOptionsStub);
    cred_release(stub.owner as *mut c_void)
}

unsafe extern "system" fn fo_get_field_options(
    _this: *mut c_void, _field: u32, options: *mut u32,
) -> i32 {
    *options = 0; // CPCFO_NONE
    0 // S_OK
}

// ─── ICredentialProviderCredentialWithSubmissionOptions (nested stub) ──

unsafe extern "system" fn so_query_interface(
    this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void,
) -> i32 {
    let stub = &*(this as *const SubmissionOptionsStub);
    cred_query_interface(stub.owner as *mut c_void, riid, ppv)
}

unsafe extern "system" fn so_add_ref(this: *mut c_void) -> u32 {
    let stub = &*(this as *const SubmissionOptionsStub);
    cred_add_ref(stub.owner as *mut c_void)
}

unsafe extern "system" fn so_release(this: *mut c_void) -> u32 {
    let stub = &*(this as *const SubmissionOptionsStub);
    cred_release(stub.owner as *mut c_void)
}

unsafe extern "system" fn so_get_submission_options(
    _this: *mut c_void, options: *mut u32,
) -> i32 {
    crate::provider_com::trace("SubmissionOptions::GetSubmissionOptions called");
    if !options.is_null() {
        *options = 0; // No special submission options
    }
    0 // S_OK
}

// ─── Helpers ─────────────────────────────────────────────────────

fn wide_to_string(wide: &[u16]) -> String {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..len])
}

const SM_REMOTESESSION: i32 = 0x1000;

/// Identify the logon environment for diagnostics and audit tagging.
/// Returns (session_id, is_remote_desktop).
pub(crate) fn logon_environment() -> (u32, bool) {
    let mut session_id: u32 = 0;
    unsafe {
        let _ = ProcessIdToSessionId(std::process::id(), &mut session_id);
    }
    let is_remote = unsafe { GetSystemMetrics(SM_REMOTESESSION) } != 0;
    (session_id, is_remote)
}

/// Audit tag for the current logon source: "console" or "rdp-session-N".
pub(crate) fn logon_source_tag() -> String {
    let (session_id, is_remote) = logon_environment();
    if is_remote {
        format!("rdp-session-{}", session_id)
    } else {
        "console".to_string()
    }
}

unsafe fn set_status(
    c: &mut DualAuthCredentialCom,
    status_text: *mut *mut u16,
    status_icon: *mut u32,
    msg: &str,
    icon: u32,
) {
    c.status_text = to_wide(msg);
    *status_icon = icon;

    let text = to_wide(msg);
    let bytes = text.len() * 2;
    let ptr = windows::Win32::System::Com::CoTaskMemAlloc(bytes) as *mut u16;
    if !ptr.is_null() {
        std::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len());
        *status_text = ptr;
    }
}

/// ANSI_STRING for LSA calls (matches Windows SDK layout)
#[repr(C)]
struct LsaAnsiString {
    length: u16,
    maximum_length: u16,
    buffer: *const u8,
}

/// PROCESSENTRY32W for Toolhelp process enumeration
#[repr(C)]
struct ProcessEntry32W {
    size: u32,
    cnt_usage: u32,
    process_id: u32,
    default_heap_id: usize,
    module_id: u32,
    cnt_threads: u32,
    parent_process_id: u32,
    pri_class_base: i32,
    flags: u32,
    exe_file: [u16; 260],
}

const TH32CS_SNAPPROCESS: u32 = 0x2;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const TOKEN_QUERY: u32 = 0x8;

/// Resolve an account name ("DOMAIN\\user" or UPN) to a binary SID.
unsafe fn sid_from_account_name(name: &str) -> Option<Vec<u8>> {
    let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut sid_len: u32 = 0;
    let mut dom_len: u32 = 0;
    let mut sid_use: u32 = 0;
    LookupAccountNameW(
        std::ptr::null(), name_w.as_ptr(), std::ptr::null_mut(), &mut sid_len,
        std::ptr::null_mut(), &mut dom_len, &mut sid_use,
    );
    if sid_len == 0 {
        return None;
    }
    let mut sid = vec![0u8; sid_len as usize];
    let mut dom = vec![0u16; dom_len as usize];
    let ok = LookupAccountNameW(
        std::ptr::null(), name_w.as_ptr(), sid.as_mut_ptr() as *mut c_void, &mut sid_len,
        dom.as_mut_ptr(), &mut dom_len, &mut sid_use,
    );
    if ok == 0 {
        return None;
    }
    Some(sid)
}

/// If process `pid` runs as the user identified by `target_sid`, return its
/// token's AuthenticationId (the LUID of that user's logon session).
unsafe fn process_token_logon_id_if_user(pid: u32, target_sid: &[u8]) -> Option<u64> {
    let hproc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if hproc == 0 {
        return None;
    }
    let mut htok: isize = 0;
    let result = (|| {
        if OpenProcessToken(hproc, TOKEN_QUERY, &mut htok) == 0 {
            return None;
        }
        // TokenUser (class 1)
        let mut need: u32 = 0;
        GetTokenInformation(htok, 1, std::ptr::null_mut(), 0, &mut need);
        if need == 0 || need > 4096 {
            return None;
        }
        let mut buf = vec![0u8; need as usize];
        if GetTokenInformation(htok, 1, buf.as_mut_ptr() as *mut c_void, need, &mut need) == 0 {
            return None;
        }
        let sid_ptr = *(buf.as_ptr() as *const *mut c_void);
        if EqualSid(sid_ptr, target_sid.as_ptr() as *mut c_void) == 0 {
            return None;
        }
        // TokenStatistics (class 10): TokenId@0, AuthenticationId@8, 56 bytes total
        let mut stats = [0u8; 64];
        let mut stats_len: u32 = 0;
        if GetTokenInformation(htok, 10, stats.as_mut_ptr() as *mut c_void, 64, &mut stats_len) == 0 {
            return None;
        }
        let auth_id = u64::from_le_bytes(stats[8..16].try_into().ok()?);
        Some(auth_id)
    })();
    if htok != 0 {
        CloseHandle(htok);
    }
    CloseHandle(hproc);
    result
}

/// Find the LUID of an existing interactive logon session for `canonical_user`
/// ("DOMAIN\\user") in OUR terminal session (the one LogonUI is running in).
/// A match exists exactly in the CPUS_UNLOCK_WORKSTATION case: winlogon then
/// expects KERB_INTERACTIVE_UNLOCK_LOGON.LogonId to reference that locked
/// session, but only patches it for tiles it can SID-match via
/// ICredentialProviderCredential2::GetUserSid (which we deliberately do not
/// implement). Without it the Kerberos package receives LogonId=0 and the
/// unlock dies with 0xC000006D / 0xC00000E5 before any KDC traffic.
///
/// NOTE: LsaEnumerateLogonSessions cannot be used here - LogonUI's SYSTEM
/// token is stripped of SeTcbPrivilege (LsaRegisterLogonProcess fails).
/// Instead we enumerate processes in our session and read the matching user
/// token's AuthenticationId (no special privilege required).
/// For fresh logons there is no user process in the session and the LogonId
/// stays zero. Fail-safe: any lookup error -> None -> old behavior.
unsafe fn find_locked_session_logon_id(canonical_user: &str) -> Option<u64> {
    let target_sid = sid_from_account_name(canonical_user)?;

    let mut our_session: u32 = 0;
    if ProcessIdToSessionId(std::process::id(), &mut our_session) == 0 {
        return None;
    }

    let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snap == 0 || snap == -1isize as isize {
        return None;
    }

    let mut found: Option<u64> = None;
    let mut pe: ProcessEntry32W = std::mem::zeroed();
    pe.size = std::mem::size_of::<ProcessEntry32W>() as u32;
    if Process32FirstW(snap, &mut pe) != 0 {
        loop {
            let mut proc_session: u32 = 0;
            if ProcessIdToSessionId(pe.process_id, &mut proc_session) != 0
                && proc_session == our_session
            {
                if let Some(luid) = process_token_logon_id_if_user(pe.process_id, &target_sid) {
                    crate::provider_com::trace(&format!(
                        "find_locked_session_logon_id: match pid={} session={} luid=0x{:x}",
                        pe.process_id, proc_session, luid
                    ));
                    found = Some(luid);
                    break;
                }
            }
            if Process32NextW(snap, &mut pe) == 0 {
                break;
            }
        }
    }
    CloseHandle(snap);
    if found.is_none() {
        crate::provider_com::trace("find_locked_session_logon_id: no matching user process in session");
    }
    found
}

#[link(name = "secur32")]
extern "system" {
    fn LsaConnectUntrusted(lsa_handle: *mut isize) -> i32;
    fn LsaRegisterLogonProcess(
        logon_process_name: *const LsaAnsiString,
        lsa_handle: *mut isize,
        security_mode: *mut u32,
    ) -> i32;
    fn LsaLookupAuthenticationPackage(
        lsa_handle: isize, package_name: *const LsaAnsiString, auth_package: *mut u32,
    ) -> i32;
    fn LsaDeregisterLogonProcess(lsa_handle: isize) -> i32;
    fn LsaLogonUser(
        lsa_handle: isize,
        origin_name: *const LsaAnsiString,
        logon_type: u32,
        auth_package: u32,
        protocol_submit_buffer: *const c_void,
        submit_buffer_length: u32,
        local_groups: *const c_void,
        source_context: *const c_void,
        profile_buffer: *mut *mut c_void,
        profile_buffer_length: *mut u32,
        logon_id: *mut c_void,
        token: *mut isize,
        quotas: *mut c_void,
        sub_status: *mut i32,
    ) -> i32;
    fn LsaFreeReturnBuffer(buffer: *mut c_void) -> i32;
}

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: isize, desired_access: u32, token: *mut isize) -> i32;
    fn GetTokenInformation(
        token: isize, token_information_class: u32, token_information: *mut c_void,
        token_information_length: u32, return_length: *mut u32,
    ) -> i32;
    fn LookupAccountNameW(
        system_name: *const u16, account_name: *const u16, sid: *mut c_void,
        cb_sid: *mut u32, referenced_domain_name: *mut u16, cch_domain_name: *mut u32,
        pe_use: *mut u32,
    ) -> i32;
    fn EqualSid(sid1: *const c_void, sid2: *const c_void) -> i32;
    fn ConvertSidToStringSidW(sid: *const c_void, string_sid: *mut *mut u16) -> i32;
    fn LookupPrivilegeValueW(
        system_name: *const u16, privilege_name: *const u16, luid: *mut u64,
    ) -> i32;
    fn AdjustTokenPrivileges(
        token: isize, disable_all: i32, new_state: *const c_void,
        buffer_length: u32, prev_state: *mut c_void, return_length: *mut u32,
    ) -> i32;
    fn LogonUserW(
        username: *const u16, domain: *const u16, password: *const u16,
        logon_type: u32, logon_provider: u32, token: *mut isize,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> isize;
    fn CloseHandle(handle: isize) -> i32;
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
    fn OpenProcess(desired_access: u32, inherit_handle: i32, process_id: u32) -> isize;
    fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> isize;
    fn Process32FirstW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snapshot: isize, entry: *mut ProcessEntry32W) -> i32;
    fn LocalFree(mem: isize) -> isize;
}

#[link(name = "user32")]
extern "system" {
    fn GetSystemMetrics(n_index: i32) -> i32;
}

#[link(name = "netapi32")]
extern "system" {
    fn NetGetJoinInformation(
        server: *const u16,
        name_buffer: *mut *mut u16,
        buffer_type: *mut u32,
    ) -> u32;
    fn NetApiBufferFree(buffer: *mut c_void) -> u32;
}

#[link(name = "credui")]
extern "system" {
    fn CredPackAuthenticationBufferW(
        dw_flags: u32,
        psz_username: *const u16,
        psz_password: *const u16,
        p_packed_credentials: *mut u8,
        pcb_packed_credentials: *mut u32,
    ) -> i32; // BOOL
}

/// Get authentication package IDs from LSA.
/// Returns the Kerberos package ID (for use with manually-packed KERB buffer).
/// Also looks up MSV1_0 and Negotiate for diagnostics.
unsafe fn get_negotiate_package_id() -> u32 {
    let mut handle: isize = 0;
    let status = LsaConnectUntrusted(&mut handle);
    if status != 0 {
        crate::provider_com::trace(&format!("LsaConnectUntrusted failed: 0x{:08X}", status as u32));
        return 2; // Fallback: typical Kerberos ID
    }
    crate::provider_com::trace(&format!("LSA handle={:#x}", handle));

    // Look up all three packages for diagnostics
    let mut lookup = |name: &[u8]| -> (i32, u32) {
        let ansi = LsaAnsiString {
            length: name.len() as u16,
            maximum_length: (name.len() + 1) as u16,
            buffer: name.as_ptr(),
        };
        let mut pkg: u32 = 0xFFFFFFFF;
        let st = LsaLookupAuthenticationPackage(handle, &ansi, &mut pkg);
        (st, pkg)
    };

    let (msv_st, msv_pkg) = lookup(b"MSV1_0");
    let (neg_st, neg_pkg) = lookup(b"Negotiate");
    let (kerb_st, kerb_pkg) = lookup(b"Kerberos");

    crate::provider_com::trace(&format!(
        "PkgIDs: MSV1_0(st=0x{:08X},id={}) Negotiate(st=0x{:08X},id={}) Kerberos(st=0x{:08X},id={})",
        msv_st as u32, msv_pkg, neg_st as u32, neg_pkg, kerb_st as u32, kerb_pkg
    ));

    let _ = LsaDeregisterLogonProcess(handle);

    // Use Negotiate for empty-domain KERB buffer (handles auto-domain resolution)
    if neg_st == 0 && neg_pkg != 0xFFFFFFFF {
        return neg_pkg;
    }
    // Fallback: Kerberos
    if kerb_st == 0 && kerb_pkg != 0xFFFFFFFF {
        return kerb_pkg;
    }
    0 // Last resort: assume Negotiate is 0
}

/// Look up the Kerberos authentication package ID via an untrusted LSA
/// connection. Returns 0xFFFFFFFF on failure.
unsafe fn get_kerberos_package_id() -> u32 {
    let mut handle: isize = 0;
    if LsaConnectUntrusted(&mut handle) != 0 {
        return 0xFFFFFFFF;
    }
    let ansi = LsaAnsiString {
        length: 8,
        maximum_length: 9,
        buffer: b"Kerberos".as_ptr(),
    };
    let mut pkg: u32 = 0xFFFFFFFF;
    let st = LsaLookupAuthenticationPackage(handle, &ansi, &mut pkg);
    let _ = LsaDeregisterLogonProcess(handle);
    if st != 0 {
        0xFFFFFFFF
    } else {
        pkg
    }
}

/// Get the machine's NetBIOS name via GetComputerNameExW (ComputerNameNetBIOS=3)
unsafe fn get_machine_name() -> String {
    use windows::Win32::System::SystemInformation::{
GetComputerNameExW, COMPUTER_NAME_FORMAT,
    };
    let mut size: u32 = 0;
let _ = GetComputerNameExW(COMPUTER_NAME_FORMAT(3), windows::core::PWSTR::null(), &mut size);
    if size == 0 {
        return String::new();
    }
    let mut buf: Vec<u16> = vec![0; size as usize];
let ok = GetComputerNameExW(
        COMPUTER_NAME_FORMAT(3),
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut size,
    );
    if ok.is_err() || size == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..size as usize])
}

/// Get the machine's DNS domain name via GetComputerNameExW
unsafe fn get_machine_domain() -> String {
    use windows::Win32::System::SystemInformation::{
        GetComputerNameExW, COMPUTER_NAME_FORMAT,
    };

    // ComputerNameDnsDomain = 2
    let mut size: u32 = 0;
    let _ = GetComputerNameExW(COMPUTER_NAME_FORMAT(2), windows::core::PWSTR::null(), &mut size);
    if size == 0 {
        return String::new();
    }
    let mut buf: Vec<u16> = vec![0; size as usize];
    let ok = GetComputerNameExW(
        COMPUTER_NAME_FORMAT(2),
        windows::core::PWSTR(buf.as_mut_ptr()),
        &mut size,
    );
    if ok.is_err() || size == 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..size as usize])
}

/// Get the NetBIOS domain name via NetGetJoinInformation.
/// Returns e.g. "HOT" for a machine joined to domain HOT.
unsafe fn get_netbios_domain() -> String {
    let mut name_buf: *mut u16 = std::ptr::null_mut();
    let mut buf_type: u32 = 0;
    let status = NetGetJoinInformation(
        std::ptr::null(),
        &mut name_buf,
        &mut buf_type,
    );
    if status != 0 || name_buf.is_null() {
        crate::provider_com::trace(&format!("NetGetJoinInformation failed: status={}", status));
        return String::new();
    }
    let result = wide_ptr_to_string(name_buf);
    NetApiBufferFree(name_buf as *mut c_void);
    // NETSETUP_JOIN_STATUS: 0=Unknown 1=Unjoined 2=WorkgroupName 3=DomainName.
    // Only a real domain join yields a usable logon domain - a workgroup name
    // must never be prefixed onto the username (RDP "WORKGROUP\user" bug).
    if buf_type != 3 {
        crate::provider_com::trace(&format!(
            "NetGetJoinInformation: not domain-joined (type={}), ignoring '{}'", buf_type, result
        ));
        return String::new();
    }
    crate::provider_com::trace(&format!("NetGetJoinInformation: domain='{}' type={}", result, buf_type));
    result
}

/// Convert a wide string pointer to a Rust String.
unsafe fn wide_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = std::slice::from_raw_parts(ptr, len);
    String::from_utf16_lossy(slice)
}

/// Allocate an empty wide string via CoTaskMemAlloc for status_text.
/// LogonUI on Windows 25H2 may crash (ucrtbase.dll ACCESS_VIOLATION) if status_text is NULL.
unsafe fn set_empty_status(status_text: *mut *mut u16) {
    let ptr = windows::Win32::System::Com::CoTaskMemAlloc(2) as *mut u16;
    if !ptr.is_null() {
        *ptr = 0; // null terminator only
        *status_text = ptr;
    } else {
        *status_text = std::ptr::null_mut();
    }
}

/// Dump the raw bytes of the serialization struct for crash correlation.
unsafe fn log_serialization_bytes(tag: &str, serialization: *mut c_void) {
    let p = serialization as *const u8;
    let mut hex = String::new();
    for i in 0..32usize {
        hex.push_str(&format!("{:02x} ", *p.add(i)));
    }
    crate::provider_com::trace(&format!(
        "{}: ptr=0x{:x} bytes(32)=[{}]", tag, serialization as usize, hex.trim()
    ));
}

/// CredPackAuthenticationBufferW flag: CredProtect the password field while
/// keeping the KERB_INTERACTIVE_UNLOCK_LOGON layout (verified by local probe:
/// identical 64-byte header, only the password becomes a protected blob).
const CRED_PACK_PROTECTED_CREDENTIALS: u32 = 0x1;

/// Fill CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION using CredPackAuthenticationBufferW.
/// This is the standard format used by the built-in PasswordProvider.
/// With response=2 (CPGSR_RETURN_CREDENTIAL_FINISHED), LogonUI will process this buffer.
unsafe fn fill_serialization(
    serialization: *mut c_void, username: &str, password: &str, usage_scenario: u32,
) {
    // Verify struct layout at compile time (must be 32 bytes, matching SDK)
    const _: () = assert!(std::mem::size_of::<CredSerialization>() == 32);

    // Zero-initialize the entire struct first (clears padding bytes)
    let base = serialization as *mut u8;
    std::ptr::write_bytes(base, 0, std::mem::size_of::<CredSerialization>());

    // The caller passes the service-canonicalized logon name ("DOMAIN\user",
    // "MACHINE\user" or a UPN). Pack it VERBATIM - rebuilding the domain here
    // was the root cause of LSA rejecting RDP logons (local accounts became
    // "WORKGROUP\user", and ".\user" / UPN / trusted-domain input was silently
    // rewritten to the machine's joined domain).
    let fq_user = if username.contains('\\') || username.contains('@') {
        username.to_string()
    } else {
        // Defensive fallback for bare names without canonical info (legacy
        // service): prefix only a real joined domain, never a workgroup name.
        let domain = get_netbios_domain();
        if domain.is_empty() {
            username.to_string()
        } else {
            format!("{}\\{}", domain, username)
        }
    };

    // Packing flags history (verified on TEST-WIN, HOT domain):
    // - flags=0 (plaintext): fresh logons OK, but CPUS_UNLOCK_WORKSTATION is
    //   rejected with 0xC000006D / 0xC00000E5 before any KDC traffic - the
    //   unlock path requires a protected password (Microsoft sample comment:
    //   "CredPackAuthenticationBuffer() cannot be used because it won't work
    //   with unlock scenario" - i.e. not with plaintext packing).
    // - flags=0x8 (CRED_PACK_ID_PROVIDER_CREDENTIALS): produces a CloudAP/
    //   NegoExtender blob (NOT KERB layout) -> 0xC0000003 in every scenario.
    // - flags=0x1 (CRED_PACK_PROTECTED_CREDENTIALS): same KERB layout with
    //   CredProtect'ed password - the format the built-in PasswordProvider
    //   uses; works for logon and unlock alike.
    let pack_flags: u32 = 0; // plaintext: KerbUnlockLogon never CredUnprotects (with GetUserSid+LogonId patch)

    crate::provider_com::trace(&format!(
        "fill_serialization: fq_user='{}' password_len={} pack_flags=0x{:X} (CredPackAuthenticationBufferW)",
        fq_user, password.len(), pack_flags
    ));

    let user_wide: Vec<u16> = fq_user.encode_utf16().chain(std::iter::once(0)).collect();
    let pass_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();
    crate::provider_com::trace(&format!(
        "fill_serialization: user_wide_len={} pass_wide_len={}",
        user_wide.len(), pass_wide.len()
    ));

    // Auth package selection:
    // - Fresh logon (cpus=1) or local account: Negotiate (proven working for
    //   every logon variant; its Kerberos/NTLM dispatch is correct there).
    // - UNLOCK (cpus=2) of a domain account: the Kerberos package DIRECTLY.
    //   winlogon calls LsaLogonUser with the locked session's ORIGINAL logon
    //   type - for RDP sessions that is RemoteInteractive(10), and Negotiate
    //   then dispatches the KERB-formatted buffer to MSV1_0, which cannot
    //   parse MessageType=2 and fails 0xC000006D/0xC00000E5 before any audit
    //   event (verified on TEST-WIN: local unlock with LogonType=7 succeeds,
    //   RDP-session unlock fails; built-in PasswordProvider serializes domain
    //   credentials to the Kerberos package for the same reason).
    let auth_pkg = {
        let domain_part = fq_user.split('\\').next().unwrap_or("");
        let is_domain_account = fq_user.contains('@')
            || (!domain_part.is_empty()
                && !domain_part.eq_ignore_ascii_case(&get_machine_name()));
        if usage_scenario == 2 && is_domain_account {
            let kerb = get_kerberos_package_id();
            if kerb != 0xFFFFFFFF {
                crate::provider_com::trace(
                    "fill_serialization: unlock + domain account -> Kerberos package"
                );
                kerb
            } else {
                get_negotiate_package_id()
            }
        } else {
            get_negotiate_package_id()
        }
    };

    // Use OUR provider's CLSID (matches registry registration {E4D9F6E7-...}).
    // Using PasswordProvider's CLSID here causes the PasswordProvider's
    // ICredentialProviderFilter to intercept/mishandle the serialization, leading to
    // a silent LSA logon failure (login UI loop). The buffer format is determined by
    // the authentication package, NOT the CLSID, so our own CLSID is correct here.
    let clsid_cp: GUID = crate::CLSID_DUAL_AUTH_PROVIDER;

    // TEST MODE: Empty serialization to isolate structure vs buffer content issues
    // If LogonUI still crashes with empty serialization, the issue is in the structure/interface.
    // If LogonUI doesn't crash, the issue is in the CredPack buffer content.
    #[cfg(feature = "empty_serialization_test")]
    {
        let ser = &mut *(serialization as *mut CredSerialization);
        ser.ul_authentication_package = auth_pkg;
        ser.clsid_credential_provider = clsid_cp;
        ser.cb_serialization = 0;  // Empty serialization
        ser.rgb_serialization = std::ptr::null_mut();
        crate::provider_com::trace("fill_serialization: EMPTY TEST MODE - cb_serialization=0, rgb_serialization=NULL");
        return;
    }

    // Normal path: Use CredPackAuthenticationBufferW
    // First call: get required buffer size
    let mut cb: u32 = 0;
    let rc1 = CredPackAuthenticationBufferW(
        pack_flags, user_wide.as_ptr(), pass_wide.as_ptr(),
        std::ptr::null_mut(), &mut cb,
    );
    crate::provider_com::trace(&format!(
        "fill_serialization: CredPack size-query rc={} cb={} GetLastError={}",
        rc1, cb, std::io::Error::last_os_error()
    ));
    if cb == 0 {
        crate::provider_com::trace("fill_serialization: CredPack size query FAILED");
        return;
    }

    // Allocate and pack
    let buffer = windows::Win32::System::Com::CoTaskMemAlloc(cb as usize) as *mut u8;
    if buffer.is_null() {
        crate::provider_com::trace("fill_serialization: CoTaskMemAlloc FAILED");
        return;
    }
    let ok = CredPackAuthenticationBufferW(
        pack_flags, user_wide.as_ptr(), pass_wide.as_ptr(),
        buffer, &mut cb,
    );
    if ok == 0 {
        crate::provider_com::trace("fill_serialization: CredPack pack FAILED");
        windows::Win32::System::Com::CoTaskMemFree(Some(buffer as *const c_void));
        return;
    }

    crate::provider_com::trace(&format!("fill_serialization: CredPack OK, {} bytes", cb));

    // DIAGNOSTIC: dump only the buffer header (struct layout/offsets). Never
    // dump the full buffer: plaintext packing contains the clear-text password.
    let dump_len = std::cmp::min(cb as usize, 64);
    let buffer_bytes = std::slice::from_raw_parts(buffer, dump_len);
    let hex_dump: String = buffer_bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    crate::provider_com::trace(&format!(
        "CredPack buffer header ({} of {} bytes): {}", dump_len, cb, hex_dump
    ));

    // CPUS_UNLOCK_WORKSTATION fix: patch KERB_INTERACTIVE_UNLOCK_LOGON.LogonId
    // (offset 56, right after the three UNICODE_STRINGs) with the locked
    // session's LUID so the Kerberos package can associate the unlock with
    // the existing logon session. No-op for fresh logons (no session found).
    if cb >= 64 {
        if let Some(luid) = find_locked_session_logon_id(&fq_user) {
            *(buffer.add(56) as *mut u64) = luid;
            crate::provider_com::trace(&format!(
                "fill_serialization: patched LogonId=0x{:x} into unlock buffer", luid
            ));
        }
    }

    // Fill using the typed struct (ensures correct offsets and padding)
    let ser = &mut *(serialization as *mut CredSerialization);
    ser.ul_authentication_package = auth_pkg;
    ser.clsid_credential_provider = clsid_cp;
    ser.cb_serialization = cb;
    ser.rgb_serialization = buffer;

    // Dump struct bytes for diagnostics
    let bytes = std::slice::from_raw_parts(base, 32);
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    crate::provider_com::trace(&format!(
        "fill_serialization: DONE cb={} auth_pkg={} fq_user='{}' ptr={:p}",
        cb, auth_pkg, fq_user, buffer
    ));
    crate::provider_com::trace(&format!("fill_serialization: struct hex: {}", hex));
}

/// Diagnostic: test credentials with LogonUserW API.
/// This doesn't require SeTcbPrivilege and tests if credentials + domain resolution work.
unsafe fn diag_lsa_logon(username: &str, password: &str) {
    let user_w: Vec<u16> = username.encode_utf16().chain(std::iter::once(0)).collect();
    let pass_w: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();

    // Try with NULL domain (auto-resolve)
    let mut token: isize = 0;
    // LOGON32_LOGON_INTERACTIVE=2, LOGON32_PROVIDER_DEFAULT=0
    let ok = LogonUserW(
        user_w.as_ptr(),
        std::ptr::null(),  // domain = NULL (auto-resolve)
        pass_w.as_ptr(),
        2,  // LOGON32_LOGON_INTERACTIVE
        0,  // LOGON32_PROVIDER_DEFAULT
        &mut token,
    );
    if ok != 0 {
        crate::provider_com::trace("DIAG: LogonUserW(NULL domain) SUCCESS");
        CloseHandle(token);
    } else {
        let err = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        crate::provider_com::trace(&format!("DIAG: LogonUserW(NULL domain) FAILED err={}", err));
    }

    // Try with empty string domain
    let empty_domain: Vec<u16> = "\0".encode_utf16().collect();
    let mut token2: isize = 0;
    let ok2 = LogonUserW(
        user_w.as_ptr(),
        empty_domain.as_ptr(),  // domain = "" (empty)
        pass_w.as_ptr(),
        2,
        0,
        &mut token2,
    );
    if ok2 != 0 {
        crate::provider_com::trace("DIAG: LogonUserW(empty domain) SUCCESS");
        CloseHandle(token2);
    } else {
        let err2 = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        crate::provider_com::trace(&format!("DIAG: LogonUserW(empty domain) FAILED err={}", err2));
    }

    // Try with WINNT50 provider (Kerberos)
    let mut token3: isize = 0;
    // LOGON32_PROVIDER_WINNT50=3
    let ok3 = LogonUserW(
        user_w.as_ptr(),
        std::ptr::null(),
        pass_w.as_ptr(),
        2,
        3,  // LOGON32_PROVIDER_WINNT50
        &mut token3,
    );
    if ok3 != 0 {
        crate::provider_com::trace("DIAG: LogonUserW(WINNT50) SUCCESS");
        CloseHandle(token3);
    } else {
        let err3 = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
        crate::provider_com::trace(&format!("DIAG: LogonUserW(WINNT50) FAILED err={}", err3));
    }
}
