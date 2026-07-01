//! cyb-cli — the terminal face of a cyb.
//!
//!   cyb-cli                    the `cyb›` REPL (interactive)
//!   cyb-cli link cat dog       one-shot
//!   cyb-cli query              one-shot query
//!
//! Both modes drive a `cyb_core::Cell`. The REPL is just the interactive mode.

use cyb_core::Cell;
use inf_value::Value;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

/// A readable particle from a label: its bytes, padded to 32.
fn pid(label: &str) -> [u8; 32] {
    let mut p = [0u8; 32];
    let b = label.as_bytes();
    let n = b.len().min(32);
    p[..n].copy_from_slice(&b[..n]);
    p
}

/// Render a particle id: the label if it's printable-ascii-padded, else a
/// short hash prefix.
fn particle(h: &[u8; 32]) -> String {
    let end = h.iter().position(|&b| b == 0).unwrap_or(32);
    if end > 0 && h[..end].iter().all(u8::is_ascii_graphic) && h[end..].iter().all(|&b| b == 0) {
        String::from_utf8_lossy(&h[..end]).into_owned()
    } else {
        let hex: String = h[..3].iter().map(|b| format!("{b:02x}")).collect();
        format!("{hex}…")
    }
}

/// Render one query cell.
fn cell_str(v: &Value) -> String {
    match v {
        Value::Hash(h) => particle(h),
        Value::Int(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Word(w) => w.to_string(),
        Value::Bytes(b) => String::from_utf8_lossy(b).into_owned(),
        Value::Null => "·".into(),
        other => format!("{other:?}"),
    }
}

const HELP: &str = "  link <a> <b>   assert a cyberlink a → b
  query [inf]    read the graph (default: every particle + energy)
  state          how many particles the cell holds
  help           this
  quit";

/// Run one command line against the cell. Returns false to stop the REPL.
fn exec(cell: &mut Cell, neuron: [u8; 32], line: &str) -> bool {
    let mut it = line.trim().splitn(2, ' ');
    match it.next().unwrap_or("") {
        "link" => {
            let mut ab = it.next().unwrap_or("").split_whitespace();
            match (ab.next(), ab.next()) {
                (Some(a), Some(b)) => match cell.link(neuron, pid(a), pid(b)) {
                    Ok(sig) => {
                        let s: String = sig[..3].iter().map(|b| format!("{b:02x}")).collect();
                        println!("  linked {a} → {b}   (signal {s}…)");
                    }
                    Err(e) => println!("  error: {e:?}"),
                },
                _ => println!("  usage: link <a> <b>"),
            }
        }
        "query" => {
            let q = it.next().unwrap_or("?[particle, energy] := particles{particle, energy}");
            match cell.query(q) {
                Ok(out) => {
                    println!("  {}", out.columns.join("    "));
                    for row in &out.rows {
                        let cells: Vec<String> = row.iter().map(cell_str).collect();
                        println!("  {}", cells.join("    "));
                    }
                }
                Err(e) => println!("  error: {e:?}"),
            }
        }
        "state" => println!("  {} particles", cell.particles()),
        "help" | "?" => println!("{HELP}"),
        "quit" | "exit" | "q" => return false,
        "" => {}
        other => println!("  unknown: {other}   (try: help)"),
    }
    true
}

/// Where a cyb keeps its graph by default.
fn default_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".cyb").join("graph.log")
}

/// Receive links from peers into this cell, forever. Each applied link
/// persists (it goes through the durable log), so this node's graph grows
/// from the network exactly as from local links.
fn listen(cell: &mut Cell, addr: &str) {
    let listener = match TcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("bind {addr}: {e}");
            return;
        }
    };
    println!("cyb listening on {addr} — links from peers land in this cell");
    for stream in listener.incoming().flatten() {
        let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            match cell.receive(&line) {
                Some(_) => println!("  ← {peer}  applied a link"),
                None => println!("  ← {peer}  bad line"),
            }
        }
    }
}

/// Ship one link to a peer:  send <addr> <a> <b>
fn send(neuron: [u8; 32], args: &[String]) {
    let (addr, a, b) = match (args.get(1), args.get(2), args.get(3)) {
        (Some(addr), Some(a), Some(b)) => (addr, a, b),
        _ => {
            eprintln!("usage: send <addr> <a> <b>");
            return;
        }
    };
    let line = Cell::wire(&neuron, &pid(a), &pid(b));
    match TcpStream::connect(addr) {
        Ok(mut s) => {
            if writeln!(s, "{line}").is_ok() {
                println!("sent {a} → {b}  to {addr}");
            }
        }
        Err(e) => eprintln!("connect {addr}: {e}"),
    }
}

fn main() {
    let neuron = [1u8; 32];
    let path = default_path();
    // durable by default — the graph survives restart
    let mut cell = Cell::open(&path).unwrap_or_else(|_| Cell::ephemeral());

    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("listen") => {
            listen(&mut cell, args.get(1).map(String::as_str).unwrap_or("127.0.0.1:7700"));
            return;
        }
        Some("send") => {
            send(neuron, &args);
            return;
        }
        _ => {}
    }
    if !args.is_empty() {
        exec(&mut cell, neuron, &args.join(" "));
        return;
    }

    println!("cyb — a live cell.   {}   type `help`.", path.display());
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
