//! Raw FFI bindings to Apple frameworks: IOKit, CoreFoundation, IOSurface, libobjc

#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    dead_code,
    clippy::not_unsafe_ptr_arg_deref
)]

use std::ffi::{c_char, c_void, CStr};
use std::ptr;

// ── Type aliases ──

pub type io_object_t = u32;
pub type io_service_t = io_object_t;
pub type io_connect_t = u32;
pub type mach_port_t = u32;
pub type kern_return_t = i32;
pub type IOOptionBits = u32;
pub type CFMutableDictionaryRef = *mut c_void;
pub type CFStringRef = *const c_void;
pub type CFTypeRef = *const c_void;
pub type IOSurfaceRef = *mut c_void;
pub type ObjcClass = *const c_void;
pub type ObjcSel = *const c_void;
pub type ObjcId = *mut c_void;

// ── Constants ──

pub const KERN_SUCCESS: kern_return_t = 0;
pub const kIOMasterPortDefault: mach_port_t = 0;
pub const kCFStringEncodingUTF8: u32 = 0x08000100;
pub const kCFNumberSInt32Type: i32 = 3;
pub const RTLD_NOW: i32 = 0x2;

pub const KIO_BAD_ARG: i32 = 0xe00002bc_u32 as i32;
pub const KIO_NOT_PRIV: i32 = 0xe00002c1_u32 as i32;
pub const KIO_UNSUPPORTED: i32 = 0xe00002d8_u32 as i32;
pub const KIO_EXCLUSIVE: i32 = 0xe00002be_u32 as i32;
pub const KIO_NOT_READY: i32 = 0xe00002c0_u32 as i32;
pub const KIO_NOT_PERMITTED: i32 = 0xe00002c2_u32 as i32;

// ── IOKit ──

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    pub fn IOServiceMatching(name: *const c_char) -> CFMutableDictionaryRef;
    pub fn IOServiceGetMatchingService(
        mainPort: mach_port_t,
        matching: CFMutableDictionaryRef,
    ) -> io_service_t;
    pub fn IOServiceOpen(
        service: io_service_t,
        owningTask: mach_port_t,
        type_: u32,
        connect: *mut io_connect_t,
    ) -> kern_return_t;
    pub fn IOServiceClose(connect: io_connect_t) -> kern_return_t;
    pub fn IOConnectCallScalarMethod(
        connection: io_connect_t,
        selector: u32,
        input: *const u64,
        inputCnt: u32,
        output: *mut u64,
        outputCnt: *mut u32,
    ) -> kern_return_t;
    pub fn IOConnectCallStructMethod(
        connection: io_connect_t,
        selector: u32,
        inputStruct: *const c_void,
        inputStructCnt: usize,
        outputStruct: *mut c_void,
        outputStructCnt: *mut usize,
    ) -> kern_return_t;
    pub fn IORegistryEntryCreateCFProperty(
        entry: io_object_t,
        key: CFStringRef,
        allocator: *const c_void,
        options: IOOptionBits,
    ) -> CFTypeRef;
    pub fn IORegistryEntryGetName(entry: io_object_t, name: *mut c_char) -> kern_return_t;
    pub fn IOObjectGetClass(object: io_object_t, class_name: *mut c_char) -> kern_return_t;
    pub fn IOObjectRelease(object: io_object_t) -> kern_return_t;
}

extern "C" {
    pub fn mach_task_self() -> mach_port_t;
}

