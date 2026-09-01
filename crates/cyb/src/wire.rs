//! wire — two cells converge over radio, with no server between them.
//!
//! This is the milestone the whole stack points at: the graph living on
//! *peers*, not on a node someone else runs. One cyb listens, another dials
//! its id, and from that moment the pair holds one growing graph.
//!
//! The protocol is the smallest honest one. On connect each side ships its
//! entire signal log (snapshot → [`Cell::absorb`]); afterwards every locally
//! cast signal is pushed as it lands (frame → absorb again — absorb decodes
//! a batch of one just as well). Both motions feed the same idempotent
//! commit, so replay, anti-entropy and push are one mechanism and the pair
//! converges to the union of its signals. This is exactly the shape
//! foculus's gossip spec names — a signal *is* the message — carried over
//! radio's QUIC. Fan-out beyond a pair belongs to gossip proper; a pair
//! needs no rebroadcast.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, TransportAddr};
use tokio::sync::broadcast;

use crate::cell::Cell;
use cybergraph::{NeuronId, Particle};

/// The sync protocol's name on the wire.
pub const ALPN: &[u8] = b"cyb/sync/0";

/// A length-prefixed blob, the only message shape: first blob each way is
/// the snapshot, every later one is a single signal frame. No discriminant —
/// `absorb` decodes a batch of any size, including one.
async fn write_blob(send: &mut iroh::endpoint::SendStream, bytes: &[u8]) -> Result<()> {
    send.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
    send.write_all(bytes).await?;
    Ok(())
}

async fn read_blob(recv: &mut iroh::endpoint::RecvStream) -> Result<Vec<u8>> {
    let mut len = [0u8; 4];
    recv.read_exact(&mut len).await?;
    let len = u32::from_le_bytes(len) as usize;
    anyhow::ensure!(len <= 1 << 26, "blob too large: {len}");
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await?;
    Ok(buf)
}

/// One live pairing: snapshot handshake, then frames both ways until either
/// side closes. `dialer` decides who opens the stream — QUIC needs one.
async fn session(
    conn: iroh::endpoint::Connection,
    dialer: bool,
    cell: Arc<Mutex<Cell>>,
    tx: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    let (mut send, mut recv) = if dialer {
        let pair = conn.open_bi().await?;
        pair
    } else {
        conn.accept_bi().await?
    };

    // anti-entropy: my whole log for yours, absorbed idempotently
    let mine = cell.lock().unwrap().snapshot();
    write_blob(&mut send, &mine).await?;
    let theirs = read_blob(&mut recv).await?;
    let applied = cell.lock().unwrap().absorb(&theirs);
    let held = cell.lock().unwrap().len();
    println!("~ synced: +{applied} from peer, cell holds {held} signal(s)");

    // push: my new frames to you, yours to me, one select loop
    let mut rx = tx.subscribe();
    loop {
        tokio::select! {
            frame = rx.recv() => {
                let Ok(frame) = frame else { break };
                write_blob(&mut send, &frame).await?;
            }
            blob = read_blob(&mut recv) => {
                let Ok(blob) = blob else { break };
                let applied = cell.lock().unwrap().absorb(&blob);
                if applied > 0 {
                    let held = cell.lock().unwrap().len();
                    println!("← +{applied} signal(s), cell holds {held}");
                }
            }
        }
    }
    println!("~ peer left");
    Ok(())
}

/// A particle from words: the hemera hash of the text. The same words are
/// the same particle on every machine — content addressing is the rendezvous.
fn particle(text: &str) -> Particle {
    *cyber_hemera::hash(text.as_bytes()).as_bytes()
}

/// This process's neuron: derived from the endpoint identity, so two cells
/// on one machine are two neurons, and reconnecting keeps the chain.
fn neuron_of(id: &EndpointId) -> NeuronId {
    *cyber_hemera::hash(id.as_bytes()).as_bytes()
}

