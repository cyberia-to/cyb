//! Embedding lookup kernel

use cubecl::prelude::*;
use cubecl::wgpu::WgpuRuntime;
use crate::runtime::tensor::{Client, GpuTensor};

/// Embedding lookup: output[i] = table[token_ids[i]]
/// table: [vocab, hidden], token_ids: [seq] (as f32), output: [seq, hidden]
#[cube(launch_unchecked)]
fn embed_kernel(
    table: &Tensor<Line<f32>>,
    ids: &Tensor<Line<f32>>,
    output: &mut Tensor<Line<f32>>,
    #[comptime] hidden: u32,
) {
    let pos = ABSOLUTE_POS;
    if pos >= output.len() / hidden as usize { terminate!(); }

    let token_id = u32::cast_from(f32::cast_from(ids[pos])) as usize;
    let src_base = token_id * hidden as usize;
    let dst_base = pos * hidden as usize;

    let mut i: u32 = 0;
    while i < hidden {
        output[dst_base + i as usize] = table[src_base + i as usize];
        i += 1;
    }
}

pub fn embed(client: &Client, table: &GpuTensor, ids: &GpuTensor, hidden: usize) -> GpuTensor {
    let seq = ids.numel();
    let output = GpuTensor::empty_f32(client, vec![seq, hidden]);
    let cube_dim = CubeDim::new(client, seq);
    let cube_count = cubecl::calculate_cube_count_elemwise(client, seq, cube_dim);
    unsafe {
        embed_kernel::launch_unchecked::<WgpuRuntime>(
            client, cube_count, cube_dim,
            table.as_arg(), ids.as_arg(), output.as_arg(),
            hidden as u32,
        ).expect("embed failed");
    }
    output
}
