//! Shared rune cell runtime — the seam cyb worlds use to author pages in rune.
//!
//! Load source → parse/lower/eval (with a host) → prysm chunks. The binary
//! stays a frozen shell; the page is the cell.

use bevy::prelude::*;
use prysm::dispatch;
use rune_ast::{Noun, act, tag};
use rune_interp::{Host, InterpError};
use tade::{Chunk, render, sigil};

const BUILTIN: &[(&str, &str)] = &[
    ("landing", include_str!("../../../cells/landing.rune")),
    ("memory", include_str!("../../../cells/memory.rune")),
    ("log", include_str!("../../../cells/log.rune")),
];

pub fn load(name: &str) -> Result<String, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cells")
        .join(format!("{name}.rune"));
    std::fs::read_to_string(&path)
        .or_else(|e| {
            BUILTIN
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, src)| (*src).to_string())
                .ok_or_else(|| format!("cell {name}: {e}"))
        })
        .map(|s| s.replace(['\n', '\r'], " "))
}

pub fn eval(src: &str, host: &mut dyn Host) -> Result<Vec<Chunk>, String> {
    let ast = rune_parse::parse(src).map_err(|e| format!("parse: {}", e.message))?;
    let formula = rune_lower::lower(ast).map_err(|e| format!("lower: {}", e.message))?;
    let subject = rune_subject::Subject::minimal().to_noun();
    let result = rune_interp::eval_with_host(&subject, &formula, host)
        .map_err(|e| format!("eval: {}", e.message))?;
    Ok(rune_prysm::noun_to_chunks(&result))
}

pub fn tape(s: &str) -> Noun {
    let mut n = Noun::Atom(0);
    for &b in s.as_bytes().iter().rev() {
        n = Noun::cell(Noun::Atom(b as u64), n);
    }
    n
}

pub fn list(items: Vec<Noun>) -> Noun {
    let mut n = Noun::Atom(0);
    for item in items.into_iter().rev() {
        n = Noun::cell(item, n);
    }
    Noun::cell(Noun::Atom(tag::LIST), n)
}

pub fn button(label: &str, target: &str) -> Noun {
    Noun::cell(
        Noun::Atom(tag::BUTTON),
        Noun::cell(tape(label), tape(target)),
    )
}

pub fn row(cells: &[&str]) -> Noun {
    let mut n = Noun::Atom(0);
    for s in cells.iter().rev() {
        n = Noun::cell(tape(s), n);
    }
    Noun::cell(Noun::Atom(tag::ROW), n)
}

pub fn query_name(args: &Noun) -> String {
    String::from_utf8_lossy(&rune_prysm::noun_to_bytes(args)).into_owned()
}

pub fn unknown_query(name: &str) -> InterpError {
    InterpError {
        message: format!("query {name}: unknown"),
    }
}

pub fn act_is_query(act: u64) -> bool {
    act == act::QUERY
}

/// Compact census numbers: 12, 12.4k, 1.2M.
pub fn compact(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 10_000 {
        format!("{:.1}k", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}

/// Dispatch a cell's chunks into `parent`. A wrapping component chunk
/// (the whole page) is one dispatch; otherwise each child is.
pub fn dispatch_page(commands: &mut Commands, parent: Entity, chunks: &[Chunk]) {
    let mut rest = chunks;
    if let Some(first) = rest.first() {
        if first.sigil == sigil::LUS && first.render == render::COMPONENT {
            dispatch(commands, parent, first);
            rest = &rest[1..];
        }
    }
    for chunk in rest {
        dispatch(commands, parent, chunk);
    }
}
