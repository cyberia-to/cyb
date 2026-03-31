//! IOSurface wrapper for ANE tensor I/O

#![allow(dead_code)]

use crate::ffi::*;
use crate::AneError;
use std::ffi::c_void;
use std::ptr;

/// A shared-memory tensor buffer backed by IOSurface.
/// Used to pass fp16 data to/from the Apple Neural Engine.
pub struct AneSurface {
    raw: IOSurfaceRef,
    size: usize,
}

impl AneSurface {
    /// Maximum surface size: 256 MB (ANE practical limit).
    const MAX_SURFACE_BYTES: usize = 256 * 1024 * 1024;

    /// Create an IOSurface of the given byte size.
    pub fn new(bytes: usize) -> Result<Self, AneError> {
        if bytes == 0 || bytes > Self::MAX_SURFACE_BYTES {
            return Err(AneError::SurfaceCreationFailed(format!(
                "{} bytes (must be 1..={})",
                bytes,
                Self::MAX_SURFACE_BYTES
            )));
        }
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
            let raw = IOSurfaceCreate(dict);
            if raw.is_null() {
                return Err(AneError::SurfaceCreationFailed(format!("{} bytes", bytes)));
            }
            let size = IOSurfaceGetAllocSize(raw);
            Ok(AneSurface { raw, size })
        }
    }

    /// Create with ANE tensor shape `[1, channels, 1, spatial]` in fp16.
    pub fn with_shape(channels: usize, spatial: usize) -> Result<Self, AneError> {
        Self::new(channels * spatial * 2)
    }

    /// Lock surface, call closure with mutable fp16 slice, unlock.
    pub fn with_data_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut [u16]) -> R,
    {
        unsafe {
            IOSurfaceLock(self.raw, 0, ptr::null_mut());
            let base = IOSurfaceGetBaseAddress(self.raw) as *mut u16;
            let len = self.size / 2;
            let slice = std::slice::from_raw_parts_mut(base, len);
            let result = f(slice);
            IOSurfaceUnlock(self.raw, 0, ptr::null_mut());
            result
        }
    }

    /// Lock surface (read-only), call closure with fp16 slice, unlock.
    pub fn with_data<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&[u16]) -> R,
    {
        unsafe {
            IOSurfaceLock(self.raw, 1, ptr::null_mut()); // kIOSurfaceLockReadOnly = 1
            let base = IOSurfaceGetBaseAddress(self.raw) as *const u16;
            let len = self.size / 2;
            let slice = std::slice::from_raw_parts(base, len);
            let result = f(slice);
            IOSurfaceUnlock(self.raw, 1, ptr::null_mut());
            result
        }
    }

    /// Get the raw IOSurfaceRef for passing to ObjC wrappers.
    pub(crate) fn as_raw(&self) -> IOSurfaceRef {
        self.raw
    }

    /// IOSurface ID.
    pub fn id(&self) -> u32 {
        unsafe { IOSurfaceGetID(self.raw) }
    }

    /// Allocation size in bytes.
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for AneSurface {
    fn drop(&mut self) {
        unsafe {
            CFRelease(self.raw as CFTypeRef);
        }
    }
}

/// Decode fp16 to f32 — NEON fcvt on aarch64
#[inline(always)]
pub fn fp16_to_f32(v: u16) -> f32 {
    #[cfg(target_arch = "aarch64")]
    {
        let result: u32;
        unsafe {
            std::arch::asm!(
                "fmov h0, {src:w}",
                "fcvt s0, h0",
                "fmov {dst:w}, s0",
                src = in(reg) v as u32,
                dst = out(reg) result,
                out("v0") _,
            );
        }
        f32::from_bits(result)
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        fp16_to_f32_soft(v)
    }
}

/// Encode f32 to fp16 — NEON fcvt on aarch64
#[inline(always)]
pub fn f32_to_fp16(v: f32) -> u16 {
    #[cfg(target_arch = "aarch64")]
    {
        let result: u32;
        unsafe {
            std::arch::asm!(
                "fmov s0, {src:w}",
                "fcvt h0, s0",
                "fmov {dst:w}, s0",
                src = in(reg) v.to_bits(),
                dst = out(reg) result,
                out("v0") _,
            );
        }
        result as u16
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        f32_to_fp16_soft(v)
    }
}

