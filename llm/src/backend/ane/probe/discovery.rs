//! Levels 1-4: IOKit service discovery, user client, selector probing, IOSurface creation.

use super::ffi::*;
use std::ffi::{c_void, CStr};
use std::ptr;

// ============================================================
// Level 1: IOKit Service Discovery
// ============================================================

pub(crate) fn level1_discover() -> Option<io_service_t> {
    println!("═══════════════════════════════════════════════════");
    println!("  LEVEL 1: IOKit Service Discovery");
    println!("═══════════════════════════════════════════════════\n");

    let candidates = [
        "H11ANEIn",
        "H11ANEInDirectPathClient",
        "AppleH11ANEInterface",
        "AppleANELoadBalancer",
    ];

    let mut found_service: Option<io_service_t> = None;

    for name in &candidates {
        let c_name = std::ffi::CString::new(*name).unwrap();
        unsafe {
            let matching = IOServiceMatching(c_name.as_ptr());
            if matching.is_null() {
                continue;
            }

            let service = IOServiceGetMatchingService(kIOMasterPortDefault, matching);
            if service == 0 {
                println!("  [-] {} — not found", name);
            } else {
                let mut class_name = [0i8; 128];
                IOObjectGetClass(service, class_name.as_mut_ptr());
                let class_str = CStr::from_ptr(class_name.as_ptr()).to_string_lossy();

                let mut entry_name = [0i8; 128];
                IORegistryEntryGetName(service, entry_name.as_mut_ptr());
                let entry_str = CStr::from_ptr(entry_name.as_ptr()).to_string_lossy();

                println!(
                    "  [+] {} — FOUND (id={:#x}, class='{}', name='{}')",
                    name, service, class_str, entry_str
                );

                // Read DeviceProperties
                let key = cf_str("DeviceProperties");
                let val = IORegistryEntryCreateCFProperty(service, key, ptr::null(), 0);
                if !val.is_null() {
                    let type_id = CFGetTypeID(val);
                    let dict_id = CFDictionaryGetTypeID();
                    if type_id == dict_id {
                        println!("      DeviceProperties = <CFDictionary>");
                    }
                    CFRelease(val);
                }

                if found_service.is_none() {
                    found_service = Some(service);
                } else {
                    IOObjectRelease(service);
                }
            }
        }
    }

    found_service
}

// ============================================================
// Level 2: Open User Client
// ============================================================

pub(crate) fn level2_open(service: io_service_t) -> Option<io_connect_t> {
    println!("\n═══════════════════════════════════════════════════");
    println!("  LEVEL 2: Open User Client Connection");
    println!("═══════════════════════════════════════════════════\n");

    for uc_type in 0..=10 {
        unsafe {
            let mut connect: io_connect_t = 0;
            let kr = IOServiceOpen(service, mach_task_self(), uc_type, &mut connect);
            if kr == KERN_SUCCESS {
                println!(
                    "  [+] IOServiceOpen(type={}) => connection={:#x}  *** SUCCESS ***",
                    uc_type, connect
                );
                return Some(connect);
            } else {
                println!("  [-] IOServiceOpen(type={}) => {}", uc_type, kern_err(kr));
            }
        }
    }
    None
}

// ============================================================
// Level 3: Probe Selectors
// ============================================================

pub(crate) fn level3_probe(connect: io_connect_t) {
    println!("\n═══════════════════════════════════════════════════");
    println!("  LEVEL 3: Probe IOKit Selectors");
    println!("═══════════════════════════════════════════════════\n");

    for selector in 0..64 {
        unsafe {
            // Try scalar (no input)
            let mut output = [0u64; 16];
            let mut output_cnt: u32 = 16;
            let kr_scalar = IOConnectCallScalarMethod(
                connect,
                selector,
                ptr::null(),
                0,
                output.as_mut_ptr(),
                &mut output_cnt,
            );

            // Try struct (no input)
            let mut out_buf = [0u8; 4096];
            let mut out_size: usize = 4096;
            let kr_struct = IOConnectCallStructMethod(
                connect,
                selector,
                ptr::null(),
                0,
                out_buf.as_mut_ptr() as *mut c_void,
                &mut out_size,
            );

            // Only print if something interesting happened
            const MIG_BAD_ID: i32 = 0xfffffbc4_u32 as i32;
            let scalar_interesting = kr_scalar != KIO_BAD_ARG && kr_scalar != MIG_BAD_ID;
            let struct_interesting = kr_struct != KIO_BAD_ARG && kr_struct != MIG_BAD_ID;

            if scalar_interesting || struct_interesting {
                print!("  sel {:2}:", selector);
                if scalar_interesting {
                    if kr_scalar == KERN_SUCCESS {
                        print!(
                            " scalar=OK({}:{:?})",
                            output_cnt,
                            &output[..output_cnt as usize]
                        );
                    } else {
                        print!(" scalar={}", kern_err(kr_scalar));
                    }
                }
                if struct_interesting {
                    if kr_struct == KERN_SUCCESS {
                        print!(" struct=OK({} bytes)", out_size);
                        if out_size > 0 && out_size <= 64 {
                            print!(" data={:02x?}", &out_buf[..out_size]);
                        }
                    } else {
                        print!(" struct={}", kern_err(kr_struct));
                    }
                }
                println!();
            }
        }
    }
}

