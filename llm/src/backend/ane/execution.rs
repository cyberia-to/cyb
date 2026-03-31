//! AneModel: compile MIL, load into ANE, run on hardware, unload

use crate::ffi::*;
use crate::mil::MilProgram;
use crate::surface::AneSurface;
use crate::AneError;
use std::ffi::{c_char, c_void, CStr};
use std::path::PathBuf;
use std::ptr;

/// A compiled ANE model that can be loaded and evaluated on hardware.
pub struct AneModel {
    objc_model: ObjcId,
    empty_dict: ObjcId,
    loaded: bool,
    tmp_dir: PathBuf,
}

impl AneModel {
    /// Compile a MIL program into ANE bytecode.
    ///
    /// `weights` is a slice of `(path, fp16_data)` pairs where path follows
    /// the `@model_path/weights/filename.bin` convention.
    /// For models without external weights (e.g. dynamic matmul), pass `&[]`.
    pub fn compile(program: &MilProgram, weights: &[(&str, &[u8])]) -> Result<Self, AneError> {
        // Ensure AppleNeuralEngine.framework is loaded
        load_ane_frameworks();

        // SAFETY: All FFI calls below go through objc_msgSend transmuted to typed
        // function pointers matching the private ObjC method signatures. Null checks
        // validate every return value before use. Frameworks are loaded via dlopen
        // before any ObjC call (load_ane_frameworks above).
        unsafe {
            let cls_descriptor = cls("_ANEInMemoryModelDescriptor");
            if cls_descriptor.is_null() {
                return Err(AneError::ClassNotFound("_ANEInMemoryModelDescriptor"));
            }
            let cls_model_class = cls("_ANEInMemoryModel");
            if cls_model_class.is_null() {
                return Err(AneError::ClassNotFound("_ANEInMemoryModel"));
            }

            // Create NSData from MIL text
            let mil_bytes = program.as_str().as_bytes();
            let mil_data = msg_send_2::<*const u8, u64>(
                cls("NSData") as ObjcId,
                "dataWithBytes:length:",
                mil_bytes.as_ptr(),
                mil_bytes.len() as u64,
            );
            if mil_data.is_null() {
                return Err(AneError::CompilationFailed(
                    "Failed to create NSData".into(),
                ));
            }

            // Build weights NSDictionary
            let weights_dict = build_weights_dict(weights);

            // Create empty dict for options
            let empty_dict: ObjcId = msg_send_0(cls("NSDictionary") as ObjcId, "dictionary");

            // _ANEInMemoryModelDescriptor.modelWithMILText:weights:optionsPlist:
            let descriptor = msg_send_3::<ObjcId, ObjcId, ObjcId>(
                cls_descriptor as ObjcId,
                "modelWithMILText:weights:optionsPlist:",
                mil_data,
                weights_dict,
                ptr::null_mut(),
            );
            if descriptor.is_null() {
                return Err(AneError::DescriptorCreationFailed);
            }

            // _ANEInMemoryModel.inMemoryModelWithDescriptor:
            let model = msg_send_1::<ObjcId>(
                cls_model_class as ObjcId,
                "inMemoryModelWithDescriptor:",
                descriptor,
            );
            if model.is_null() {
                return Err(AneError::ModelCreationFailed);
            }

            // Get hex ID and set up temp directory
            let hex_str = objc_nsstring_to_rust(msg_send_0(model, "hexStringIdentifier"))
                .unwrap_or_else(|| "unknown".to_string());
            let tmp_dir = std::env::temp_dir().join(&hex_str);
            let _ = std::fs::create_dir_all(tmp_dir.join("weights"));
            std::fs::write(tmp_dir.join("model.mil"), program.as_str())
                .map_err(|e| AneError::CompilationFailed(format!("Failed to write MIL: {}", e)))?;

            // Write weight blobs to temp dir
            for &(path, data) in weights {
                let rel = path.replace("@model_path/", "");
                let full = tmp_dir.join(&rel);
                // Prevent path traversal: resolved path must stay inside tmp_dir
                if !full.starts_with(&tmp_dir) {
                    return Err(AneError::CompilationFailed(format!(
                        "Path traversal in weight path: {}",
                        rel
                    )));
                }
                if let Some(parent) = full.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(&full, data).map_err(|e| {
                    AneError::CompilationFailed(format!("Failed to write weight {}: {}", rel, e))
                })?;
            }

            // compileWithQoS:options:error:
            let mut error: ObjcId = ptr::null_mut();
            let ok = msg_send_compile(
                model,
                "compileWithQoS:options:error:",
                21,
                empty_dict,
                &mut error,
            );
            if !ok {
                let msg = nserror_string(error).unwrap_or_else(|| "unknown error".into());
                return Err(AneError::CompilationFailed(msg));
            }

            Ok(AneModel {
                objc_model: model,
                empty_dict,
                loaded: false,
                tmp_dir,
            })
        }
    }

