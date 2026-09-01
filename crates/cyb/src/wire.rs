//! wire — cells converge over radio, and the graph itself does the routing.
//!
//! Following is not a network primitive here — it is a cyberlink. `follow X`
//! casts `FOLLOW → X` onto my chain; a node's address is likewise two links
//! on its own chain, `ANTENNA → endpoint-id` and `SOCKET → packed ip:port`.
//! The wire then merely *obeys the graph*: it syncs the chains of neurons I
//! follow, forwards a frame to any peer whose follows want it, and when my
//! cell holds a followed neuron's antenna — learned through any other peer —
//! it dials that neuron directly. Discovery, subscription and routing are
//! all graph content, replicated by the same signals they route.
//!
//! The session protocol stays the smallest honest one (ALPN `cyb/sync/1`).
//! Every blob wears one tag byte. A HELLO blob carries the 32-byte neuron
//! ids the sender wants (its follows plus itself) and is answered with a
//! snapshot of those chains; a FRAMES blob carries signals. HELLO can be
//! re-sent at any time — casting a new follow re-hellos every live session,
//! so a subscription made mid-session takes effect at once. Snapshot,
//! replay and push all feed the one idempotent commit, so any topology
//! converges to the union of what its follow edges ask for. A frame that
//! deduplicates (nothing applied) is not re-forwarded, so echoes die at one
//! hop; gossip-grade dedup at scale is foculus's seat, not the wire's.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayMode, TransportAddr};
use tokio::sync::{broadcast, mpsc};

use crate::cell::Cell;
use cybergraph::{NeuronId, Particle};

/// The sync protocol's name on the wire.
pub const ALPN: &[u8] = b"cyb/sync/1";

/// A well-known particle — the wire's vocabulary. Anyone can mint them:
/// they are just the hemera hashes of the words.
fn wkp(word: &str) -> Particle {
    *cyber_hemera::hash(word.as_bytes()).as_bytes()
}

/// A particle from words — the same words are the same particle everywhere;
/// content addressing is the rendezvous.
fn particle(text: &str) -> Particle {
    wkp(text)
}

fn short(p: &[u8; 32]) -> String {
    p[..4].iter().map(|b| format!("{b:02x}")).collect()
}