// ============================================================
// Level 4: IOSurface creation
// ============================================================

pub(crate) fn level4_surface() -> Option<IOSurfaceRef> {
    println!("\n═══════════════════════════════════════════════════");
    println!("  LEVEL 4: IOSurface (ANE tensor buffer)");
    println!("═══════════════════════════════════════════════════\n");

    unsafe {
        let dict = CFDictionaryCreateMutable(
            ptr::null(),
            0,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        );

        // ANE native format: [1, C, 1, S] where C=channels, S=spatial
        // Map to IOSurface: width=S, height=1, bytesPerElement=C*2 (fp16)
        let channels: i32 = 64;
        let spatial: i32 = 64;
        let bpe: i32 = channels * 2;

        CFDictionarySetValue(dict, cf_str("IOSurfaceWidth") as _, cf_num(spatial));
        CFDictionarySetValue(dict, cf_str("IOSurfaceHeight") as _, cf_num(1));
        CFDictionarySetValue(dict, cf_str("IOSurfaceBytesPerElement") as _, cf_num(bpe));
        CFDictionarySetValue(dict, cf_str("IOSurfacePixelFormat") as _, cf_num(0));

        let surface = IOSurfaceCreate(dict);
        if surface.is_null() {
            println!("  [-] IOSurfaceCreate FAILED");
            return None;
        }

        let alloc = IOSurfaceGetAllocSize(surface);
        let sid = IOSurfaceGetID(surface);
        println!("  [+] IOSurface created from pure Rust!");
        println!("      ID     = {}", sid);
        println!(
            "      Size   = {} bytes ({:.1} KB)",
            alloc,
            alloc as f64 / 1024.0
        );
        println!("      Shape  = [1, {}, 1, {}] fp16", channels, spatial);

        // Write recognizable pattern
        IOSurfaceLock(surface, 0, ptr::null_mut());
        let base = IOSurfaceGetBaseAddress(surface) as *mut u16;
        if !base.is_null() {
            for i in 0..(channels * spatial) as usize {
                *base.add(i) = 0x3C00; // fp16(1.0)
            }
            println!("      Wrote {} fp16 values = 1.0 each", channels * spatial);
        }
        IOSurfaceUnlock(surface, 0, ptr::null_mut());

        Some(surface)
    }
}

/// Helper: create IOSurface of given byte size
pub(crate) fn level4_make_surface(bytes: usize) -> IOSurfaceRef {
    unsafe {
        let dict = CFDictionaryCreateMutable(
            ptr::null(),
            0,
            &kCFTypeDictionaryKeyCallBacks as *const c_void,
            &kCFTypeDictionaryValueCallBacks as *const c_void,
        );
        CFDictionarySetValue(dict, cf_str("IOSurfaceWidth") as _, cf_num(bytes as i32));
        CFDictionarySetValue(dict, cf_str("IOSurfaceHeight") as _, cf_num(1));
        CFDictionarySetValue(dict, cf_str("IOSurfaceBytesPerElement") as _, cf_num(1));
        CFDictionarySetValue(
            dict,
            cf_str("IOSurfaceBytesPerRow") as _,
            cf_num(bytes as i32),
        );
        CFDictionarySetValue(
            dict,
            cf_str("IOSurfaceAllocSize") as _,
            cf_num(bytes as i32),
        );
        CFDictionarySetValue(dict, cf_str("IOSurfacePixelFormat") as _, cf_num(0));
        IOSurfaceCreate(dict)
    }
}
