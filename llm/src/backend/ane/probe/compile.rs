//! Level 7: ObjC runtime helpers, MIL generation, model compilation.

use super::eval::{load_eval_verify, MsgSendCompile};
use super::ffi::*;
use std::ffi::{c_char, c_void, CStr};
use std::ptr;

// ============================================================
// Weight blob builder
// ============================================================

/// Build ANE weight blob: 128-byte header + fp16 data
/// Header format from training_dynamic/io.h build_blob():
///   byte 0: 1, byte 4: 2, bytes 64-67: 0xDEADBEEF, byte 68: 1
///   bytes 72-75: weight_size_bytes, bytes 80-83: 128 (data offset)
pub(crate) fn build_weight_blob(fp16_data: &[u16]) -> Vec<u8> {
    let weight_bytes = fp16_data.len() * 2;
    let total = 128 + weight_bytes;
    let mut blob = vec![0u8; total];

    blob[0] = 1;
    blob[4] = 2;
    // DEADBEEF magic
    blob[64] = 0xEF;
    blob[65] = 0xBE;
    blob[66] = 0xAD;
    blob[67] = 0xDE;
    blob[68] = 1;
    // Weight size in bytes
    blob[72..76].copy_from_slice(&(weight_bytes as u32).to_le_bytes());
    // Data offset
    blob[80..84].copy_from_slice(&128u32.to_le_bytes());
    // Copy fp16 data
    for (i, &val) in fp16_data.iter().enumerate() {
        let off = 128 + i * 2;
        blob[off..off + 2].copy_from_slice(&val.to_le_bytes());
    }
    blob
}

// ============================================================
// ObjC runtime msg_send type aliases (used during compilation)
// ============================================================

type MsgSendInit = unsafe extern "C" fn(ObjcClass, ObjcSel) -> ObjcId;
type MsgSendDataInit = unsafe extern "C" fn(ObjcClass, ObjcSel, *const u8, u64) -> ObjcId;
type MsgSendDesc = unsafe extern "C" fn(ObjcClass, ObjcSel, ObjcId, ObjcId, ObjcId) -> ObjcId;
type MsgSendModel = unsafe extern "C" fn(ObjcClass, ObjcSel, ObjcId) -> ObjcId;
type MsgSendStr = unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId;
type MsgSendUtf8 = unsafe extern "C" fn(ObjcId, ObjcSel) -> *const c_char;

// ============================================================
// Level 7: Compile MIL -> ANE bytecode
// ============================================================