fn hex(p: &[u8; 32]) -> String {
    p.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

// ── the graph as address book ────────────────────────────────────────────

/// Pack `ip:port` into a particle: a tag byte, then the address. IPv4 only
/// for now; a relay URL is content, and content belongs to blobs — later.
fn pack_sock(s: &std::net::SocketAddr) -> Option<Particle> {
    let std::net::IpAddr::V4(ip) = s.ip() else { return None };
    let mut p = [0u8; 32];
    p[0] = b'4';
    p[1..5].copy_from_slice(&ip.octets());
    p[5..7].copy_from_slice(&s.port().to_be_bytes());
    Some(p)
}

fn unpack_sock(p: &Particle) -> Option<std::net::SocketAddr> {
    if p[0] != b'4' {
        return None;
    }
    let ip = std::net::Ipv4Addr::new(p[1], p[2], p[3], p[4]);
    let port = u16::from_be_bytes([p[5], p[6]]);
    Some(std::net::SocketAddr::from((ip, port)))
}

/// Every `to` that `neuron` has cast from `from`, in chain order.
fn links_from(cell: &Cell, neuron: &NeuronId, from: &Particle) -> Vec<Particle> {
    let Some(chain) = cell.graph.chains.get(neuron) else { return Vec::new() };
    let mut out = Vec::new();
    for sig in chain.entries.values() {
        for l in &sig.links {
            if &l.from == from {
                out.push(l.to);
            }
        }
    }
    out
}

/// Whom `me` follows, per the FOLLOW links on my own chain.
fn follows(cell: &Cell, me: &NeuronId) -> HashSet<NeuronId> {
    links_from(cell, me, &wkp("follow")).into_iter().collect()
}

/// A neuron's address, as its own chain last declared it: the latest
/// ANTENNA link is the endpoint id, SOCKET links are where it listens.
fn lookup(cell: &Cell, neuron: &NeuronId) -> Option<EndpointAddr> {
    let id = *links_from(cell, neuron, &wkp("antenna")).last()?;
    let id = EndpointId::from_bytes(&id).ok()?;
    let addrs: Vec<TransportAddr> = links_from(cell, neuron, &wkp("socket"))
        .iter()
        .filter_map(unpack_sock)
        .map(TransportAddr::Ip)
        .collect();
    if addrs.is_empty() {
        return None;
    }
    Some(EndpointAddr::from_parts(id, addrs))
}

// ── the hub: one cell, many sessions ─────────────────────────────────────

struct Peer {
    id: EndpointId,
    /// The neurons this peer asked for at hello.
    wants: HashSet<NeuronId>,
    tx: mpsc::Sender<Vec<u8>>,
}

struct Hub {
    cell: Mutex<Cell>,
    me: NeuronId,
    peers: Mutex<Vec<Peer>>,
    /// Rings when my follows change, so every session re-hellos.
    rehello: broadcast::Sender<()>,
    /// Endpoints a dial is in flight to, so a slow connect is not raced by
    /// the next dialer tick into a second session.
    dialing: Mutex<HashSet<EndpointId>>,
}

impl Hub {
    /// Send a signal by `neuron` to every connected peer that wants that
    /// neuron — except the one it came from. Best-effort: a full queue drops
    /// the frame; anti-entropy at the next hello repairs it.
    fn fanout(&self, origin: Option<&EndpointId>, neuron: &NeuronId, frame: &[u8]) {
        for p in self.peers.lock().unwrap().iter() {
            if Some(&p.id) != origin && p.wants.contains(neuron) {
                let _ = p.tx.try_send(frame.to_vec());
            }
        }
    }

    /// Commit a batch of frames from a peer; forward what was genuinely new.
    fn absorb_and_forward(&self, origin: &EndpointId, blob: &[u8]) -> usize {
        let mut applied = 0;
        for sig in foculus::decode_signals(blob) {
            let neuron = sig.neuron;
            let frame = foculus::encode_signal_frame(&sig);
            let landed = self.cell.lock().unwrap().commit_public(sig).is_ok();
            if landed {
                applied += 1;
                self.fanout(Some(origin), &neuron, &frame);
            }
        }
        applied
    }

    /// Cast links locally and push the frame to every peer that wants me.
    fn cast(&self, links: impl IntoIterator<Item = (Particle, Particle)>) -> Result<()> {
        let (neuron, frame) = {
            let mut c = self.cell.lock().unwrap();
            let p = c.cast(self.me, links).map_err(|e| anyhow::anyhow!("{e:?}"))?;
            let frame = c
                .signals()
                .into_iter()
                .find(|s| s.hash() == p)
                .map(foculus::encode_signal_frame)
                .context("frame vanished after cast")?;
            (self.me, frame)
        };
        self.fanout(None, &neuron, &frame);
        Ok(())
    }

    /// What I want from a peer: the chains I follow, and my own (another
    /// device of mine may hold signals this one lost).
    fn wants(&self) -> HashSet<NeuronId> {
        let cell = self.cell.lock().unwrap();
        let mut w = follows(&cell, &self.me);
        w.insert(self.me);
        w
    }

    /// The frames of every signal I hold whose neuron is in `wanted`.
    fn snapshot_for(&self, wanted: &HashSet<NeuronId>) -> Vec<u8> {
        let cell = self.cell.lock().unwrap();
        let mut out = Vec::new();
        for sig in cell.signals() {
            if wanted.contains(&sig.neuron) {
                out.extend_from_slice(&foculus::encode_signal_frame(sig));
            }
        }
        out
    }
}

// ── framing ──────────────────────────────────────────────────────────────

const HELLO: u8 = 0;
const FRAMES: u8 = 1;

async fn write_blob(send: &mut iroh::endpoint::SendStream, tag: u8, bytes: &[u8]) -> Result<()> {
    send.write_all(&[tag]).await?;
    send.write_all(&(bytes.len() as u32).to_le_bytes()).await?;
    send.write_all(bytes).await?;
    Ok(())
}

async fn read_blob(recv: &mut iroh::endpoint::RecvStream) -> Result<(u8, Vec<u8>)> {
    let mut tag = [0u8; 1];
    recv.read_exact(&mut tag).await?;
    let mut len = [0u8; 4];
    recv.read_exact(&mut len).await?;
    let len = u32::from_le_bytes(len) as usize;
    anyhow::ensure!(len <= 1 << 26, "blob too large: {len}");
    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf).await?;
    Ok((tag[0], buf))
}

fn parse_wants(bytes: &[u8]) -> HashSet<NeuronId> {
    bytes
        .chunks_exact(32)
        .map(|c| {
            let mut n = [0u8; 32];
            n.copy_from_slice(c);
            n
        })
        .collect()
}

// ── a session ────────────────────────────────────────────────────────────

