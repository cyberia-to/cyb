//! Levels 5-6: framework loading, symbol enumeration, architecture discovery, XPC detection.

use super::ffi::*;
use std::ffi::{c_char, c_void};

// ============================================================
// ANEServices function type signatures (reverse-engineered)
// ============================================================

type ANEServicesDeviceOpenFn =
    unsafe extern "C" fn(device_id: u32, flags: u32, out_handle: *mut *mut c_void) -> i32;
type ANEServicesDeviceCloseFn = unsafe extern "C" fn(handle: *mut c_void) -> i32;
type ANEServicesProgramCreateFn = unsafe extern "C" fn(
    device: *mut c_void,
    program_data: *const c_void,
    program_size: usize,
    options: *const c_void, // CFDictionary
    out_program: *mut *mut c_void,
) -> i32;

// ANECCompile signature guess — likely takes CFDictionary of options
type ANECCompileFn = unsafe extern "C" fn(
    input: *const c_void, // CFDictionary with model + options
    output: *mut c_void,  // output buffer/dict
    flags: u32,
) -> i32;

// ============================================================
// AneFrameworks struct
// ============================================================

pub(crate) struct AneFrameworks {
    pub compiler: *mut c_void,
    pub services: *mut c_void,
    pub engine: *mut c_void,
}

// ============================================================
// Helpers
// ============================================================

fn load_framework(name: &str) -> *mut c_void {
    let path = format!(
        "/System/Library/PrivateFrameworks/{}.framework/{}",
        name, name
    );
    let c = std::ffi::CString::new(path.as_str()).unwrap();
    let h = unsafe { dlopen(c.as_ptr(), RTLD_NOW) };
    if h.is_null() {
        println!("  [-] {} — failed to load", name);
    } else {
        println!("  [+] {} — loaded @ {:?}", name, h);
    }
    h
}

pub(crate) fn find_sym(name: &str) -> *mut c_void {
    let c = std::ffi::CString::new(name).unwrap();
    let rtld_default = (-2isize) as *mut c_void;
    unsafe { dlsym(rtld_default, c.as_ptr()) }
}

// ============================================================
// Level 5: Load ANE Frameworks (C API)
// ============================================================

pub(crate) fn level5_frameworks() -> Option<AneFrameworks> {
    println!("\n═══════════════════════════════════════════════════");
    println!("  LEVEL 5: Load ANE Frameworks (C API)");
    println!("═══════════════════════════════════════════════════\n");

    let engine = load_framework("AppleNeuralEngine");
    let compiler = load_framework("ANECompiler");
    let services = load_framework("ANEServices");

    // Now scan for key C functions across all loaded images
    println!("\n  Key C functions (no ObjC needed):");
    let key_symbols = [
        // Compiler
        "ANECCompile",
        "ANECCompileJIT",
        "ANECCompileOnline",
        "ANECCompileOffline",
        "ANECCreateCompilerInputDictionary",
        "ANECCreateCompilerOptionDictionary",
        "ANECCreateModelDictionary",
        "ANECCreateDeviceProperty",
        "ANECGetDeviceProperty",
        "ANECValidate",
        "ANECGetCompilerFileFormat",
        // Services
        "ANEServicesDeviceOpen",
        "ANEServicesDeviceClose",
        "ANEServicesProgramCreate",
        "ANEServicesProgramPrepare",
        "ANEServicesProgramInputsReady",
        "ANEServicesProgramDestroy",
        "ANEServicesProgramStop",
        "ANEServicesProgramProcessRequestDirect",
        "ANEServicesProgramOutputSetEnqueue",
        "ANEServicesProgramMemoryMapRequest",
        "ANEServicesProgramMemoryUnmapRequest",
        "ANEServicesProgramChainingPrepare",
        "ANEServicesInitializePlatformServices",
        // Validation
        "ANEValidateMILNetworkOnHost",
        "ANEValidateNetworkCreate",
        "ANEGetValidateNetworkSupportedVersion",
    ];

    for sym in &key_symbols {
        let p = find_sym(sym);
        if !p.is_null() {
            println!("  [+] {:50} @ {:?}", sym, p);
        }
    }

    if compiler.is_null() && services.is_null() && engine.is_null() {
        return None;
    }

    Some(AneFrameworks {
        compiler,
        services,
        engine,
    })
}

// ============================================================
// Level 6: Architecture Discovery + XPC detection
// ============================================================

pub(crate) fn level6_direct_api() {
    println!("\n═══════════════════════════════════════════════════");
    println!("  LEVEL 6: Architecture Discovery");
    println!("═══════════════════════════════════════════════════\n");

    println!("  Communication path discovered:");
    println!("    Your code");
    println!("      → AppleNeuralEngine.framework (_ANEInMemoryModel etc.)");
    println!("      → XPC to 'com.apple.appleneuralengine' (Mach service)");
    println!("      → aned daemon (/usr/libexec/aned)");
    println!("      → IOKit H11ANEIn (entitlement: com.apple.ane.iokit-user-access)");
    println!("      → ANE hardware\n");

    println!("  IOKit selectors return kIOReturnNotPermitted because");
    println!("  our process lacks com.apple.ane.iokit-user-access entitlement.");
    println!("  Only aned has this. All user code goes through XPC.\n");

    // _ANEDaemonInterface — returns NSXPCInterface describing the protocol
    let daemon_iface = find_sym("_ANEDaemonInterface");
    if !daemon_iface.is_null() {
        println!("  [+] _ANEDaemonInterface @ {:?}", daemon_iface);
        println!("      This returns the NSXPCInterface for talking to aned.");
        println!("      The ObjC classes wrap this, but we could call it directly.");
    }

    let daemon_iface_private = find_sym("_ANEDaemonInterfacePrivate");
    if !daemon_iface_private.is_null() {
        println!(
            "  [+] _ANEDaemonInterfacePrivate @ {:?}",
            daemon_iface_private
        );
    }

    // Check if we can connect to the XPC service from Rust
    // Using bootstrap_look_up to find the mach port
    println!("\n  [*] Checking XPC service reachability...");
    unsafe {
        extern "C" {
            fn bootstrap_look_up(
                bp: mach_port_t,
                service_name: *const c_char,
                sp: *mut mach_port_t,
            ) -> kern_return_t;
            fn bootstrap_port() -> mach_port_t;

            // We can access bootstrap_port via task_get_special_port
            fn task_get_special_port(
                task: mach_port_t,
                which_port: i32,
                special_port: *mut mach_port_t,
            ) -> kern_return_t;
        }

        const TASK_BOOTSTRAP_PORT: i32 = 4;
        let mut bp: mach_port_t = 0;
        let kr = task_get_special_port(mach_task_self(), TASK_BOOTSTRAP_PORT, &mut bp);
        if kr == KERN_SUCCESS {
            println!("  [+] Got bootstrap port: {:#x}", bp);

            let svc_name = std::ffi::CString::new("com.apple.appleneuralengine").unwrap();
            let mut svc_port: mach_port_t = 0;
            let kr2 = bootstrap_look_up(bp, svc_name.as_ptr(), &mut svc_port);
            if kr2 == KERN_SUCCESS {
                println!(
                    "  [+] com.apple.appleneuralengine => port {:#x} *** REACHABLE ***",
                    svc_port
                );
                println!("      This is the Mach port to aned. XPC talks here.");
            } else {
                println!("  [-] bootstrap_look_up failed: {}", kr2);
            }
        }
    }
}
