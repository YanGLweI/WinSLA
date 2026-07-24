//! ICredentialProviderCredential COM implementation
//!
//! Handles the dual-account authentication UI tile on the Windows login screen.
//! Fields: User A name, User A password, User B name, User B password, Submit button, Status text.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::GUID;

// ICredentialProviderCredential IID
pub const IID_ICREDENTIAL_PROVIDER_CREDENTIAL: GUID = GUID {
    data1: 0x87387110,
    data2: 0x4B45,
    data3: 0x4B18,
    data4: [0x9E, 0x46, 0x93, 0xB1, 0xE4, 0xB0, 0xE4, 0xB5],
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

// GetSerialization responses
const CPGSR_NO_CREDENTIAL_FINISHED: u32 = 0;
const CPGSR_CREDENTIAL_FINISHED: u32 = 1;

// Status icons
const CPSI_NONE: u32 = 0;
const CPSI_ERROR: u32 = 3;
const CPSI_SUCCESS: u32 = 2;

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
    pub get_submit_button_value: unsafe extern "system" fn(*mut c_void, u32, *mut u32, *mut *mut u16) -> i32,
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
        });
        Box::into_raw(cred) as *mut c_void
    }
}

// ─── IUnknown ────────────────────────────────────────────────────

unsafe extern "system" fn cred_query_interface(
    this: *mut c_void, riid: *const GUID, ppv: *mut *mut c_void,
) -> i32 {
    let iid = &*riid;
    let iid_iunknown = GUID::from_u128(0x00000000_0000_0000_C000_000000000046);
    if *iid == iid_iunknown || *iid == IID_ICREDENTIAL_PROVIDER_CREDENTIAL {
        *ppv = this;
        cred_add_ref(this);
        0
    } else {
        *ppv = std::ptr::null_mut();
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
    _this: *mut c_void, _field: u32, adjacent: *mut u32, label: *mut *mut u16,
) -> i32 {
    *adjacent = FIELD_STATUS; // Status text is adjacent to submit button
    let text = to_wide("Verify & Login");
    let bytes = text.len() * 2;
    let ptr = windows::Win32::System::Com::CoTaskMemAlloc(bytes) as *mut u16;
    if ptr.is_null() { return -2147467259i32; }
    std::ptr::copy_nonoverlapping(text.as_ptr(), ptr, text.len());
    *label = ptr;
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

    // If already authenticated, return the serialized credential
    if c.auth_success {
        *response = CPGSR_CREDENTIAL_FINISHED;
        return 0;
    }

    // Perform dual-account authentication via named pipe
    let user_a = wide_to_string(&c.user_a_name);
    let pass_a = wide_to_string(&c.user_a_pass);
    let user_b = wide_to_string(&c.user_b_name);
    let pass_b = wide_to_string(&c.user_b_pass);

    if user_a.is_empty() || pass_a.is_empty() || user_b.is_empty() || pass_b.is_empty() {
        set_status(c, status_text, status_icon, "Please fill in all fields", CPSI_ERROR);
        *response = CPGSR_NO_CREDENTIAL_FINISHED;
        return 0;
    }

    // Send auth request to service via pipe
    use crate::com_types::{AuthRequest, AuthResponse};
    let request = AuthRequest::new(user_a.clone(), &pass_a, user_b.clone(), &pass_b);

    match crate::pipe_client::send_auth_request(&request) {
        Ok(AuthResponse::Success) => {
            c.auth_success = true;
            // Store User A's credentials for Windows logon serialization
            c.serialized_user = to_wide(&user_a);
            c.serialized_pass = to_wide(&pass_a);
            set_status(c, status_text, status_icon, "Authentication successful!", CPSI_SUCCESS);
            *response = CPGSR_CREDENTIAL_FINISHED;

            // Fill the serialization structure with User A's credentials
            // The serialization struct is CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION
            // We use User A as the Windows logon identity
            fill_serialization(serialization, &user_a, &pass_a);
            0
        }
        Ok(AuthResponse::FailUserA(msg)) => {
            set_status(c, status_text, status_icon, &format!("User A failed: {}", msg), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_FINISHED;
            0
        }
        Ok(AuthResponse::FailUserB(msg)) => {
            set_status(c, status_text, status_icon, &format!("User B failed: {}", msg), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_FINISHED;
            0
        }
        Ok(AuthResponse::BothFailed(a, b)) => {
            set_status(c, status_text, status_icon, &format!("Both failed: {} | {}", a, b), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_FINISHED;
            0
        }
        Ok(_) => {
            set_status(c, status_text, status_icon, "Authentication failed", CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_FINISHED;
            0
        }
        Err(e) => {
            set_status(c, status_text, status_icon, &format!("Service error: {}", e), CPSI_ERROR);
            *response = CPGSR_NO_CREDENTIAL_FINISHED;
            0
        }
    }
}

unsafe extern "system" fn cred_report_result(
    _this: *mut c_void, _ntstatus: i32, _substatus: i32,
    _status_text: *mut *mut u16, _status_icon: *mut u32,
) -> i32 {
    0 // S_OK - acknowledge the result
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

/// Fill CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION with User A's credentials
/// Uses CredPackAuthenticationBufferW to create the serialized credential blob.
///
/// Structure layout (x64):
///   offset 0:  ULONG cbSize
///   offset 4:  ULONG ulAuthenticationPackage
///   offset 8:  GUID clsidCredentialProvider (16 bytes)
///   offset 24: ULONG cbSerialization
///   offset 32: byte* rgbSerialization (pointer, 8 bytes on x64)
unsafe fn fill_serialization(serialization: *mut c_void, username: &str, password: &str) {
    use windows::Win32::Security::Credentials::CredPackAuthenticationBufferW;
    use windows::core::PCWSTR;

    let user_wide: Vec<u16> = username.encode_utf16().chain(std::iter::once(0)).collect();
    let pass_wide: Vec<u16> = password.encode_utf16().chain(std::iter::once(0)).collect();

    // First call to get required buffer size
    let mut cb_size: u32 = 0;
    let _ = CredPackAuthenticationBufferW(
        windows::Win32::Security::Credentials::CRED_PACK_FLAGS(0),
        PCWSTR(user_wide.as_ptr()),
        PCWSTR(pass_wide.as_ptr()),
        None,
        &mut cb_size,
    );

    if cb_size == 0 {
        return;
    }

    // Allocate buffer with CoTaskMemAlloc (LogonUI will free it)
    let buffer = windows::Win32::System::Com::CoTaskMemAlloc(cb_size as usize) as *mut u8;
    if buffer.is_null() {
        return;
    }

    // Second call to fill the buffer
    let result = CredPackAuthenticationBufferW(
        windows::Win32::Security::Credentials::CRED_PACK_FLAGS(0),
        PCWSTR(user_wide.as_ptr()),
        PCWSTR(pass_wide.as_ptr()),
        Some(buffer),
        &mut cb_size,
    );

    if result.is_err() {
        windows::Win32::System::Com::CoTaskMemFree(Some(buffer as *const c_void));
        return;
    }

    // Fill the CREDENTIAL_PROVIDER_CREDENTIAL_SERIALIZATION struct
    let base = serialization as *mut u8;
    // cbSize = total struct size (40 bytes on x64)
    *(base as *mut u32) = 40;
    // ulAuthenticationPackage = 0 (Negotiate/default)
    *(base.add(4) as *mut u32) = 0;
    // clsidCredentialProvider (16 bytes at offset 8)
    let clsid = crate::dual_auth_credential::CLSID_DUAL_AUTH_PROVIDER;
    std::ptr::copy_nonoverlapping(
        &clsid as *const _ as *const u8,
        base.add(8),
        16,
    );
    // cbSerialization (4 bytes at offset 24)
    *(base.add(24) as *mut u32) = cb_size;
    // rgbSerialization (pointer at offset 32 on x64)
    *(base.add(32) as *mut *mut u8) = buffer;
}
