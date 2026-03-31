//! Model loading, ANE evaluation, and result verification.

use super::discovery::level4_make_surface;
use super::ffi::*;
use std::ffi::{c_char, c_void, CStr};
use std::ptr;

// ============================================================
// ObjC msg_send type aliases used during evaluation
// ============================================================

type MsgSendStr = unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId;
type MsgSendUtf8 = unsafe extern "C" fn(ObjcId, ObjcSel) -> *const c_char;
pub(crate) type MsgSendCompile =
    unsafe extern "C" fn(ObjcId, ObjcSel, u32, ObjcId, *mut ObjcId) -> bool;
type MsgSendIO = unsafe extern "C" fn(ObjcClass, ObjcSel, IOSurfaceRef) -> ObjcId;
type MsgSendArr = unsafe extern "C" fn(ObjcClass, ObjcSel, *const ObjcId, u64) -> ObjcId;
type MsgSendNum = unsafe extern "C" fn(ObjcClass, ObjcSel, i32) -> ObjcId;
type MsgSendReq = unsafe extern "C" fn(
    ObjcClass,
    ObjcSel,
    ObjcId,
    ObjcId,
    ObjcId,
    ObjcId,
    ObjcId,
    ObjcId,
    ObjcId,
) -> ObjcId;
type MsgSendEval = unsafe extern "C" fn(ObjcId, ObjcSel, u32, ObjcId, ObjcId, *mut ObjcId) -> bool;
type MsgSendUnload = unsafe extern "C" fn(ObjcId, ObjcSel, u32, *mut ObjcId) -> bool;

// ============================================================
// Load, evaluate, verify
// ============================================================

/// Load compiled model, evaluate on ANE, verify results.
pub(crate) unsafe fn load_eval_verify(
    model: ObjcId,
    empty_weights: ObjcId,
    compile_fn: MsgSendCompile,
    tmp_dir: &std::path::Path,
) {
    // Load: model.loadWithQoS:options:error:
    println!("\n  [*] Loading compiled model into ANE...");
    let mut load_err: ObjcId = ptr::null_mut();
    let load_ok = compile_fn(
        model,
        sel("loadWithQoS:options:error:"),
        21,
        empty_weights,
        &mut load_err,
    );

    if load_ok {
        println!("  [+] *** MODEL LOADED INTO ANE! ***");

        // Create IOSurfaces for I/O
        let ic: usize = 64;
        let oc: usize = 64;
        let seq: usize = 64;
        let sp: usize = seq + oc;
        let io_in = level4_make_surface(ic * sp * 2);
        let io_out = level4_make_surface(oc * seq * 2);

        if !io_in.is_null() && !io_out.is_null() {
            // Write test data: activations = 1.0, weights = identity-like
            IOSurfaceLock(io_in, 0, ptr::null_mut());
            let base_in = IOSurfaceGetBaseAddress(io_in) as *mut u16;
            for ch in 0..ic {
                for s in 0..seq {
                    *base_in.add(ch * sp + s) = 0x3C00; // fp16(1.0)
                }
                for o in 0..oc {
                    *base_in.add(ch * sp + seq + o) = if ch == o { 0x3C00 } else { 0 };
                }
            }
            IOSurfaceUnlock(io_in, 0, ptr::null_mut());
            println!(
                "  [+] Input: acts=[{},{}] all 1.0, weights=[{},{}] identity",
                ic, seq, ic, oc
            );

            evaluate_on_ane(model, empty_weights, io_in, io_out);

            // Unload
            let mut unload_err: ObjcId = ptr::null_mut();
            let unload_fn: MsgSendUnload = std::mem::transmute(objc_msgSend as *const c_void);
            let _ = unload_fn(model, sel("unloadWithQoS:error:"), 21, &mut unload_err);
            println!("\n  [+] Model unloaded");

            // Cleanup temp dir
            let _ = std::fs::remove_dir_all(tmp_dir);
        }
    } else {
        println!("  [-] Load failed");
        if !load_err.is_null() {
            println!("      Error: {}", cf_desc(load_err));
        }
    }
}

