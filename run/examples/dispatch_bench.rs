//! Measure raw Metal dispatch + wait overhead with a no-op kernel.
//! Helps decide: are we limited by dispatch latency or kernel compute?

const NOOP_MSL: &str = r#"
#include <metal_stdlib>
using namespace metal;
kernel void kmain(uint gid [[thread_position_in_grid]]) { /* nothing */ }
"#;

fn main() {
    let gpu = aruminium::Gpu::open().expect("open");
    let queue = gpu.new_command_queue().expect("queue");
    let dispatch = aruminium::Dispatch::new(&queue);
    let lib = gpu.compile(NOOP_MSL).expect("compile");
    let f = lib.function("kmain").expect("function");
    let pipe = gpu.pipeline(&f).expect("pipeline");

    // Warmup
    for _ in 0..10 {
        unsafe {
            aruminium::autorelease_pool(|| {
                dispatch.batch_raw(|enc| {
                    enc.bind(&pipe);
                    enc.launch_groups((1, 1, 1), (1, 1, 1));
                });
            });
        }
    }

    let runs = 1000;

    // Test 1: 1 dispatch per batch, 1 wait per batch
    let t = std::time::Instant::now();
    for _ in 0..runs {
        unsafe {
            aruminium::autorelease_pool(|| {
                dispatch.batch_raw(|enc| {
                    enc.bind(&pipe);
                    enc.launch_groups((1, 1, 1), (1, 1, 1));
                });
            });
        }
    }
    let dt = t.elapsed();
    println!("1 dispatch / batch:  {} runs took {:?} ({:.2}us per dispatch+wait)",
             runs, dt, dt.as_micros() as f64 / runs as f64);

    // Test 2: 5 dispatches per batch, 1 wait per batch
    let t = std::time::Instant::now();
    for _ in 0..runs {
        unsafe {
            aruminium::autorelease_pool(|| {
                dispatch.batch_raw(|enc| {
                    for _ in 0..5 {
                        enc.bind(&pipe);
                        enc.launch_groups((1, 1, 1), (1, 1, 1));
                    }
                });
            });
        }
    }
    let dt = t.elapsed();
    println!("5 dispatches / batch: {} runs took {:?} ({:.2}us per batch, {:.2}us per dispatch)",
             runs, dt, dt.as_micros() as f64 / runs as f64,
             dt.as_micros() as f64 / (runs as f64 * 5.0));

    // Test 3: 100 dispatches per batch
    let t = std::time::Instant::now();
    for _ in 0..(runs / 10) {
        unsafe {
            aruminium::autorelease_pool(|| {
                dispatch.batch_raw(|enc| {
                    for _ in 0..100 {
                        enc.bind(&pipe);
                        enc.launch_groups((1, 1, 1), (1, 1, 1));
                    }
                });
            });
        }
    }
    let dt = t.elapsed();
    println!("100 dispatches / batch: {} runs took {:?} ({:.2}us per batch, {:.2}us per dispatch)",
             runs / 10, dt, dt.as_micros() as f64 / (runs as f64 / 10.0),
             dt.as_micros() as f64 / (runs as f64 / 10.0 * 100.0));
}