pub(crate) fn level7_mil_compile() {
    println!("\n═══════════════════════════════════════════════════");
    println!("  LEVEL 7: MIL → ANE Bytecode (Pure Rust)");
    println!("═══════════════════════════════════════════════════\n");

    // ── Step 1: Minimal MIL program ──
    // Dynamic matmul: y = x @ W (exact format from training_dynamic/mil_dynamic.h)
    // Input: [1, 64, 1, 128] — first 64 cols = activations, last 64 = weights
    // Output: [1, 64, 1, 64] — matmul result
    let ic = 64;
    let oc = 64;
    let seq = 64;
    let sp = seq + oc; // 128

    let mil = format!(
        concat!(
            "program(1.3)\n",
            "[buildInfo = dict<string, string>(",
            "{{{{\"coremlc-component-MIL\", \"3510.2.1\"}}, ",
            "{{\"coremlc-version\", \"3505.4.1\"}}, ",
            "{{\"coremltools-component-milinternal\", \"\"}}, ",
            "{{\"coremltools-version\", \"9.0\"}}}}",
            ")]\n{{\n",
            "    func main<ios18>(tensor<fp16, [1, {ic}, 1, {sp}]> x) {{\n",
            "        tensor<int32, [4]> ba = const()[name=string(\"ba\"), val=tensor<int32, [4]>([0,0,0,0])];\n",
            "        tensor<int32, [4]> sa = const()[name=string(\"sa\"), val=tensor<int32, [4]>([1,{ic},1,{seq}])];\n",
            "        tensor<fp16, [1,{ic},1,{seq}]> act = slice_by_size(x=x,begin=ba,size=sa)[name=string(\"act\")];\n",
            "        tensor<int32, [4]> bw = const()[name=string(\"bw\"), val=tensor<int32, [4]>([0,0,0,{seq}])];\n",
            "        tensor<int32, [4]> sw = const()[name=string(\"sw\"), val=tensor<int32, [4]>([1,{ic},1,{oc}])];\n",
            "        tensor<fp16, [1,{ic},1,{oc}]> wt = slice_by_size(x=x,begin=bw,size=sw)[name=string(\"wt\")];\n",
            "        tensor<int32, [4]> ra = const()[name=string(\"ra\"), val=tensor<int32, [4]>([1,1,{ic},{seq}])];\n",
            "        tensor<fp16, [1,1,{ic},{seq}]> a2 = reshape(shape=ra,x=act)[name=string(\"a2\")];\n",
            "        tensor<int32, [4]> pm = const()[name=string(\"pm\"), val=tensor<int32, [4]>([0,1,3,2])];\n",
            "        tensor<fp16, [1,1,{seq},{ic}]> a3 = transpose(perm=pm,x=a2)[name=string(\"a3\")];\n",
            "        tensor<int32, [4]> rw = const()[name=string(\"rw\"), val=tensor<int32, [4]>([1,1,{ic},{oc}])];\n",
            "        tensor<fp16, [1,1,{ic},{oc}]> W = reshape(shape=rw,x=wt)[name=string(\"W\")];\n",
            "        bool bF = const()[name=string(\"bF\"), val=bool(false)];\n",
            "        tensor<fp16, [1,1,{seq},{oc}]> yh = matmul(transpose_x=bF,transpose_y=bF,x=a3,y=W)[name=string(\"yh\")];\n",
            "        tensor<fp16, [1,1,{oc},{seq}]> yt = transpose(perm=pm,x=yh)[name=string(\"yt\")];\n",
            "        tensor<int32, [4]> ro = const()[name=string(\"ro\"), val=tensor<int32, [4]>([1,{oc},1,{seq}])];\n",
            "        tensor<fp16, [1,{oc},1,{seq}]> y = reshape(shape=ro,x=yt)[name=string(\"y\")];\n",
            "    }} -> (y);\n",
            "}}\n"
        ),
        ic=ic, oc=oc, seq=seq, sp=sp
    );
    println!("  MIL program ({} bytes):", mil.len());
    for line in mil.lines() {
        if !line.is_empty() {
            println!("    {}", line);
        }
    }

    // ── Step 2: Try ANECCompile C API first ──
    println!("\n  ── Approach A: ANECCompile (pure C) ──\n");
    let approach_a_ok = try_anec_compile(&mil);

    if !approach_a_ok {
        // ── Step 3: ObjC runtime path ──
        println!("\n  ── Approach B: ObjC runtime from Rust ──\n");
        try_objc_compile(&mil);
    }
}

fn try_anec_compile(_mil: &str) -> bool {
    // ANECCreateCompilerInputDictionary/OptionDictionary hang when called
    // with no args — they likely require arguments we don't know yet.
    // Skip pure C API for now, go straight to ObjC runtime path.
    println!("  [~] Skipping ANECCompile C API (helper functions require");
    println!("      unknown arguments). Using ObjC runtime instead.");
    false
}

