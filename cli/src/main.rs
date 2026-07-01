//! cyb-cli — the terminal face of a cyb.
//!
//!   cyb-cli                    the `cyb›` REPL (interactive)
//!   cyb-cli link cat dog       one-shot
//!   cyb-cli query              one-shot query
//!
//! Both modes drive a `cyb_core::Cell`. The REPL is just the interactive mode —
//! not a separate thing.

use cyb_core::Cell;
use std::io::{self, BufRead, Write};

/// A readable particle from a label: its bytes, padded to 32.
fn pid(label: &str) -> [u8; 32] {
    let mut p = [0u8; 32];
    let b = label.as_bytes();
    let n = b.len().min(32);
    p[..n].copy_from_slice(&b[..n]);
    p
}

fn hex4(p: &[u8; 32]) -> String {
    p[..4].iter().map(|b| format!("{b:02x}")).collect()
}

/// Run one command line against the cell. Returns false to stop the REPL.
fn exec(cell: &mut Cell, neuron: [u8; 32], line: &str) -> bool {
    let mut it = line.trim().splitn(2, ' ');
    match it.next().unwrap_or("") {
        "link" => {
            let mut ab = it.next().unwrap_or("").split_whitespace();
            match (ab.next(), ab.next()) {
                (Some(a), Some(b)) => match cell.link(neuron, pid(a), pid(b)) {
                    Ok(sig) => println!("  linked {a} → {b}  (signal {})", hex4(&sig)),
                    Err(e) => println!("  error: {e:?}"),
                },
                _ => println!("  usage: link <a> <b>"),
            }
        }
        "query" => {
            let q = it.next().unwrap_or("?[particle, energy] := particles{particle, energy}");
            match cell.query(q) {
                Ok(out) => {
                    println!("  {:?}", out.columns);
                    for row in &out.rows {
                        println!("  {row:?}");
                    }
                }
                Err(e) => println!("  error: {e:?}"),
            }
        }
        "state" => println!("  {} particles", cell.particles()),
        "quit" | "exit" | "q" => return false,
        "" => {}
        other => println!("  unknown: {other}   (link | query | state | quit)"),
    }
    true
}

fn main() {
    let mut cell = Cell::new();
    let neuron = [1u8; 32];

    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        // one-shot mode (stateless: a fresh cell per invocation, for now)
        exec(&mut cell, neuron, &args.join(" "));
        return;
    }

    // interactive mode — the REPL
    println!("cyb — a live cell.  commands:  link <a> <b>  |  query [inf]  |  state  |  quit");
    let stdin = io::stdin();
    loop {
        print!("cyb› ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        if !exec(&mut cell, neuron, &line) {
            break;
        }
    }
}