/// hello (wants) → snapshot of those chains → live frames; hello may repeat.
async fn session(conn: iroh::endpoint::Connection, dialer: bool, hub: Arc<Hub>) -> Result<()> {
    let peer_id = conn.remote_id();
    let (mut send, mut recv) = if dialer {
        conn.open_bi().await?
    } else {
        conn.accept_bi().await?
    };

    // opening hello, both ways
    let hello: Vec<u8> = hub.wants().iter().flat_map(|n| n.iter().copied()).collect();
    write_blob(&mut send, HELLO, &hello).await?;
    let (tag, theirs) = read_blob(&mut recv).await?;
    anyhow::ensure!(tag == HELLO, "peer began without hello");
    let peer_wants = parse_wants(&theirs);

    // anti-entropy over exactly the wanted chains
    write_blob(&mut send, FRAMES, &hub.snapshot_for(&peer_wants)).await?;

    // register for live fanout and for re-hello
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(64);
    hub.peers.lock().unwrap().push(Peer { id: peer_id, wants: peer_wants, tx });
    let mut rehello = hub.rehello.subscribe();
    println!("~ {} connected", short(peer_id.as_bytes()));

    let result: Result<()> = async {
        loop {
            tokio::select! {
                frame = rx.recv() => {
                    let Some(frame) = frame else { break };
                    write_blob(&mut send, FRAMES, &frame).await?;
                }
                _ = rehello.recv() => {
                    // my follows changed — ask for the new chains at once
                    let hello: Vec<u8> =
                        hub.wants().iter().flat_map(|n| n.iter().copied()).collect();
                    write_blob(&mut send, HELLO, &hello).await?;
                }
                blob = read_blob(&mut recv) => {
                    let (tag, blob) = blob?;
                    match tag {
                        HELLO => {
                            // the peer's follows changed: remember them and
                            // ship what it now wants — absorb dedups repeats
                            let wants = parse_wants(&blob);
                            let snap = hub.snapshot_for(&wants);
                            if let Some(p) = hub
                                .peers
                                .lock()
                                .unwrap()
                                .iter_mut()
                                .find(|p| p.id == peer_id)
                            {
                                p.wants = wants;
                            }
                            write_blob(&mut send, FRAMES, &snap).await?;
                        }
                        _ => {
                            let applied = hub.absorb_and_forward(&peer_id, &blob);
                            if applied > 0 {
                                println!(
                                    "← +{applied} from {}, cell holds {}",
                                    short(peer_id.as_bytes()),
                                    hub.cell.lock().unwrap().len()
                                );
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    .await;

    hub.peers.lock().unwrap().retain(|p| p.id != peer_id);
    println!("~ {} left", short(peer_id.as_bytes()));
    result
}

// ── the node ─────────────────────────────────────────────────────────────

/// Bring a cell onto the wire: bind, declare my antenna on my own chain,
/// accept peers, obey the graph's follow-links to dial peers, and take
/// commands. `bootstrap` is the first contact ever — after that, addresses
/// come from the graph.
pub async fn up(cell_path: Option<&str>, bootstrap: Option<(String, Vec<String>)>) -> Result<()> {
    let cell = match cell_path {
        Some(p) => {
            println!("cell log: {p}");
            Cell::open(p)?
        }
        None => {
            println!("ephemeral cell (pass a path to persist)");
            Cell::ephemeral()
        }
    };

    let endpoint = Endpoint::builder()
        .alpns(vec![ALPN.to_vec()])
        .relay_mode(RelayMode::Disabled)
        .bind()
        .await?;
    let id = endpoint.id();
    let me: NeuronId = *cyber_hemera::hash(id.as_bytes()).as_bytes();
    let hub = Arc::new(Hub {
        cell: Mutex::new(cell),
        me,
        peers: Mutex::new(Vec::new()),
        rehello: broadcast::channel(8).0,
        dialing: Mutex::new(HashSet::new()),
    });

    // My address, as content on my own chain — what lets a stranger who
    // learns my chain through anyone dial me without being told my socket.
    let socks: Vec<std::net::SocketAddr> = endpoint
        .bound_sockets()
        .into_iter()
        .map(|mut s| {
            // 0.0.0.0 is where I listen; 127.0.0.1 is where this machine's
            // neighbours find me. LAN and relay reachability are a richer
            // antenna record, later.
            if s.ip().is_unspecified() {
                s.set_ip(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
            }
            s
        })
        .collect();
    {
        let mut links: Vec<(Particle, Particle)> = vec![(wkp("antenna"), *id.as_bytes())];
        links.extend(socks.iter().filter_map(pack_sock).map(|p| (wkp("socket"), p)));
        // one signal: the address record lands atomically
        hub.cast(links)?;
    }

    println!("neuron   {}", hex(&me));
    println!("endpoint {id}");
    if let Some(s) = socks.iter().find(|s| s.is_ipv4()) {
        println!("bootstrap from elsewhere:  cy wire up {id} {s}");
    }
    println!("commands:  follow <neuron>   link <from> <to>   ls   follows   peers   q");

    // accept
    let accept = {
        let (hub, endpoint) = (hub.clone(), endpoint.clone());
        tokio::spawn(async move {
            while let Some(incoming) = endpoint.accept().await {
                let Ok(conn) = incoming.await else { continue };
                let hub = hub.clone();
                tokio::spawn(async move {
                    if let Err(e) = session(conn, false, hub).await {
                        println!("~ session ended: {e:#}");
                    }
                });
            }
        })
    };

    // bootstrap contact, if given
    if let Some((peer, socks)) = bootstrap {
        let peer: EndpointId = peer.parse().context("bad endpoint id")?;
        let addrs: Vec<TransportAddr> = socks
            .iter()
            .filter_map(|s| s.parse().ok())
            .map(TransportAddr::Ip)
            .collect();
        anyhow::ensure!(!addrs.is_empty(), "bootstrap needs ip:port");
        let conn = endpoint.connect(EndpointAddr::from_parts(peer, addrs), ALPN).await?;
        let hub2 = hub.clone();
        tokio::spawn(async move {
            if let Err(e) = session(conn, true, hub2).await {
                println!("~ session ended: {e:#}");
            }
        });
    }

    // The graph dials: any followed neuron whose antenna I hold and whose
    // endpoint I am not connected to gets a call. Addresses arrive by sync
    // like any other signal, so a third cell reaches me through a friend.
    let dialer = {
        let (hub, endpoint) = (hub.clone(), endpoint.clone());
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                let targets: Vec<(NeuronId, EndpointAddr)> = {
                    let cell = hub.cell.lock().unwrap();
                    follows(&cell, &hub.me)
                        .iter()
                        .filter_map(|n| lookup(&cell, n).map(|a| (*n, a)))
                        .collect()
                };
                for (neuron, addr) in targets {
                    let busy = hub.peers.lock().unwrap().iter().any(|p| p.id == addr.id)
                        || !hub.dialing.lock().unwrap().insert(addr.id);
                    if busy || addr.id == endpoint.id() {
                        continue;
                    }
                    let (hub, endpoint) = (hub.clone(), endpoint.clone());
                    tokio::spawn(async move {
                        if let Ok(conn) = endpoint.connect(addr.clone(), ALPN).await {
                            println!("° dialed {} via the graph", short(&neuron));
                            if let Err(e) = session(conn, true, hub.clone()).await {
                                println!("~ session ended: {e:#}");
                            }
                        }
                        // unreachable is fine — the graph will say when
                        hub.dialing.lock().unwrap().remove(&addr.id);
                    });
                }
            }
        })
    };

    repl(hub).await?;
    accept.abort();
    dialer.abort();
    Ok(())
}

async fn repl(hub: Arc<Hub>) -> Result<()> {
    let (line_tx, mut line_rx) = mpsc::channel::<String>(4);
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
            Some("follow") => {
                let Some(n) = it.next().and_then(hex32) else {
                    println!("usage: follow <64-hex neuron>");
                    continue;
                };
                match hub.cast([(wkp("follow"), n)]) {
                    Ok(()) => {
                        // live sessions learn my new appetite immediately
                        let _ = hub.rehello.send(());
                        println!("→ following {}", short(&n));
                    }
                    Err(e) => println!("cast failed: {e:#}"),
                }
            }
            Some("follows") => {
                let cell = hub.cell.lock().unwrap();
                for n in follows(&cell, &hub.me) {
                    let known = lookup(&cell, &n).is_some();
                    println!("  {}{}", hex(&n), if known { "  · antenna known" } else { "" });
                }
            }
            Some("link") => {
                let (Some(a), Some(b)) = (it.next(), it.next()) else {
                    println!("usage: link <from> <to>");
                    continue;
                };
                match hub.cast([(particle(a), particle(b))]) {
                    Ok(()) => println!(
                        "→ {a} → {b} cast, cell holds {}",
                        hub.cell.lock().unwrap().len()
                    ),
                    Err(e) => println!("cast failed: {e:#}"),
                }
            }
            Some("ls") => {
                let c = hub.cell.lock().unwrap();
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
            Some("peers") => {
                for p in hub.peers.lock().unwrap().iter() {
                    println!("  {} wants {} chain(s)", short(p.id.as_bytes()), p.wants.len());
                }
            }
            Some("q") | Some("quit") | Some("exit") => break,
            Some(other) => {
                println!("unknown `{other}` — follow / link / ls / follows / peers / q")
            }
            None => {}
        }
    }
    Ok(())
}
