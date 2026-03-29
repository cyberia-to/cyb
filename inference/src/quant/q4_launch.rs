//! Launch Q4 vecmat kernel through cubecl — zero-copy integration with burn

use std::collections::HashMap;
use std::sync::Mutex;

use cubecl::prelude::*;
use cubecl::wgpu::WgpuRuntime;
use cubecl::Runtime;
use cubecl::frontend::TensorHandleRef;

use crate::graph::value::Value;

type BurnDevice = <crate::Backend as burn::tensor::backend::Backend>::Device;

/// Cached Q4 weight handles on GPU (cubecl handles)
struct Q4CachedWeight {
    packed_handle: cubecl::server::Handle,
    scales_handle: cubecl::server::Handle,
    packed_len: usize,
    scales_len: usize,
    n: usize,
    k: usize,
    block_size: usize,
}

static Q4_WEIGHT_CACHE: std::sync::LazyLock<Mutex<HashMap<String, Q4CachedWeight>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Execute Q4 vecmat via cubecl kernel on burn's GPU device
pub fn q4_vecmat_cubecl(
    cache_key: &str,
    activation: &Value,
    packed_weights_f32: &Value,
    scales_val: &Value,
    n: usize,
    k: usize,
    block_size: usize,
    device: &BurnDevice,
) -> Value {
    let client = WgpuRuntime::client(device);

    // Cache weight handles on GPU
    let mut cache = Q4_WEIGHT_CACHE.lock().unwrap();
    if !cache.contains_key(cache_key) {
        let packed_f32 = packed_weights_f32.to_vec_f32();
        let packed_bytes: Vec<u8> = packed_f32.iter().map(|&v| v as u8).collect();

        // Pack into u32
        let mut packed_u32: Vec<u32> = Vec::with_capacity((packed_bytes.len() + 3) / 4);
        for chunk in packed_bytes.chunks(4) {
            let mut val: u32 = 0;
            for (i, &b) in chunk.iter().enumerate() {
                val |= (b as u32) << (i * 8);
            }
            packed_u32.push(val);
        }

        let packed_handle = client.create_from_slice(bytemuck::cast_slice(&packed_u32));
        let scales_data = scales_val.to_vec_f32();
        let scales_handle = client.create_from_slice(bytemuck::cast_slice(&scales_data));

        cache.insert(cache_key.to_string(), Q4CachedWeight {
            packed_len: packed_u32.len(),
            scales_len: scales_data.len(),
            packed_handle,
            scales_handle,
            n, k, block_size,
        });
    }
    let cached = cache.get(cache_key).unwrap();

    // Activation: GPU → CPU → cubecl handle (TODO: zero-copy from burn tensor)
    let act_data = activation.to_vec_f32();
    let act_handle = client.create_from_slice(bytemuck::cast_slice(&act_data));

    // Output buffer
    let output_handle = client.empty(n * std::mem::size_of::<f32>());

    // Param buffers (K and block_size as u32 tensors)
    let k_handle = client.create_from_slice(bytemuck::cast_slice(&[k as u32]));
    let bs_handle = client.create_from_slice(bytemuck::cast_slice(&[block_size as u32]));

    // Launch kernel
    let act_shape = [k];
    let act_strides = [1usize];
    let packed_shape = [cached.packed_len];
    let packed_strides = [1usize];
    let scales_shape = [cached.scales_len];
    let scales_strides = [1usize];
    let output_shape = [n];
    let output_strides = [1usize];
    let param_shape = [1usize];
    let param_strides = [1usize];

    let cube_dim = CubeDim::new(&client, n);
    let ph = std::marker::PhantomData::<WgpuRuntime>;

    unsafe {
        super::q4_kernel::q4_vecmat::launch_unchecked::<WgpuRuntime>(
            &client,
            CubeCount::Static(n as u32, 1, 1),
            cube_dim,
            TensorArg::Handle {
                handle: TensorHandleRef { handle: &act_handle, strides: &act_strides, shape: &act_shape, elem_size: 4, runtime: ph },
                line_size: 1,
            },
            TensorArg::Handle {
                handle: TensorHandleRef { handle: &cached.packed_handle, strides: &packed_strides, shape: &packed_shape, elem_size: 4, runtime: ph },
                line_size: 1,
            },
            TensorArg::Handle {
                handle: TensorHandleRef { handle: &cached.scales_handle, strides: &scales_strides, shape: &scales_shape, elem_size: 4, runtime: ph },
                line_size: 1,
            },
            TensorArg::Handle {
                handle: TensorHandleRef { handle: &output_handle, strides: &output_strides, shape: &output_shape, elem_size: 4, runtime: ph },
                line_size: 1,
            },
            TensorArg::Handle {
                handle: TensorHandleRef { handle: &k_handle, strides: &param_strides, shape: &param_shape, elem_size: 4, runtime: ph },
                line_size: 1,
            },
            TensorArg::Handle {
                handle: TensorHandleRef { handle: &bs_handle, strides: &param_strides, shape: &param_shape, elem_size: 4, runtime: ph },
                line_size: 1,
            },
        ).expect("Q4 kernel launch failed");
    }

    // Read output
    let output_bytes = client.read_one(output_handle);
    let output_f32: Vec<f32> = bytemuck::cast_slice(&output_bytes).to_vec();

    let mut out_shape = activation.shape();
    *out_shape.last_mut().unwrap() = n;
    Value::from_data(output_f32, out_shape, device)
}
