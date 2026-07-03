//! cy — the terminal face of a cyb.  (`cyb` is the GUI; `cy` is the CLI.)
//!
//!   cy                         the `cyb›` REPL (interactive)
//!   cy link cat dog            one-shot
//!   cy query                   one-shot query
//!   cy bind <claim>            record a verified mudra migration claim
//!   cy pull <peer.log>         absorb a peer's signals (file anti-entropy)
//!
//! Both modes drive a `cyb_core::Cell`. The REPL is just the interactive mode.
//! Networking rides the stack's own signal frames — a cell's durable log *is*
//! its snapshot of tape frames, so pulling a peer is just absorbing that file.
//! Live transport (radio/QUIC) plugs in above this on the same frames.

use cyb_core::{Cell, Signal};
use inf_value::Value;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::path::Path;

// ── color ─────────────────────────────────────────────────────────────────
// ANSI only when stdout is a terminal — piped output stays clean.

fn tty() -> bool {
    io::stdout().is_terminal()
}

/// Wrap `s` in an SGR code (e.g. "36" for cyan), or return it bare off-tty.
fn paint(code: &str, s: &str) -> String {
    if tty() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

fn dim(s: &str) -> String {
    paint("90", s)
}
fn cyan(s: &str) -> String {
    paint("36", s)
}
fn green(s: &str) -> String {
    paint("32", s)
}
fn yellow(s: &str) -> String {
    paint("33", s)
}
fn bold(s: &str) -> String {
    paint("1", s)
}

/// The rainbow ANSI-Shadow wordmark — printed only on a terminal.
const LOGO: &str = "\
\x1b[31m ██████╗██╗   ██╗██████╗ \x1b[0m
\x1b[33m██╔════╝╚██╗ ██╔╝██╔══██╗\x1b[0m
\x1b[32m██║      ╚████╔╝ ██████╔╝\x1b[0m
\x1b[36m██║       ╚██╔╝  ██╔══██╗\x1b[0m
\x1b[34m╚██████╗   ██║   ██████╔╝\x1b[0m
\x1b[35m ╚═════╝   ╚═╝   ╚═════╝ \x1b[0m";

/// Banner: wordmark + tagline + the field/graph parameters, hemera-style.
fn banner() -> String {
    if !tty() {
        return String::new();
    }
    format!(
        "{LOGO}\n{tag}\n{params}\n",
        tag = paint("37", "    an immortal robot"),
        params = dim(
            "\n    Goldilocks field · p = 2^64 - 2^32 + 1\n    \
             cyberlink graph · per-neuron signal chains\n    \
             event-sourced · never forgets · converges to φ*\n"
        ),
    )
}

// ── rendering ───────────────────────────────────────────────────────────────

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

/// Render one query cell to a plain string (no color).
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

/// Print a query result as an aligned, colored table.
fn print_table(cols: &[String], rows: &[Vec<Value>]) {
    // rendered cell text, per column, to size the columns
    let ncol = cols.len();
    let mut width: Vec<usize> = cols.iter().map(|c| c.chars().count()).collect();
    let text: Vec<Vec<String>> =
        rows.iter().map(|r| r.iter().map(cell_str).collect::<Vec<_>>()).collect();
    for row in &text {
        for (i, cell) in row.iter().enumerate().take(ncol) {
            width[i] = width[i].max(cell.chars().count());
        }
    }

    // header
    let header: String = cols
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:<w$}", c, w = width[i]))
        .collect::<Vec<_>>()
        .join("  ");
    println!("  {}", dim(&header));

    // rows — first column (the particle) cyan, numbers yellow
    for (row, vals) in text.iter().zip(rows) {
        let cells: Vec<String> = row
            .iter()
            .enumerate()
            .take(ncol)
            .map(|(i, cell)| {
                let padded = format!("{:<w$}", cell, w = width[i]);
                match (i, vals.get(i)) {
                    (0, _) => cyan(&padded),
                    (_, Some(Value::Int(_))) => yellow(&padded),
                    _ => padded,
                }
            })
            .collect();
        println!("  {}", cells.join("  "));
    }
    if rows.is_empty() {
        println!("  {}", dim("(empty)"));
    }
}

fn help() {
    let rows = [
        ("id", "this cyb's neuron + pussy address"),
        ("link <a> <b>", "assert a cyberlink a → b"),
        ("bind <claim>", "record a verified mudra migration claim"),
        ("query [inf]", "the graph's nodes (or run a raw inf script)"),
        ("axons", "the graph's edges: from → to, with weight"),
        ("log", "the signal log — the event history state derives from"),
        ("pull <path>", "absorb a peer's signal log (file anti-entropy)"),
        ("state", "how many nodes, axons, and signals the cell holds"),
        ("tools", "the cyber toolset (hemera, nox, …) — run any by name"),
        ("help · quit", "this · leave"),
    ];
    let w = rows.iter().map(|(c, _)| c.len()).max().unwrap_or(0);
    println!("{}", dim("commands"));
    for (cmd, desc) in rows {
        println!("  {}   {}", bold(&format!("{cmd:<w$}")), dim(desc));
    }
    let sep = dim(" · ");
    println!(
        "\n  {}\n    {}",
        dim("one-shot"),
        [green("cy link cat dog"), green("cy bind <claim>"), green("cy axons"), green("cy log")]
            .join(&sep),
    );
}

/// Print the signal log — the event history the graph state is derived from.
/// Each signal shows its particle, the neuron that headed it, its chain step,
/// and the cyberlinks it carries. This is the source of truth; `query` reads
/// the state these signals produce.
fn show_log(cell: &Cell) {
    let sigs = cell.signals();
    if sigs.is_empty() {
        println!("  {}", dim("(no signals yet)"));
        return;
    }
    for s in &sigs {
        let sh: String = s.hash()[..3].iter().map(|b| format!("{b:02x}")).collect();
        print_signal(s, &sh);
    }
    println!("  {}", dim(&format!("{} signal(s)", sigs.len())));
}

/// One signal header + its cyberlinks.
fn print_signal(s: &Signal, short_hash: &str) {
    println!(
        "  {} {}  {} {}  {} {}",
        dim("signal"),
        yellow(&format!("{short_hash}…")),
        dim("neuron"),
        cyan(&particle(&s.neuron)),
        dim("step"),
        yellow(&s.step.to_string()),
    );
    for l in &s.links {
        println!("    {} {} {}", cyan(&particle(&l.from)), dim("→"), cyan(&particle(&l.to)));
    }
}

/// The graph's nodes: linked particles and their energy. A node's `particle`
/// column is a readable label when it was linked from one, else a short hash.
fn show_nodes(cell: &Cell) {
    let nodes = cell.nodes();
    if nodes.is_empty() {
        println!("  {}", dim("(no nodes yet)"));
        return;
    }
    let labels: Vec<String> = nodes.iter().map(|(p, _)| particle(p)).collect();
    let w = labels.iter().map(|l| l.chars().count()).max().unwrap_or(0).max(8);
    println!("  {}  {}", dim(&format!("{:<w$}", "particle")), dim("energy"));
    for ((_, e), label) in nodes.iter().zip(&labels) {
        println!("  {}  {}", cyan(&format!("{label:<w$}")), yellow(&e.to_string()));
    }
}

/// The graph's axons (edges): `from → to` with the pair's accumulated weight.
fn show_axons(cell: &Cell) {
    let axons = cell.axons();
    if axons.is_empty() {
        println!("  {}", dim("(no axons yet)"));
        return;
    }
    for (from, to, weight) in &axons {
        println!(
            "  {} {} {}  {}",
            cyan(&particle(from)),
            dim("→"),
            cyan(&particle(to)),
            dim(&format!("weight {weight}")),
        );
    }
    println!("  {}", dim(&format!("{} axon(s)", axons.len())));
}

// ── the cyber toolset ────────────────────────────────────────────────────────
// cy is the doorway to the stack: any sibling tool's name, typed here, runs it.

const TOOLS: &[(&str, &str)] = &[
    ("hemera", "the hash — Poseidon2 over Goldilocks"),
    ("mudra", "the seal — keys, seeds, migration claims"),
    ("nox", "the VM — reduce nouns, run programs"),
    ("rune", "the language — lowers to nox"),
    ("cybergraph", "the cyberlink processor — link, seal, chain"),
    ("bbg", "the authenticated state — root, prove, dump"),
    ("inf", "the query engine — datalog over sets"),
    ("zheng", "the proof system — run, prove, verify"),
    ("eidos", "the proof kernel — CIC type theory"),
    ("tru", "the truth layer — focus, cyberank, valence"),
];

/// Is `cmd` a binary on `$PATH`?
fn on_path(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|dir| dir.join(cmd).exists()))
        .unwrap_or(false)
}

/// Split a command tail into arguments, honoring single and double quotes so a
/// quoted value with spaces (e.g. an inf query script) stays one argument.
fn split_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut cur = String::new();
    let mut open = false; // are we mid-token?
    let (mut in_single, mut in_double) = (false, false);
    for c in s.chars() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if open {
                    args.push(std::mem::take(&mut cur));
                    open = false;
                }
                continue;
            }
            c => cur.push(c),
        }
        open = true;
    }
    if open {
        args.push(cur);
    }
    args
}

/// cy's own builtin verbs — everything else is dispatched to a sibling tool.
const BUILTINS: &[&str] = &[
    "id", "whoami", "link", "query", "axons", "edges", "pull", "bind", "log", "state", "tools",
    "deps", "help", "?", "quit", "exit", "q", "",
];

fn is_builtin(cmd: &str) -> bool {
    BUILTINS.contains(&cmd)
}

/// Run a sibling cyber tool with an already-split argument list (one-shot: argv
/// is passed through verbatim, so quoting is preserved). Returns `false` only
/// when the command is neither a known tool nor on PATH.
fn dispatch_argv(cmd: &str, args: &[String]) -> bool {
    match std::process::Command::new(cmd).args(args).status() {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if TOOLS.iter().any(|(t, _)| *t == cmd) {
                println!(
                    "  {} is a cyber tool, not yet on PATH — {}",
                    bold(cmd),
                    green(&format!("sh ~/cyber/scripts/install-bins.sh {cmd}")),
                );
                true
            } else {
                false
            }
        }
        Err(e) => {
            println!("  {}: {cmd}: {e}", paint("31", "error"));
            true
        }
    }
}

/// Dispatch from the REPL, where the quoted tail must be re-split.
fn dispatch(cmd: &str, rest: &str) -> bool {
    dispatch_argv(cmd, &split_args(rest))
}

/// List the cyber toolset and which tools are installed.
fn show_tools() {
    println!("{}", dim("cyber toolset — type any as `<tool> …`, right here"));
    for (name, desc) in TOOLS {
        let (mark, _) = if on_path(name) { (green("●"), true) } else { (dim("○"), false) };
        println!("  {} {}  {}", mark, bold(&format!("{name:<10}")), dim(desc));
    }
    println!("  {}", dim("○ not installed · sh ~/cyber/scripts/install-bins.sh <tool>"));
}

// ── commands ────────────────────────────────────────────────────────────────

/// Absorb a peer's durable log — its concatenated signal frames — into this
/// cell. The grow-only graph converges to the union; re-pulling is a no-op.
fn pull(cell: &mut Cell, peer: &Path) {
    match std::fs::File::open(peer) {
        Ok(mut f) => {
            let mut bytes = Vec::new();
            if f.read_to_end(&mut bytes).is_ok() {
                let n = cell.absorb(&bytes);
                let tag = if n == 0 { dim("already in sync") } else { green(&format!("+{n} new")) };
                println!("  {} {}  {}", dim("pull"), peer.display(), tag);
            } else {
                println!("  {}: could not read {}", paint("31", "error"), peer.display());
            }
        }
        Err(e) => println!("  {}: {} — {e}", paint("31", "error"), peer.display()),
    }
}

/// Ingest a mudra migration claim. Verify that the holder controls the legacy
/// Cosmos key, then record the binding `legacy address → native neuron` as a
/// cyberlink authored by that neuron — migration becomes an immortal graph fact.
fn bind(cell: &mut Cell, claim_str: &str) {
    let Some(c) = mudra::Claim::decode(claim_str) else {
        println!(
            "  {}: malformed claim (expected: address pubkey neuron signature)",
            paint("31", "error")
        );
        return;
    };
    if !mudra::claim::verify(&c, mudra::cosmos::PUSSY) {
        println!("  {} {}", paint("31", "✗"), bold("claim does not verify"));
        return;
    }
    // legacy identity as a particle: the 20-byte Cosmos account id, padded
    let mut legacy = [0u8; 32];
    legacy[..20].copy_from_slice(&mudra::cosmos::account_id(&c.pubkey));
    // authored by the migrating neuron itself: it asserts it owns the legacy account
    match cell.link(c.neuron, legacy, c.neuron) {
        Ok(sig) => {
            let s: String = sig[..3].iter().map(|b| format!("{b:02x}")).collect();
            println!(
                "  {} {} {} {}  {}",
                green("✓"),
                cyan(&c.address),
                dim("→ neuron"),
                cyan(&particle(&c.neuron)),
                dim(&format!("signal {s}…")),
            );
            println!("  {}", dim("migration recorded — the neuron now owns its legacy address"));
        }
        Err(e) => println!("  {}: {e:?}", paint("31", "error")),
    }
}

/// Run one command line against the cell. Returns false to stop the REPL.
fn exec(cell: &mut Cell, id: &Id, line: &str) -> bool {
    let mut it = line.trim().splitn(2, ' ');
    match it.next().unwrap_or("") {
        "id" | "whoami" => show_id(id),
        "link" => {
            let mut ab = it.next().unwrap_or("").split_whitespace();
            match (ab.next(), ab.next()) {
                (Some(a), Some(b)) => match cell.link(id.neuron, pid(a), pid(b)) {
                    Ok(sig) => {
                        let s: String = sig[..3].iter().map(|b| format!("{b:02x}")).collect();
                        println!(
                            "  {} {} {} {}  {}",
                            green("✓"),
                            cyan(a),
                            dim("→"),
                            cyan(b),
                            dim(&format!("signal {s}…")),
                        );
                    }
                    Err(e) => println!("  {}: {e:?}", paint("31", "error")),
                },
                _ => println!("  {}: link <a> <b>", dim("usage")),
            }
        }
        "query" => match it.next() {
            // bare `query` = the graph's nodes; `query <inf>` runs a raw inf script
            None => show_nodes(cell),
            Some(inf) => match cell.query(inf) {
                Ok(out) => print_table(&out.columns, &out.rows),
                Err(e) => println!("  {}: {e:?}", paint("31", "error")),
            },
        },
        "axons" | "edges" => show_axons(cell),
        "pull" => match it.next() {
            Some(path) => pull(cell, Path::new(path.trim())),
            None => println!("  {}: pull <peer-log-path>", dim("usage")),
        },
        "bind" => match it.next() {
            Some(claim) => bind(cell, claim.trim()),
            None => println!("  {}: bind <claim>   (from `mudra claim …`)", dim("usage")),
        },
        "log" => show_log(cell),
        "state" => println!(
            "  {} {} {} {} {} {} {} {}",
            yellow(&cell.nodes().len().to_string()),
            dim("nodes"),
            dim("·"),
            yellow(&cell.axons().len().to_string()),
            dim("axons"),
            dim("·"),
            yellow(&cell.len().to_string()),
            dim("signals"),
        ),
        "tools" | "deps" => show_tools(),
        "help" | "?" => help(),
        "quit" | "exit" | "q" => return false,
        "" => {}
        // anything else: try to run it as a sibling cyber tool
        other => {
            if !dispatch(other, it.next().unwrap_or("")) {
                println!("  {}: {other}   {}", dim("unknown"), dim("(try: help · tools)"));
            }
        }
    }
    true
}

/// Where a cyb keeps its graph by default. `~/cyb` is visible, not
/// dotfile-hidden — nothing about a neuron's own graph needs hiding from
/// its owner.
fn default_path() -> std::path::PathBuf {
    cyb_dir().join("graph.log")
}

fn cyb_dir() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb")
}

/// This cyb's identity: a real neuron, not a placeholder.
struct Id {
    /// the author of every signal this cyb emits: `Hemera(pubkey)`
    neuron: [u8; 32],
    /// the SEC1-compressed secp256k1 public key
    pubkey: [u8; 33],
    /// the matching legacy `pussy1…` account (same key, coin type 118)
    address: String,
}

/// Load this cyb's identity, creating one on first run. The identity is a BIP-39
/// mnemonic stored next to the graph; the neuron is `Hemera(pubkey)` of the key
/// derived at the Cosmos path — the *same* key that owns the matching pussy
/// account, so a neuron can migrate its own legacy identity to itself.
fn identity() -> Id {
    let dir = cyb_dir();
    let _ = std::fs::create_dir_all(&dir);
    let file = dir.join("mnemonic");

    let mnemonic = match std::fs::read_to_string(&file) {
        Ok(m) if !m.trim().is_empty() => m.trim().to_string(),
        _ => {
            let m = mudra::seed::generate_mnemonic().expect("generate mnemonic");
            let _ = std::fs::write(&file, format!("{m}\n"));
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600));
            }
            eprintln!("  {} new neuron minted · {}", green("✦"), dim(&file.display().to_string()));
            eprintln!("  {}", dim("back up this mnemonic — it is the only key to this neuron"));
            m
        }
    };

    let key = mudra::seed::cosmos_key(&mnemonic, "").expect("derive key from mnemonic");
    let pubkey = mudra::cosmos::compressed(key.verifying_key());
    let neuron = mudra::claim::neuron_of(&pubkey);
    let address = mudra::cosmos::address(&pubkey, mudra::cosmos::PUSSY).unwrap_or_default();
    Id { neuron, pubkey, address }
}

