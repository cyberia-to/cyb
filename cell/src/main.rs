//! `cell` — drive a live cyb cell from the terminal.
//!
//!   cargo run
//!   cell› link cat dog
//!   cell› link dog food
//!   cell› query
//!   cell› state
//!   cell› quit

use cell::Cell;
use std::collections::HashMap;
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

fn main() {
    let mut cell = Cell::new();
    let neuron = [1u8; 32];
    let mut labels: HashMap<[u8; 32], String> = HashMap::new();

    println!("cell — a live cyb cell.  commands:  link <a> <b>  |  query [inf]  |  state  |  quit");
    let stdin = io::stdin();
    loop {
        print!("cell› ");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let line = line.trim();
        let mut it = line.splitn(2, ' ');
        match it.next().unwrap_or("") {
            "link" => {
                let mut ab = it.next().unwrap_or("").split_whitespace();
                match (ab.next(), ab.next()) {
                    (Some(a), Some(b)) => {
                        let (pa, pb) = (pid(a), pid(b));
                        labels.insert(pa, a.into());
                        labels.insert(pb, b.into());
                        match cell.link(neuron, pa, pb) {
                            Ok(sig) => println!("  linked {a} → {b}  (signal {})", hex4(&sig)),
                            Err(e) => println!("  error: {e:?}"),
                        }
                    }
                    _ => println!("  usage: link <a> <b>"),
                }
            }
            "query" => {
                let q = it.next().unwrap_or("?[cid, energy] := particles{cid, energy}");
                match cell.query(q) {
                    Ok(out) => {
                        println!("  {:?}", out.columns);
                        for row in &out.rows {
                            let named: Vec<String> = row
                                .iter()
                                .map(|v| format!("{v:?}"))
                                .collect();
                            println!("  {}", named.join("  "));
                        }
                    }
                    Err(e) => println!("  error: {e:?}"),
                }
            }
            "state" => println!("  {} particles", cell.graph.bbg.state.particles.len()),
            "quit" | "exit" | "q" => break,
            "" => {}
            other => println!("  unknown: {other}   (link | query | state | quit)"),
        }
    }
}
