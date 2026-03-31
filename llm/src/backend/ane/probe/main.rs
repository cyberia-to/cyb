//! ANE Probe — reverse engineering Apple Neural Engine from pure Rust
//!
//! Level 1: IOKit service discovery — find the ANE driver
//! Level 2: Open user client — establish connection
//! Level 3: Probe selectors — discover the IOKit command interface
//! Level 4: IOSurface creation — prepare data buffers
//! Level 5: Attempt communication

#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    dead_code,
    clippy::too_many_arguments,
    clippy::unnecessary_cast,
    clippy::missing_transmute_annotations
)]

mod compile;
mod discovery;
mod eval;
mod ffi;
mod frameworks;

use ffi::*;

fn main() {
    println!("╔═══════════════════════════════════════════════════╗");
    println!("║   ANE PROBE — Pure Rust → Apple Neural Engine    ║");
    println!("║   Target: M1 Pro / H11ANEIn / ane,t8020          ║");
    println!("╚═══════════════════════════════════════════════════╝\n");

    // Level 1: Find the ANE driver in IORegistry
    let service = discovery::level1_discover();

    // Level 2: Try to open user client
    let connect = if let Some(svc) = service {
        let conn = discovery::level2_open(svc);
        unsafe {
            IOObjectRelease(svc);
        }
        conn
    } else {
        println!("\n  [!] No ANE service found in IORegistry");
        None
    };

    // Level 3: Probe selectors if we got a connection
    if let Some(conn) = connect {
        discovery::level3_probe(conn);
    } else {
        println!("\n  [!] No user client — will still probe IOSurface and symbols");
    }

    // Level 4: IOSurface — works regardless of ANE access
    let _surface = discovery::level4_surface();

    // Level 5: Load frameworks and enumerate C API
    let _fw = frameworks::level5_frameworks();

    // Level 6: Try ANEServices direct C calls
    frameworks::level6_direct_api();

    // Level 7: Compile MIL → ANE bytecode
    compile::level7_mil_compile();

    // Cleanup
    if let Some(conn) = connect {
        unsafe {
            IOServiceClose(conn);
        }
    }

    println!("\n═══════════════════════════════════════════════════");
    println!("  PROBE COMPLETE");
    println!("═══════════════════════════════════════════════════");
}
