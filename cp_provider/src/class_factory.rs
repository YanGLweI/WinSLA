//! COM Class Factory for the Credential Provider
//!
//! Implements IClassFactory to allow COM to instantiate our Credential Provider.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

use windows::core::GUID;

use crate::provider_com::DualAuthProviderCom;

/// Global server lock count
static SERVER_LOCK_COUNT: AtomicU32 = AtomicU32::new(0);

/// IClassFactory vtable layout
#[repr(C)]
pub struct ClassFactoryVTable {
    // IUnknown
    pub query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    // IClassFactory
    pub create_instance: unsafe extern "system" fn(*mut c_void, *mut c_void, *const GUID, *mut *mut c_void) -> i32,
    pub lock_server: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

/// Class Factory COM object
#[repr(C)]
pub struct ClassFactory {
    pub vtable: *const ClassFactoryVTable,
    pub ref_count: AtomicU32,
}

static CLASS_FACTORY_VTABLE: ClassFactoryVTable = ClassFactoryVTable {
    query_interface: class_factory_query_interface,
    add_ref: class_factory_add_ref,
    release: class_factory_release,
    create_instance: class_factory_create_instance,
    lock_server: class_factory_lock_server,
};

/// Create a new ClassFactory instance and return it as a raw pointer
pub fn create_class_factory() -> *mut c_void {
    let factory = Box::new(ClassFactory {
        vtable: &CLASS_FACTORY_VTABLE,
        ref_count: AtomicU32::new(1),
    });
    Box::into_raw(factory) as *mut c_void
}

// IUnknown::QueryInterface
unsafe extern "system" fn class_factory_query_interface(
    this: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    let iid = &*riid;

    // IUnknown: {00000000-0000-0000-C000-000000000046}
    let iid_iunknown = GUID::from_u128(0x00000000_0000_0000_C000_000000000046);
    // IClassFactory: {00000001-0000-0000-C000-000000000046}
    let iid_iclassfactory = GUID::from_u128(0x00000001_0000_0000_C000_000000000046);

    if *iid == iid_iunknown || *iid == iid_iclassfactory {
        *ppv = this;
        class_factory_add_ref(this);
        0 // S_OK
    } else {
        *ppv = std::ptr::null_mut();
        -2147467262i32 // E_NOINTERFACE
    }
}

// IUnknown::AddRef
unsafe extern "system" fn class_factory_add_ref(this: *mut c_void) -> u32 {
    let factory = &*(this as *const ClassFactory);
    factory.ref_count.fetch_add(1, Ordering::Relaxed) + 1
}

// IUnknown::Release
unsafe extern "system" fn class_factory_release(this: *mut c_void) -> u32 {
    let factory = &*(this as *const ClassFactory);
    let count = factory.ref_count.fetch_sub(1, Ordering::Release) - 1;
    if count == 0 {
        drop(Box::from_raw(this as *mut ClassFactory));
    }
    count
}

// IClassFactory::CreateInstance
unsafe extern "system" fn class_factory_create_instance(
    _this: *mut c_void,
    outer: *mut c_void,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> i32 {
    if !outer.is_null() {
        return -2147467260i32; // CLASS_E_NOAGGREGATION
    }

    // Create the Credential Provider COM object
    let provider = DualAuthProviderCom::create_instance();
    if provider.is_null() {
        return -2147467259i32; // E_OUTOFMEMORY
    }

    // QueryInterface for the requested interface
    let vtable = *(provider as *const *const c_void);
    let qi = (*(vtable as *const ClassFactoryVTable)).query_interface;
    let hr = qi(provider, riid, ppv);

    // Release our initial reference (QI added one if successful)
    let rel = (*(vtable as *const ClassFactoryVTable)).release;
    rel(provider);

    hr
}

// IClassFactory::LockServer
unsafe extern "system" fn class_factory_lock_server(_this: *mut c_void, lock: i32) -> i32 {
    if lock != 0 {
        SERVER_LOCK_COUNT.fetch_add(1, Ordering::Relaxed);
    } else {
        SERVER_LOCK_COUNT.fetch_sub(1, Ordering::Relaxed);
    }
    0 // S_OK
}

/// Check if the server can be unloaded
pub fn can_unload_now() -> bool {
    SERVER_LOCK_COUNT.load(Ordering::Relaxed) == 0
}