/// Show who this cyb is.
fn show_id(id: &Id) {
    let hx = |b: &[u8]| -> String { b.iter().map(|x| format!("{x:02x}")).collect() };
    println!("  {}  {}", dim("neuron  "), green(&hx(&id.neuron)));
    println!("  {}  {}", dim("pubkey  "), dim(&hx(&id.pubkey)));
    println!("  {}  {}", dim("pussy   "), cyan(&id.address));
}

fn main() {
    let id = identity();
    let path = default_path();
    // durable by default — the graph survives restart
    let mut cell = Cell::open(&path).unwrap_or_else(|_| Cell::ephemeral());

    let args: Vec<String> = std::env::args().skip(1).collect();
    if matches!(args.first().map(String::as_str), Some("help" | "--help" | "-h")) {
        print!("{}", banner());
        help();
        return;
    }
    if !args.is_empty() {
        let first = args[0].as_str();
        // A tool invocation keeps argv intact (quoting preserved); builtins take
        // the joined line.
        if is_builtin(first) {
            exec(&mut cell, &id, &args.join(" "));
        } else if !dispatch_argv(first, &args[1..]) {
            println!("  {}: {first}   {}", dim("unknown"), dim("(try: help · tools)"));
        }
        return;
    }

    // interactive REPL
    print!("{}", banner());
    println!(
        "  {} {}   {}",
        dim(&path.display().to_string()),
        dim("·"),
        dim(&format!("neuron {}… · {} nodes · type `help`", &hex3(&id.neuron), cell.nodes().len())),
    );
    let prompt = format!("{}{} ", cyan("cyb"), dim("›"));
    let stdin = io::stdin();
    loop {
        print!("{prompt}");
        io::stdout().flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            break;
        }
        if !exec(&mut cell, &id, &line) {
            break;
        }
    }
}

/// Short hex prefix of a particle, for one-line display.
fn hex3(b: &[u8]) -> String {
    b[..3].iter().map(|x| format!("{x:02x}")).collect()
}
