pub mod robot;
pub mod graph;
pub mod sigma;
pub mod com;
pub mod attention;
pub mod content;
pub mod identity;
pub mod models;
pub mod soma_bridge;

use bevy::prelude::*;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WorldState {
    /// The cybergraph itself, rendered by mir.
    #[default]
    Graph,
    /// The commander's own world: nushell, rune, the prompt.
    Com,
    /// The robot: live-loaded prysm cells (its landing and other pages).
    Robot,
    /// Money: balance, send, events, sense (MoneyWallet).
    Sigma,
    /// Which mind runs, and which are on disk.
    Models,
}

/// Shell command forwarded from the commander bar to nushell.
#[derive(Resource, Default)]
pub struct PendingShellCmd(pub Option<String>);

/// Who said a line that lands in com's scrollback.
///
/// com is where cyb keeps its record of what happened, so the other worlds
/// talk into it rather than keeping private logs that nobody scrolls back
/// through. Which side a line sits on is the whole distinction: what you asked
/// for on the left, what the machine answered on the right.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Speaker {
    /// An intent — a button pressed, a transfer asked for.
    User,
    /// What came back: an event, a balance, a receipt.
    System,
}

/// One thing said into com's record.
#[derive(Debug, Clone)]
pub enum ComSay {
    /// A whole line, attributed.
    Line(Speaker, String),
    /// A quiet fact about the session — a transition, a system aside.
    /// Rendered dim, full-width, unattributed.
    Note(String),
    /// A reply is about to arrive in pieces; open a row for it.
    StreamStart,
    /// The next piece of the open reply.
    StreamDelta(String),
    /// The reply is complete. The final text replaces whatever streamed in,
    /// because the stream is raw generation and the final form is cleaned.
    StreamEnd(String),
}

/// Lines waiting to be written into com's scrollback from another world.
///
/// A queue rather than a single slot: one press of `send` produces an intent
/// and then several events, and they must all arrive, in order.
#[derive(Resource, Default)]
pub struct ComInbox(pub Vec<ComSay>);

impl ComInbox {
    pub fn say(&mut self, who: Speaker, line: impl Into<String>) {
        self.0.push(ComSay::Line(who, line.into()));
    }

    /// Close an open stream with its final text. The deltas were display;
    /// this replaces them as the line of record.
    pub fn finish_stream(&mut self, text: impl Into<String>) {
        self.0.push(ComSay::StreamEnd(text.into()));
    }
}

/// A short line announcing that something finished, shown under the address
/// bar and then forgotten.
///
/// The record of what happened lives in com. This is the other half of that
/// split: you should not have to be looking at com, or at the world you acted
/// in, to learn that the thing you asked for is done.
#[derive(Resource, Default)]
pub struct Notice {
    pub text: String,
    /// Seconds left before it fades. Zero means nothing is showing.
    pub ttl: f32,
}

impl Notice {
    /// How long a notice stays up. Long enough to read a short sentence,
    /// short enough that it is gone before it becomes furniture.
    pub const LIFETIME: f32 = 3.5;

    /// A notice is one line on one row. Anything longer is a log entry, and
    /// com already has those — so it is cut here rather than allowed to run
    /// off both edges of the band.
    const MAX_CHARS: usize = 64;

    pub fn show(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.text = if text.chars().count() > Self::MAX_CHARS {
            let head: String = text.chars().take(Self::MAX_CHARS - 3).collect();
            format!("{head}...")
        } else {
            text
        };
        self.ttl = Self::LIFETIME;
    }
}

/// The one durable cybergraph this cyb runs on — `~/cyb/graph.log`, opened
/// once and shared by every world that reads or writes it.
///
/// sigma casts money signals into it, soma links questions to answers, brain
/// renders it. Give each of those its own `Cell::open` and each holds its own
/// in-memory replay of the same log: appends still interleave safely, but no
/// copy sees another's writes until restart, and "the graph" quietly becomes
/// three graphs. One cell, one lock, one truth.
#[derive(Resource, Clone)]
pub struct SharedCell {
    pub cell: std::sync::Arc<std::sync::Mutex<cyb_core::Cell>>,
    /// Bumped on every write; cheap for readers to watch instead of diffing.
    pub version: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl SharedCell {
    fn open_default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let path = std::path::Path::new(&home).join("cyb").join("graph.log");
        let cell = cyb_core::Cell::open(&path).unwrap_or_else(|_| cyb_core::Cell::ephemeral());
        Self {
            cell: std::sync::Arc::new(std::sync::Mutex::new(cell)),
            version: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn bump(&self) {
        self.version.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn version(&self) -> u64 {
        self.version.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// The pre-identity neuron: `$USER` padded with zeros. Retired as the
/// signing identity — [`identity::Identity`] replaced it — but kept so the
/// chain it accumulated can still be read: history cast before the keypair
/// era lives here, and the record does not forget its own past.
pub fn local_neuron() -> [u8; 32] {
    let host = std::env::var("USER").unwrap_or_else(|_| "cyb".into());
    let mut p = [0u8; 32];
    let bytes = host.as_bytes();
    let n = bytes.len().min(32);
    p[..n].copy_from_slice(&bytes[..n]);
    p[31] = 0x53; // 'S' tag
    p
}

pub struct WorldsPlugin;

impl Plugin for WorldsPlugin {
    fn build(&self, app: &mut App) {
        // `CYB_WORLD=brain|com|robot|sigma` boots straight into a world —
        // for scripted runs and self-shots; people use the tabs.
        let initial = match std::env::var("CYB_WORLD").as_deref() {
            Ok("brain") | Ok("graph") => Some(WorldState::Graph),
            Ok("log") | Ok("com") => Some(WorldState::Com),
            Ok("robot") => Some(WorldState::Robot),
            Ok("sigma") => Some(WorldState::Sigma),
            Ok("models") => Some(WorldState::Models),
            _ => None,
        };
        if let Some(w) = initial {
            app.insert_state(w);
        }
        app.init_state::<WorldState>()
            .init_resource::<PendingShellCmd>()
            .init_resource::<ComInbox>()
            .init_resource::<Notice>()
            .insert_resource(identity::load_or_mint())
            .insert_resource(SharedCell::open_default());
    }
}