fn try_objc_compile(mil: &str) -> bool {
    println!("  Using ObjC runtime from Rust (objc_msgSend — same as training code)");
    println!("  No ObjC compiler needed — just C FFI to libobjc\n");

    unsafe {
        // Get ObjC classes
        let cls_descriptor = cls("_ANEInMemoryModelDescriptor");
        let cls_model = cls("_ANEInMemoryModel");

        if cls_descriptor.is_null() {
            println!("  [-] _ANEInMemoryModelDescriptor class not found");
            println!("      AppleNeuralEngine.framework might not export it");
            return false;
        }
        println!("  [+] _ANEInMemoryModelDescriptor @ {:?}", cls_descriptor);

        if cls_model.is_null() {
            println!("  [-] _ANEInMemoryModel class not found");
            return false;
        }
        println!("  [+] _ANEInMemoryModel @ {:?}", cls_model);

        // Create NSData from MIL text
        let cls_nsdata = cls("NSData");
        let mil_bytes = mil.as_bytes();
        let data_init: MsgSendDataInit = std::mem::transmute(objc_msgSend as *const c_void);
        let mil_data = data_init(
            cls_nsdata as *const c_void as *mut c_void,
            sel("dataWithBytes:length:"),
            mil_bytes.as_ptr(),
            mil_bytes.len() as u64,
        );
        if mil_data.is_null() {
            println!("  [-] Failed to create NSData from MIL text");
            return false;
        }
        println!("  [+] NSData(MIL) created ({} bytes)", mil_bytes.len());

        // Create empty NSDictionary for weights (identity has no weights)
        let cls_nsdict = cls("NSDictionary");
        let dict_init: MsgSendInit = std::mem::transmute(objc_msgSend as *const c_void);
        let empty_weights = dict_init(
            cls_nsdict as *const c_void as *mut c_void,
            sel("dictionary"),
        );
        println!("  [+] Empty weights dict created");

        // _ANEInMemoryModelDescriptor.modelWithMILText:weights:optionsPlist:
        println!("\n  [*] Creating model descriptor...");
        let create_desc: MsgSendDesc = std::mem::transmute(objc_msgSend as *const c_void);
        let descriptor = create_desc(
            cls_descriptor as *const c_void as *mut c_void,
            sel("modelWithMILText:weights:optionsPlist:"),
            mil_data,
            empty_weights,
            ptr::null_mut(), // nil options
        );
        if descriptor.is_null() {
            println!("  [-] modelWithMILText:weights:optionsPlist: returned nil");
            return false;
        }
        println!("  [+] Descriptor created!");

        // _ANEInMemoryModel.inMemoryModelWithDescriptor:
        println!("  [*] Creating in-memory model...");
        let create_model: MsgSendModel = std::mem::transmute(objc_msgSend as *const c_void);
        let model = create_model(
            cls_model as *const c_void as *mut c_void,
            sel("inMemoryModelWithDescriptor:"),
            descriptor,
        );
        if model.is_null() {
            println!("  [-] inMemoryModelWithDescriptor: returned nil");
            return false;
        }
        println!("  [+] Model created!");

        // Get hex identifier for temp dir
        let get_str: MsgSendStr = std::mem::transmute(objc_msgSend as *const c_void);
        let hex_id = get_str(model, sel("hexStringIdentifier"));
        let hex_str = if !hex_id.is_null() {
            let utf8_fn: MsgSendUtf8 = std::mem::transmute(objc_msgSend as *const c_void);
            let cstr = utf8_fn(hex_id, sel("UTF8String"));
            if !cstr.is_null() {
                CStr::from_ptr(cstr).to_string_lossy().into_owned()
            } else {
                "unknown".to_string()
            }
        } else {
            "unknown".to_string()
        };
        println!("  [+] Model hex ID: {}", hex_str);

        // Set up temp directory (model.mil + weights/)
        let tmp_dir = std::env::temp_dir().join(&hex_str);
        let _ = std::fs::create_dir_all(tmp_dir.join("weights"));
        std::fs::write(tmp_dir.join("model.mil"), mil).unwrap();
        println!("  [+] Temp dir: {:?}", tmp_dir);

        // Compile: model.compileWithQoS:options:error:
        println!("\n  [*] *** COMPILING MIL → ANE BYTECODE ***");
        let mut error: ObjcId = ptr::null_mut();
        let compile_fn: MsgSendCompile = std::mem::transmute(objc_msgSend as *const c_void);
        let ok = compile_fn(
            model,
            sel("compileWithQoS:options:error:"),
            21,            // QoS
            empty_weights, // options (empty dict)
            &mut error,
        );

        if ok {
            println!("  [+] *** COMPILATION SUCCEEDED! ***");
            println!("      MIL → ANE bytecode compiled from pure Rust!");

            // Check what files were produced
            if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                    println!("      {:?}: {} bytes", name, size);
                    // Recurse one level for subdirs
                    if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        if let Ok(sub) = std::fs::read_dir(entry.path()) {
                            for se in sub.flatten() {
                                let sname = se.file_name();
                                let ssize = se.metadata().map(|m| m.len()).unwrap_or(0);
                                println!("        {:?}: {} bytes", sname, ssize);
                            }
                        }
                    }
                }
            }

            // Load + evaluate + verify
            load_eval_verify(model, empty_weights, compile_fn, &tmp_dir);

            true
        } else {
            println!("  [-] Compilation failed");
            if !error.is_null() {
                let desc_fn: MsgSendStr = std::mem::transmute(objc_msgSend as *const c_void);
                let desc = desc_fn(error, sel("description"));
                if !desc.is_null() {
                    let utf8_fn: MsgSendUtf8 = std::mem::transmute(objc_msgSend as *const c_void);
                    let cstr = utf8_fn(desc, sel("UTF8String"));
                    if !cstr.is_null() {
                        println!("      Error: {}", CStr::from_ptr(cstr).to_string_lossy());
                    }
                }
            }
            false
        }
    }
}