    /// Load the compiled model into ANE hardware.
    pub fn load(&mut self) -> Result<(), AneError> {
        // SAFETY: objc_msgSend transmuted to match loadWithQoS:options:error: signature.
        // self.objc_model is valid (created in compile). Error pointer checked on failure.
        unsafe {
            let mut error: ObjcId = ptr::null_mut();
            let ok = msg_send_compile(
                self.objc_model,
                "loadWithQoS:options:error:",
                21,
                self.empty_dict,
                &mut error,
            );
            if !ok {
                let msg = nserror_string(error).unwrap_or_else(|| "unknown error".into());
                return Err(AneError::LoadFailed(msg));
            }
            self.loaded = true;
            Ok(())
        }
    }

    /// Run the model on ANE hardware.
    /// Input and output must be `AneSurface` with correct sizes for the MIL program.
    pub fn run(&self, input: &AneSurface, output: &AneSurface) -> Result<(), AneError> {
        if !self.loaded {
            return Err(AneError::EvalFailed("Model not loaded".into()));
        }
        // SAFETY: objc_msgSend transmuted to match evaluateWithQoS:options:request:error:.
        // Model is loaded (checked above). IOSurfaces wrapped in _ANEIOSurfaceObject
        // and _ANERequest before dispatch. Error checked on failure.
        unsafe {
            let request = build_request(input.as_raw(), output.as_raw())?;

            // evaluateWithQoS:options:request:error:
            let mut error: ObjcId = ptr::null_mut();
            type EvalFn =
                unsafe extern "C" fn(ObjcId, ObjcSel, u32, ObjcId, ObjcId, *mut ObjcId) -> bool;
            let eval: EvalFn = std::mem::transmute(objc_msgSend as *const c_void);
            let ok = eval(
                self.objc_model,
                sel("evaluateWithQoS:options:request:error:"),
                21,
                self.empty_dict,
                request,
                &mut error,
            );
            if !ok {
                let msg = nserror_string(error).unwrap_or_else(|| "unknown error".into());
                return Err(AneError::EvalFailed(msg));
            }
            Ok(())
        }
    }

    /// Unload the model from ANE hardware.
    pub fn unload(&mut self) -> Result<(), AneError> {
        if !self.loaded {
            return Ok(());
        }
        unsafe {
            let mut error: ObjcId = ptr::null_mut();
            type UnloadFn = unsafe extern "C" fn(ObjcId, ObjcSel, u32, *mut ObjcId) -> bool;
            let f: UnloadFn = std::mem::transmute(objc_msgSend as *const c_void);
            let ok = f(self.objc_model, sel("unloadWithQoS:error:"), 21, &mut error);
            self.loaded = false;
            if !ok {
                let msg = nserror_string(error).unwrap_or_else(|| "unknown error".into());
                return Err(AneError::UnloadFailed(msg));
            }
            Ok(())
        }
    }
}

impl Drop for AneModel {
    fn drop(&mut self) {
        let _ = self.unload();
        let _ = std::fs::remove_dir_all(&self.tmp_dir);
    }
}

// ── Internal helpers ──

fn load_ane_frameworks() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        for name in &["AppleNeuralEngine", "ANECompiler", "ANEServices"] {
            let path = format!(
                "/System/Library/PrivateFrameworks/{}.framework/{}",
                name, name
            );
            let c = std::ffi::CString::new(path).unwrap();
            unsafe {
                dlopen(c.as_ptr(), RTLD_NOW);
            }
        }
    });
}

fn objc_nsstring_to_rust(obj: ObjcId) -> Option<String> {
    if obj.is_null() {
        return None;
    }
    unsafe {
        type F = unsafe extern "C" fn(ObjcId, ObjcSel) -> *const c_char;
        let f: F = std::mem::transmute(objc_msgSend as *const c_void);
        let cstr = f(obj, sel("UTF8String"));
        if cstr.is_null() {
            return None;
        }
        Some(CStr::from_ptr(cstr).to_string_lossy().into_owned())
    }
}

unsafe fn msg_send_0(target: ObjcId, selector: &str) -> ObjcId {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel) -> ObjcId;
    let f: F = std::mem::transmute(objc_msgSend as *const c_void);
    f(target, sel(selector))
}

unsafe fn msg_send_1<A>(target: ObjcId, selector: &str, a: A) -> ObjcId {
    type F<A> = unsafe extern "C" fn(ObjcId, ObjcSel, A) -> ObjcId;
    let f: F<A> = std::mem::transmute(objc_msgSend as *const c_void);
    f(target, sel(selector), a)
}

unsafe fn msg_send_2<A, B>(target: ObjcId, selector: &str, a: A, b: B) -> ObjcId {
    type F<A, B> = unsafe extern "C" fn(ObjcId, ObjcSel, A, B) -> ObjcId;
    let f: F<A, B> = std::mem::transmute(objc_msgSend as *const c_void);
    f(target, sel(selector), a, b)
}

unsafe fn msg_send_3<A, B, C>(target: ObjcId, selector: &str, a: A, b: B, c: C) -> ObjcId {
    type F<A, B, C> = unsafe extern "C" fn(ObjcId, ObjcSel, A, B, C) -> ObjcId;
    let f: F<A, B, C> = std::mem::transmute(objc_msgSend as *const c_void);
    f(target, sel(selector), a, b, c)
}