/// Bulk convert fp16 → f32 using inline NEON assembly (8 at a time)
pub fn cvt_f16_f32(dst: &mut [f32], src: &[u16]) {
    let n = dst.len().min(src.len());
    #[cfg(target_arch = "aarch64")]
    {
        let mut i = 0;
        while i + 8 <= n {
            unsafe {
                std::arch::asm!(
                    "ldr q0, [{src}]",          // load 8 × fp16
                    "fcvtl v1.4s, v0.4h",       // convert low 4 to f32
                    "fcvtl2 v2.4s, v0.8h",      // convert high 4 to f32
                    "stp q1, q2, [{dst}]",       // store 8 × f32
                    src = in(reg) src.as_ptr().add(i),
                    dst = in(reg) dst.as_mut_ptr().add(i),
                    out("v0") _, out("v1") _, out("v2") _,
                );
            }
            i += 8;
        }
        for j in i..n {
            dst[j] = fp16_to_f32(src[j]);
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        for i in 0..n {
            dst[i] = fp16_to_f32_soft(src[i]);
        }
    }
}

/// Bulk convert f32 → fp16 using inline NEON assembly (8 at a time)
pub fn cvt_f32_f16(dst: &mut [u16], src: &[f32]) {
    let n = dst.len().min(src.len());
    #[cfg(target_arch = "aarch64")]
    {
        let mut i = 0;
        while i + 8 <= n {
            unsafe {
                std::arch::asm!(
                    "ldp q0, q1, [{src}]",       // load 8 × f32
                    "fcvtn v2.4h, v0.4s",        // convert low 4 to fp16
                    "fcvtn2 v2.8h, v1.4s",       // convert high 4 to fp16
                    "str q2, [{dst}]",            // store 8 × fp16
                    src = in(reg) src.as_ptr().add(i),
                    dst = in(reg) dst.as_mut_ptr().add(i),
                    out("v0") _, out("v1") _, out("v2") _,
                );
            }
            i += 8;
        }
        for j in i..n {
            dst[j] = f32_to_fp16(src[j]);
        }
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        for i in 0..n {
            dst[i] = f32_to_fp16_soft(src[i]);
        }
    }
}

fn fp16_to_f32_soft(v: u16) -> f32 {
    let sign = (v >> 15) & 1;
    let exp = (v >> 10) & 0x1F;
    let frac = v & 0x3FF;
    if exp == 0 {
        ((-1.0f32).powi(sign as i32)) * (frac as f32 / 1024.0) * 2.0f32.powi(-14)
    } else if exp == 31 {
        if frac == 0 {
            f32::INFINITY
        } else {
            f32::NAN
        }
    } else {
        ((-1.0f32).powi(sign as i32)) * (1.0 + frac as f32 / 1024.0) * 2.0f32.powi(exp as i32 - 15)
    }
}

fn f32_to_fp16_soft(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = (bits >> 31) & 1;
    let f32_exp = (bits >> 23) & 0xFF;
    let frac = bits & 0x7FFFFF;
    // NaN / Inf
    if f32_exp == 0xFF {
        if frac != 0 {
            return ((sign << 15) | 0x7C00 | (frac >> 13).max(1)) as u16; // NaN
        }
        return ((sign << 15) | 0x7C00) as u16; // Inf
    }
    let exp = f32_exp as i32 - 127;
    if exp > 15 {
        ((sign << 15) | 0x7C00) as u16 // overflow → Inf
    } else if exp < -14 {
        if exp < -24 {
            (sign << 15) as u16 // flush to zero
        } else {
            // subnormal: shift = (1 - (-14)) + (23 - 10) = 14 + exp + 13 - exp...
            // mantissa with implicit 1 bit, shifted right by (14 + exp) positions less
            let shift = (-14 - exp + 13) as u32; // = -1 - exp (from -14 base, 10-bit mantissa)
            ((sign << 15) | ((0x800000 | frac) >> shift)) as u16
        }
    } else {
        ((sign << 15) | (((exp + 15) as u32) << 10) | (frac >> 13)) as u16
    }
}
