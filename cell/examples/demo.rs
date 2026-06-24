//! A cell, live. Build it, link into it, read it back.
//!
//!   cargo run --example demo

use cell::Cell;

fn h(p: &[u8; 32]) -> String {
    p[..4].iter().map(|b| format!("{:02x}", b)).collect()
}

fn main() {
    let mut cell = Cell::new();
    let alice = [0x01; 32];
    let (aa, bb, cc) = ([0xAA; 32], [0xBB; 32], [0xCC; 32]);

    println!("● fresh cell — particles: {}", cell.graph.bbg.state.particles.len());

    let s1 = cell.link(alice, aa, bb).expect("link aa→bb");
    let s2 = cell.link(alice, bb, cc).expect("link bb→cc");
    println!("● alice links  {}→{}  (signal {})", h(&aa), h(&bb), h(&s1));
    println!("● alice links  {}→{}  (signal {})", h(&bb), h(&cc), h(&s2));

    println!(
        "● state now — particles: {}   has {}? {}   has {}? {}",
        cell.graph.bbg.state.particles.len(),
        h(&bb),
        cell.has_particle(&bb),
        h(&cc),
        cell.has_particle(&cc),
    );

    let out = cell
        .query("?[cid, energy] := particles{cid, energy}")
        .expect("query runs");
    println!("● query  ?[cid, energy] := particles{{cid, energy}}");
    println!("  {out:?}");
}