unsafe fn msg_send_compile(
    target: ObjcId,
    selector: &str,
    qos: u32,
    opts: ObjcId,
    err: *mut ObjcId,
) -> bool {
    type F = unsafe extern "C" fn(ObjcId, ObjcSel, u32, ObjcId, *mut ObjcId) -> bool;
    let f: F = std::mem::transmute(objc_msgSend as *const c_void);
    f(target, sel(selector), qos, opts, err)
}

fn build_weights_dict(weights: &[(&str, &[u8])]) -> ObjcId {
    unsafe {
        if weights.is_empty() {
            return msg_send_0(cls("NSDictionary") as ObjcId, "dictionary");
        }
        // For each weight entry, create nested dict: { "offset": 0, "data": NSData }
        let cls_nsdict = cls("NSMutableDictionary");
        let dict: ObjcId = msg_send_0(cls_nsdict as ObjcId, "dictionary");
        for &(path, data) in weights {
            let key = nsstring(path);
            let inner: ObjcId = msg_send_0(cls_nsdict as ObjcId, "dictionary");
            let offset_key = nsstring("offset");
            let data_key = nsstring("data");
            let zero: ObjcId = msg_send_1(cls("NSNumber") as ObjcId, "numberWithInt:", 0i32);
            let nsdata = msg_send_2::<*const u8, u64>(
                cls("NSData") as ObjcId,
                "dataWithBytes:length:",
                data.as_ptr(),
                data.len() as u64,
            );
            type SetF = unsafe extern "C" fn(ObjcId, ObjcSel, ObjcId, ObjcId);
            let set: SetF = std::mem::transmute(objc_msgSend as *const c_void);
            set(inner, sel("setObject:forKey:"), zero, offset_key);
            set(inner, sel("setObject:forKey:"), nsdata, data_key);
            set(dict, sel("setObject:forKey:"), inner, key);
        }
        dict
    }
}

fn nsstring(s: &str) -> ObjcId {
    unsafe {
        let cstr = std::ffi::CString::new(s).unwrap();
        type F = unsafe extern "C" fn(ObjcId, ObjcSel, *const c_char) -> ObjcId;
        let f: F = std::mem::transmute(objc_msgSend as *const c_void);
        f(
            cls("NSString") as ObjcId,
            sel("stringWithUTF8String:"),
            cstr.as_ptr(),
        )
    }
}

unsafe fn build_request(io_in: IOSurfaceRef, io_out: IOSurfaceRef) -> Result<ObjcId, AneError> {
    let cls_aio = cls("_ANEIOSurfaceObject");
    if cls_aio.is_null() {
        return Err(AneError::ClassNotFound("_ANEIOSurfaceObject"));
    }
    let cls_req = cls("_ANERequest");
    if cls_req.is_null() {
        return Err(AneError::ClassNotFound("_ANERequest"));
    }

    // Wrap IOSurfaces
    let w_in = msg_send_1::<IOSurfaceRef>(cls_aio as ObjcId, "objectWithIOSurface:", io_in);
    let w_out = msg_send_1::<IOSurfaceRef>(cls_aio as ObjcId, "objectWithIOSurface:", io_out);
    if w_in.is_null() || w_out.is_null() {
        return Err(AneError::EvalFailed("Failed to wrap IOSurface".into()));
    }

    // Build NSArrays
    let cls_arr = cls("NSArray");
    type ArrFn = unsafe extern "C" fn(ObjcId, ObjcSel, *const ObjcId, u64) -> ObjcId;
    let arr: ArrFn = std::mem::transmute(objc_msgSend as *const c_void);
    let inputs = arr(cls_arr as ObjcId, sel("arrayWithObjects:count:"), &w_in, 1);
    let outputs = arr(cls_arr as ObjcId, sel("arrayWithObjects:count:"), &w_out, 1);

    let zero = msg_send_1::<i32>(cls("NSNumber") as ObjcId, "numberWithInt:", 0);
    let indices = arr(cls_arr as ObjcId, sel("arrayWithObjects:count:"), &zero, 1);

    // requestWithInputs:inputIndices:outputs:outputIndices:weightsBuffer:perfStats:procedureIndex:
    type ReqFn = unsafe extern "C" fn(
        ObjcId,
        ObjcSel,
        ObjcId,
        ObjcId,
        ObjcId,
        ObjcId,
        ObjcId,
        ObjcId,
        ObjcId,
    ) -> ObjcId;
    let req_fn: ReqFn = std::mem::transmute(objc_msgSend as *const c_void);
    let request = req_fn(
        cls_req as ObjcId,
        sel("requestWithInputs:inputIndices:outputs:outputIndices:weightsBuffer:perfStats:procedureIndex:"),
        inputs, indices, outputs, indices,
        ptr::null_mut(), ptr::null_mut(), zero,
    );
    if request.is_null() {
        return Err(AneError::EvalFailed("Failed to create _ANERequest".into()));
    }
    Ok(request)
}
