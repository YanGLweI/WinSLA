//! ICredentialProviderCredential COM implementation
//!
//! Handles the dual-account authentication UI tile on the Windows login screen.
//! Fields: User A name, User A password, User B name, User B password, Submit button, Status text.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::GUID;

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

// Field indices
const FIELD_USER_A_NAME: u32 = 0;
const FIELD_USER_A_PASS: u32 = 1;
const FIELD_USER_B_NAME: u32 = 2;
const FIELD_USER_B_PASS: u32 = 3;
const FIELD_SUBMIT: u32 = 4;
const FIELD_STATUS: u32 = 5;
const FIELD_COUNT: u32 = 6;

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

/// ICredentialProviderCredential vtable (IUnknown + 17 methods)
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
}

/// The credential COM object holding user input state
#[repr(C)]
pub struct DualAuthCredentialCom {
    pub vtable: *const CredentialVTable,
    pub ref_count: AtomicU32,
    pub events: *mut c_void,
    pub advise_context: usize,
    // Field values (stored as heap-allocated wide strings)
    pub user_a_name: Vec<u16>,
    pub user_a_pass: Vec<u16>,
    pub user_b_name: Vec<u16>,
    pub user_b_pass: Vec<u16>,
    pub status_text: Vec<u16>,
    // Authentication result
    pub auth_success: bool,
    pub serialized_user: Vec<u16>,  // DOMAIN\user for logon
    pub serialized_pass: Vec<u16>,
    // Nested stub for ICredentialProviderCredentialWithFieldOptions
    pub field_options_stub: *mut FieldOptionsStub,
    // Nested stub for ICredentialProviderCredentialWithSubmissionOptions
    pub submission_options_stub: *mut SubmissionOptionsStub,
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
    pub fn create_instance() -> *mut c_void {
        let cred = Box::new(DualAuthCredentialCom {
            vtable: &CREDENTIAL_VTABLE,
            ref_count: AtomicU32::new(1),
            events: std::ptr::null_mut(),
            advise_context: 0,
            user_a_name: to_wide(""),
            user_a_pass: to_wide(""),
            user_b_name: to_wide(""),
            user_b_pass: to_wide(""),
            status_text: to_wide("Enter credentials for both users"),
            auth_success: false,
            serialized_user: Vec::new(),
            serialized_pass: Vec::new(),
            field_options_stub: std::ptr::null_mut(),
            submission_options_stub: std::ptr::null_mut(),
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
    } else {
        // NOTE: ICredentialProviderCredential2 ({FD672C54-...}) must NOT be answered
        // with a 4-slot stub vtable. It INHERITS ICredentialProviderCredential, so
        // LogonUI expects the full 20-method vtable + GetUserArrayIndex. Returning a
        // short vtable corrupts LogonUI state and the tile disappears (v1.0.21 bug).
        // The interface is optional: v1.0.20 answered E_NOINTERFACE and tile showed fine.
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
    _this: *mut c_void, field: u32, state: *mut u32, interactive: *mut u32,
) -> i32 {
    crate::provider_com::trace(&format!("Credential::GetFieldState field={}", field));
    if field >= FIELD_COUNT {
        return -2147024809i32; // E_INVALIDARG
    }
    // All fields visible in both selected and deselected tiles
    *state = CPFS_DISPLAY_IN_BOTH;
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
        FIELD_USER_A_PASS => c.user_a_pass = new_val,
        FIELD_USER_B_NAME => c.user_b_name = new_val,
        FIELD_USER_B_PASS => c.user_b_pass = new_val,
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

    crate::provider_com::trace(&format!(
        "Credential::GetSerialization user_a='{}' user_b='{}'",
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
        fill_serialization(serialization, &user_a, &pass_a);
        log_serialization_bytes("GetSerialization(cached) FINAL struct", serialization);
        set_empty_status(status_text);
        *status_icon = 0;
        *response = CPGSR_RETURN_CREDENTIAL_FINISHED;
        return 0;
    }

    // Perform dual-account authentication via named pipe
    let user_a = wide_to_string(&c.user_a_name);
    let pass_a = wide_to_string(&c.user_a_pass);
    let user_b = wide_to_string(&c.user_b_name);
    let pass_b = wide_to_string(&c.user_b_pass);

    crate::provider_com::trace(&format!(
        "GetSerialization: pass_a_len={} pass_b_len={}",
        pass_a.len(), pass_b.len()
    ));

    if user_a.is_empty() || pass_a.is_empty() || user_b.is_empty() || pass_b.is_empty() {
        crate::provider_com::trace("GetSerialization: empty field, returning NOT_FINISHED");
        set_status(c, status_text, status_icon, "Please fill in all fields", CPSI_ERROR);
        *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
        return 0;
    }

    // Send auth request to service via pipe
    use crate::com_types::{AuthRequest, AuthResponse};
    let request = AuthRequest::new(user_a.clone(), &pass_a, user_b.clone(), &pass_b);

    crate::provider_com::trace("GetSerialization: connecting to pipe...");
    let auth_result = crate::pipe_client::send_auth_request(&request);
    crate::provider_com::trace(&format!("GetSerialization: pipe result={:?}", auth_result.is_ok()));

    match auth_result {
        Ok(AuthResponse::Success) => {
            c.auth_success = true;
            c.serialized_user = to_wide(&user_a);
            c.serialized_pass = to_wide(&pass_a);
            crate::provider_com::trace(&format!("GetSerialization: AUTH SUCCESS, serializing user='{}'", user_a));
            fill_serialization(serialization, &user_a, &pass_a);
            log_serialization_bytes("GetSerialization(success) FINAL struct", serialization);
            // IMPORTANT: When returning CREDENTIAL_FINISHED, status_text MUST be NULL
            // and status_icon MUST be CPSI_NONE (0) per Microsoft documentation.
            // Setting them to non-NULL causes Windows 25H2 LogonUI to reject the serialization.
            set_empty_status(status_text);
            *status_icon = 0; // CPSI_NONE
            *response = CPGSR_RETURN_CREDENTIAL_FINISHED;
            crate::provider_com::trace(&format!(
                "GetSerialization: returning response={} (CREDENTIAL_FINISHED), status=NULL", *response
            ));
            0
        }
        Ok(AuthResponse::FailUserA(msg)) => {
            set_status(c, status_text, status_icon, &format!("User A failed: {}", msg), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(AuthResponse::FailUserB(msg)) => {
            set_status(c, status_text, status_icon, &format!("User B failed: {}", msg), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(AuthResponse::BothFailed(a, b)) => {
            set_status(c, status_text, status_icon, &format!("Both failed: {} | {}", a, b), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Ok(_) => {
            set_status(c, status_text, status_icon, "Authentication failed", CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
        Err(e) => {
            crate::provider_com::trace(&format!("GetSerialization: PIPE ERROR: {}", e));
            set_status(c, status_text, status_icon, &format!("Service error: {}", e), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_NOT_FINISHED;
            0
        }
    }
}

unsafe extern "system" fn cred_report_result(
    _this: *mut c_void, ntstatus: i32, substatus: i32,
    _status_text: *mut *mut u16, _status_icon: *mut u32,
) -> i32 {
    crate::provider_com::trace(&format!(
        "ReportResult: ntstatus=0x{:08X} substatus=0x{:08X}",
        ntstatus as u32, substatus as u32
    ));
    0 // S_OK - acknowledge the result
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

/// Fill CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION using CredPackAuthenticationBufferW.
/// This is the standard format used by the built-in PasswordProvider.
/// With response=2 (CPGSR_RETURN_CREDENTIAL_FINISHED), LogonUI will process this buffer.
unsafe fn fill_serialization(serialization: *mut c_void, username: &str, password: &str) {
    // Verify struct layout at compile time (must be 32 bytes, matching SDK)
    const _: () = assert!(std::mem::size_of::<CredSerialization>() == 32);

    // Zero-initialize the entire struct first (clears padding bytes)
    let base = serialization as *mut u8;
    std::ptr::write_bytes(base, 0, std::mem::size_of::<CredSerialization>());

    // Extract bare username (strip domain prefix or UPN suffix)
    let bare_user = if let Some(pos) = username.find('\\') {
        &username[pos + 1..]
    } else if let Some(pos) = username.find('@') {
        &username[..pos]
    } else {
        username
    };

    // Get NetBIOS domain name (e.g. "HOT") via NetGetJoinInformation
    let domain = get_netbios_domain();

    // Construct fully-qualified username: "DOMAIN\user"
    let fq_user = if domain.is_empty() {
        bare_user.to_string()
    } else {
        format!("{}\\{}", domain, bare_user)
    };

    crate::provider_com::trace(&format!(
        "fill_serialization: fq_user='{}' password_len={} (CredPackAuthenticationBufferW)",
        fq_user, password.len()
    ));

    let user_wide: Vec<u16> = fq_user.encode_utf16().chain(std::iter::once(0)).collect();
    let pass_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();
    crate::provider_com::trace(&format!(
        "fill_serialization: user_wide_len={} pass_wide_len={}",
        user_wide.len(), pass_wide.len()
    ));

    // Get the correct Negotiate auth package ID from LSA
    let auth_pkg = get_negotiate_package_id();

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
        0, user_wide.as_ptr(), pass_wide.as_ptr(),
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
        0, user_wide.as_ptr(), pass_wide.as_ptr(),
        buffer, &mut cb,
    );
    if ok == 0 {
        crate::provider_com::trace("fill_serialization: CredPack pack FAILED");
        windows::Win32::System::Com::CoTaskMemFree(Some(buffer as *const c_void));
        return;
    }

    crate::provider_com::trace(&format!("fill_serialization: CredPack OK, {} bytes", cb));

    // DIAGNOSTIC: Dump FULL CredPack buffer content for analysis
    let buffer_bytes = std::slice::from_raw_parts(buffer, cb as usize);
    let hex_dump: String = buffer_bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    crate::provider_com::trace(&format!("CredPack FULL buffer ({} bytes): {}",
        buffer_bytes.len(), hex_dump));

    // Check for valid wide strings in the buffer
    let mut offset = 0;
    while offset + 2 <= buffer_bytes.len() {
        let wchar = u16::from_le_bytes([buffer_bytes[offset], buffer_bytes[offset + 1]]);
        if wchar == 0 {
            break;
        }
        if wchar < 32 || wchar > 126 {
            crate::provider_com::trace(&format!(
                "CredPack: Non-ASCII wchar at offset {}: 0x{:04x}", offset, wchar
            ));
        }
        offset += 2;
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