// ── CoreFoundation ──

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub fn CFStringCreateWithCString(
        alloc: *const c_void,
        cStr: *const c_char,
        encoding: u32,
    ) -> CFStringRef;
    pub fn CFGetTypeID(cf: CFTypeRef) -> u64;
    pub fn CFStringGetTypeID() -> u64;
    pub fn CFStringGetCString(
        theString: CFStringRef,
        buffer: *mut c_char,
        bufferSize: i64,
        encoding: u32,
    ) -> bool;
    pub fn CFDictionaryGetTypeID() -> u64;
    pub fn CFDictionaryCreateMutable(
        allocator: *const c_void,
        capacity: i64,
        keyCallBacks: *const c_void,
        valueCallBacks: *const c_void,
    ) -> CFMutableDictionaryRef;
    pub fn CFDictionarySetValue(
        dict: CFMutableDictionaryRef,
        key: *const c_void,
        value: *const c_void,
    );
    pub fn CFDictionaryGetCount(dict: *const c_void) -> i64;
    pub fn CFNumberCreate(
        allocator: *const c_void,
        theType: i32,
        valuePtr: *const c_void,
    ) -> *const c_void;
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFCopyDescription(cf: *const c_void) -> CFStringRef;
    pub fn CFDataCreate(allocator: *const c_void, bytes: *const u8, length: i64) -> *const c_void;

    pub static kCFTypeDictionaryKeyCallBacks: c_void;
    pub static kCFTypeDictionaryValueCallBacks: c_void;
}

// ── IOSurface ──

#[link(name = "IOSurface", kind = "framework")]
extern "C" {
    pub fn IOSurfaceCreate(properties: CFMutableDictionaryRef) -> IOSurfaceRef;
    pub fn IOSurfaceLock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> kern_return_t;
    pub fn IOSurfaceUnlock(surface: IOSurfaceRef, options: u32, seed: *mut u32) -> kern_return_t;
    pub fn IOSurfaceGetBaseAddress(surface: IOSurfaceRef) -> *mut c_void;
    pub fn IOSurfaceGetAllocSize(surface: IOSurfaceRef) -> usize;
    pub fn IOSurfaceGetID(surface: IOSurfaceRef) -> u32;
}

// ── dlopen/dlsym ──

extern "C" {
    pub fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
    pub fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

// ── ObjC runtime ──

extern "C" {
    pub fn objc_getClass(name: *const c_char) -> ObjcClass;
    pub fn sel_registerName(name: *const c_char) -> ObjcSel;
    pub fn objc_msgSend() -> ObjcId;
}

// ── Helpers ──

pub fn cf_str(s: &str) -> CFStringRef {
    unsafe {
        let c = std::ffi::CString::new(s).unwrap();
        CFStringCreateWithCString(ptr::null(), c.as_ptr(), kCFStringEncodingUTF8)
    }
}

pub fn cf_num(v: i32) -> *const c_void {
    unsafe {
        CFNumberCreate(
            ptr::null(),
            kCFNumberSInt32Type,
            &v as *const i32 as *const c_void,
        )
    }
}

pub fn sel(name: &str) -> ObjcSel {
    let c = std::ffi::CString::new(name).unwrap();
    unsafe { sel_registerName(c.as_ptr()) }
}

pub fn cls(name: &str) -> ObjcClass {
    let c = std::ffi::CString::new(name).unwrap();
    unsafe { objc_getClass(c.as_ptr()) }
}

pub fn nserror_string(err: ObjcId) -> Option<String> {
    if err.is_null() {
        return None;
    }
    unsafe {
        type MsgSendId = unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId;
        let f: MsgSendId = std::mem::transmute(objc_msgSend as *const c_void);
        let desc = f(err, sel("description"));
        if desc.is_null() {
            return None;
        }
        type MsgSendUtf8 = unsafe extern "C" fn(ObjcId, ObjcSel) -> *const c_char;
        let u: MsgSendUtf8 = std::mem::transmute(objc_msgSend as *const c_void);
        let cstr = u(desc, sel("UTF8String"));
        if cstr.is_null() {
            return None;
        }
        Some(CStr::from_ptr(cstr).to_string_lossy().into_owned())
    }
}

pub fn kern_err(kr: kern_return_t) -> String {
    match kr {
        0 => "KERN_SUCCESS".into(),
        KIO_BAD_ARG => "kIOReturnBadArgument".into(),
        KIO_NOT_PRIV => "kIOReturnNotPrivileged".into(),
        KIO_UNSUPPORTED => "kIOReturnUnsupported".into(),
        KIO_EXCLUSIVE => "kIOReturnExclusiveAccess".into(),
        KIO_NOT_READY => "kIOReturnNotReady".into(),
        KIO_NOT_PERMITTED => "kIOReturnNotPermitted".into(),
        _ => format!("{:#010x}", kr as u32),
    }
}