/// The interactive loop shared by both roles: cast links, watch them land.
async fn repl(
    endpoint: Endpoint,
    cell: Arc<Mutex<Cell>>,
    tx: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    let me = neuron_of(&endpoint.id());
    println!("commands:  link <from> <to>   ls   q");

    let (line_tx, mut line_rx) = tokio::sync::mpsc::channel::<String>(4);
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut buf = String::new();
        loop {
            buf.clear();
            if std::io::BufRead::read_line(&mut stdin.lock(), &mut buf).unwrap_or(0) == 0 {
                break;
            }
            if line_tx.blocking_send(buf.trim().to_string()).is_err() {
                break;
            }
        }
    });

    while let Some(line) = line_rx.recv().await {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("link") => {
                let (Some(a), Some(b)) = (it.next(), it.next()) else {
                    println!("usage: link <from> <to>");
                    continue;
                };
                let (pa, pb) = (particle(a), particle(b));
                let cast = {
                    let mut c = cell.lock().unwrap();
                    c.cast(me, [(pa, pb)]).map(|p| {
                        // the frame just committed, straight off the chain
                        c.signals()
                            .into_iter()
                            .find(|s| s.hash() == p)
                            .map(|s| foculus::encode_signal_frame(s))
                    })
                };
                match cast {
                    Ok(Some(frame)) => {
                        let _ = tx.send(frame);
                        println!("→ {a} → {b} cast, cell holds {}", cell.lock().unwrap().len());
                    }
                    Ok(None) => println!("cast landed but frame not found — bug"),
                    Err(e) => println!("cast failed: {e:?}"),
                }
            }
            Some("ls") => {
                let c = cell.lock().unwrap();
                println!(
                    "{} signal(s) · {} particle(s) · {} axon(s)",
                    c.len(),
                    c.particles(),
                    c.axons().len()
                );
                for (from, to, w) in c.axons() {
                    println!("  {} → {}  ({w})", short(&from), short(&to));
                }
            }
            Some("q") | Some("quit") | Some("exit") => break,
            Some(other) => println!("unknown `{other}` — link / ls / q"),
            None => {}
        }
    }
    Ok(())
}

fn short(p: &Particle) -> String {
    p[..4].iter().map(|b| format!("{b:02x}")).collect()
}

/// Open a cell and wait for peers. Prints the id and socket to dial.
pub async fn listen(cell_path: Option<&str>) -> Result<()> {
    let cell = open(cell_path)?;
    let endpoint = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(RelayMode::Disabled)
        .bind()
        .await?;
    let id = endpoint.id();
    let socks: Vec<String> = endpoint
        .bound_sockets()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    println!("cell listening as {id}");
    println!("dial from another machine or terminal:");
    println!("  cy wire dial {id} {}", socks.join(" "));

    let (tx, _) = broadcast::channel::<Vec<u8>>(64);
    let accept = {
        let (cell, tx, endpoint) = (cell.clone(), tx.clone(), endpoint.clone());
        tokio::spawn(async move {
            while let Some(incoming) = endpoint.accept().await {
                let Ok(conn) = incoming.await else { continue };
                let (cell, tx) = (cell.clone(), tx.clone());
                tokio::spawn(async move {
                    if let Err(e) = session(conn, false, cell, tx).await {
                        println!("~ session ended: {e:#}");
                    }
                });
            }
        })
    };
    repl(endpoint, cell, tx).await?;
    accept.abort();
    Ok(())
}

/// Dial a listening cell by id and socket, then converge and stay live.
pub async fn dial(id: &str, socks: &[String], cell_path: Option<&str>) -> Result<()> {
    let cell = open(cell_path)?;
    let id: EndpointId = id.parse().context("bad endpoint id")?;
    let addrs: Vec<TransportAddr> = socks
        .iter()
        .filter_map(|s| s.parse().ok())
        .map(TransportAddr::Ip)
        .collect();
    anyhow::ensure!(!addrs.is_empty(), "give at least one ip:port to dial");
    let peer = EndpointAddr::from_parts(id, addrs);

    let endpoint = Endpoint::builder()
        .relay_mode(RelayMode::Disabled)
        .bind()
        .await?;
    let conn = endpoint.connect(peer, ALPN).await?;
    println!("connected to {id}");

    let (tx, _) = broadcast::channel::<Vec<u8>>(64);
    let handle = {
        let (cell, tx) = (cell.clone(), tx.clone());
        tokio::spawn(async move {
            if let Err(e) = session(conn, true, cell, tx).await {
                println!("~ session ended: {e:#}");
            }
        })
    };
    repl(endpoint, cell, tx).await?;
    handle.abort();
    Ok(())
}

fn open(path: Option<&str>) -> Result<Arc<Mutex<Cell>>> {
    let cell = match path {
        Some(p) => {
            println!("cell log: {p}");
            Cell::open(p)?
        }
        None => {
            println!("ephemeral cell (pass a path to persist)");
            Cell::ephemeral()
        }
    };
    Ok(Arc::new(Mutex::new(cell)))
}