/// Create ANE request, evaluate, read and verify output.
unsafe fn evaluate_on_ane(
    model: ObjcId,
    empty_weights: ObjcId,
    io_in: IOSurfaceRef,
    io_out: IOSurfaceRef,
) {
    let cls_aio = cls("_ANEIOSurfaceObject");
    if cls_aio.is_null() {
        return;
    }

    let io_wrap: MsgSendIO = std::mem::transmute(objc_msgSend as *const c_void);
    let w_in = io_wrap(
        cls_aio as *const c_void as *mut c_void,
        sel("objectWithIOSurface:"),
        io_in,
    );
    let w_out = io_wrap(
        cls_aio as *const c_void as *mut c_void,
        sel("objectWithIOSurface:"),
        io_out,
    );

    // Create _ANERequest
    let cls_req = cls("_ANERequest");
    if cls_req.is_null() || w_in.is_null() || w_out.is_null() {
        return;
    }

    // Build NSArrays
    let cls_arr = cls("NSArray");
    let cls_num = cls("NSNumber");
    let arr_fn: MsgSendArr = std::mem::transmute(objc_msgSend as *const c_void);
    let inputs = arr_fn(
        cls_arr as *const _ as *mut _,
        sel("arrayWithObjects:count:"),
        &w_in as *const ObjcId,
        1,
    );
    let outputs = arr_fn(
        cls_arr as *const _ as *mut _,
        sel("arrayWithObjects:count:"),
        &w_out as *const ObjcId,
        1,
    );

    let num_fn: MsgSendNum = std::mem::transmute(objc_msgSend as *const c_void);
    let zero = num_fn(cls_num as *const _ as *mut _, sel("numberWithInt:"), 0);
    let indices = arr_fn(
        cls_arr as *const _ as *mut _,
        sel("arrayWithObjects:count:"),
        &zero as *const ObjcId,
        1,
    );

    // requestWithInputs:inputIndices:outputs:outputIndices:weightsBuffer:perfStats:procedureIndex:
    let req_fn: MsgSendReq = std::mem::transmute(objc_msgSend as *const c_void);
    let request = req_fn(
        cls_req as *const _ as *mut _,
        sel("requestWithInputs:inputIndices:outputs:outputIndices:weightsBuffer:perfStats:procedureIndex:"),
        inputs, indices, outputs, indices,
        ptr::null_mut(), ptr::null_mut(), zero,
    );

    if request.is_null() {
        println!("  [-] Failed to create _ANERequest");
        return;
    }
    println!("  [+] _ANERequest created");

    // Evaluate!
    println!("\n  [*] *** EVALUATING ON ANE HARDWARE ***");
    let mut eval_err: ObjcId = ptr::null_mut();
    let eval_fn: MsgSendEval = std::mem::transmute(objc_msgSend as *const c_void);
    let eval_ok = eval_fn(
        model,
        sel("evaluateWithQoS:options:request:error:"),
        21,
        empty_weights,
        request,
        &mut eval_err,
    );

    if eval_ok {
        println!("  [+] *** ANE EVALUATION SUCCEEDED! ***");
        println!("      Pure Rust → MIL → ANE bytecode → ANE hardware → result!");

        // Read output
        IOSurfaceLock(io_out, 0, ptr::null_mut());
        let base_out = IOSurfaceGetBaseAddress(io_out) as *const u16;
        let mut sample = [0u16; 8];
        for i in 0..8 {
            sample[i] = *base_out.add(i);
        }
        IOSurfaceUnlock(io_out, 0, ptr::null_mut());

        print!("  [+] Output[0..8] = [");
        for (i, &v) in sample.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            // Decode fp16 to f32 for display
            let sign = (v >> 15) & 1;
            let exp = (v >> 10) & 0x1F;
            let frac = v & 0x3FF;
            let f = if exp == 0 {
                ((-1.0f32).powi(sign as i32)) * (frac as f32 / 1024.0) * 2.0f32.powi(-14)
            } else if exp == 31 {
                if frac == 0 {
                    f32::INFINITY
                } else {
                    f32::NAN
                }
            } else {
                ((-1.0f32).powi(sign as i32))
                    * (1.0 + frac as f32 / 1024.0)
                    * 2.0f32.powi(exp as i32 - 15)
            };
            print!("{:.4}", f);
        }
        println!("]");

        // Verify identity: output should equal input (all 1.0)
        let all_ones = sample.iter().all(|&v| v == 0x3C00);
        if all_ones {
            println!("\n  ╔═══════════════════════════════════════════╗");
            println!("  ║  IDENTITY VERIFIED — ANE COMPUTED y = x   ║");
            println!("  ║  Pure Rust → ANE hardware. No ObjC.        ║");
            println!("  ╚═══════════════════════════════════════════╝");
        }
    } else {
        println!("  [-] Evaluation failed");
        if !eval_err.is_null() {
            let desc_sel = sel("description");
            let desc: ObjcId = std::mem::transmute((std::mem::transmute::<_, MsgSendStr>(
                objc_msgSend as *const c_void,
            ))(eval_err, desc_sel));
            if !desc.is_null() {
                let u: MsgSendUtf8 = std::mem::transmute(objc_msgSend as *const c_void);
                let s = u(desc, sel("UTF8String"));
                if !s.is_null() {
                    println!("      Error: {}", CStr::from_ptr(s).to_string_lossy());
                }
            }
        }
    }
}
