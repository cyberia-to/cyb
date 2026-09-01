//! The soma bridge: a question typed into com becomes local inference, and
//! the exchange becomes cyberlinks.
//!
//! The flow, end to end:
//!
//! ```text
//! "why is the sky blue"            typed anywhere the commander is
//!        │
//!        ▼
//! ComInbox (User, left)            the question enters the record
//!        │
//!        ▼
//! soma-kernel thread               glia runs the model, locally
//!        │
//!        ▼  token by token
//! ComInbox stream (right)          the answer arrives as it is written
//!        │
//!        ▼
//! SharedCell::cast                 the exchange becomes graph, one signal
//! ```
//!
//! What one exchange casts:
//!
//! ```text
//! previous answer ──► question     the conversation is a chain, not islands
//!        (or soma ──► question     when this opens a new thread)
//! question ──► answer              the exchange itself
//! answer ──► concept, ...          the words that recur in it
//! ```
//!
//! The concepts are the weave. `particle("cybergraph")` is the same 32 bytes
//! whichever exchange mentions it, so two conversations that touch the same
//! idea are connected *through* it — the new knowledge attaches to the graph
//! that is already there, instead of hanging off the anchor in a private
//! star. The thread link makes a session readable as a path; the anchor
//! marks only where threads begin.
//!
//! The mind runs on every body. What differs per platform is the backend
//! glia picks (honeycrisp on Apple, the CPU reference elsewhere) and how
//! weights arrive (fetched in-app on the desktop, pushed to the phone until
//! p2p distribution lands). A body without weights says so when asked.

use bevy::prelude::*;

use super::{identity::Identity, ComInbox, ComSay, Notice, SharedCell, Speaker};

pub struct SomaBridgePlugin;

/// Questions soma is currently thinking about, oldest first. Kept so the
/// answer can be attributed even though soma processes one ask at a time.
#[derive(Resource, Default)]
pub struct SomaPending(pub Vec<String>);

/// Where this session's conversation currently ends: the particle of the
/// last answer. The next question links from here, which is what makes a
/// session a chain the graph can be walked along. In-memory on purpose — a
/// new session is a new thread, hanging off the anchor.
#[derive(Resource, Default)]
pub struct SomaThread(pub Option<[u8; 32]>);

impl Plugin for SomaBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SomaPending>();
        app.init_resource::<SomaThread>();
        app.insert_non_send_resource(soma_kernel::Soma::spawn(
            soma_kernel::SomaConfig::default(),
        ));
        app.add_systems(Update, poll_soma);
        // `SOMA_ASK="..."` asks the moment the app is up — the whole
        // pipeline (wake, think, answer, cast) exercised without a hand
        // on the keyboard. A smoke test that doubles as a demo.
        if let Ok(qs) = std::env::var("SOMA_ASK") {
            let questions: Vec<String> = qs
                .split(";;")
                .map(str::trim)
                .filter(|q| !q.is_empty())
                .map(str::to_string)
                .collect();
            if !questions.is_empty() {
                app.add_systems(PostStartup, move |world: &mut World| {
                    // soma answers serially, so these form one thread.
                    for q in &questions {
                        ask(world, q);
                    }
                });
            }
        }
    }
}

/// Is this line addressed to soma rather than to the shell? `? <q>` and
/// `ask <q>` both count — the commander's own placeholder starts with "ask".
pub fn parse_ask(line: &str) -> Option<&str> {
    let t = line.trim();
    if let Some(rest) = t.strip_prefix('?') {
        let q = rest.trim();
        if !q.is_empty() {
            return Some(q);
        }
    }
    if let Some(rest) = t.strip_prefix("ask ").or_else(|| t.strip_prefix("ask\t")) {
        let q = rest.trim();
        if !q.is_empty() {
            return Some(q);
        }
    }
    None
}

/// Route a question to soma. The question goes into the record immediately —
/// on the left, it is something *you* said — then the graph is asked what it
/// already holds, and the mind is nudged with both.
pub fn ask(world: &mut World, question: &str) {
    world
        .resource_mut::<ComInbox>()
        .say(Speaker::User, question.to_string());

    let context = recall(world, question);
    let recalled = context.len();
    world.resource_mut::<SomaPending>().0.push(question.to_string());
    world
        .non_send_resource::<soma_kernel::Soma>()
        .ask_grounded(question, context);
    world.resource_mut::<Notice>().show(if recalled > 0 {
        format!("soma: thinking (recalled {recalled})...")
    } else {
        "soma: thinking...".to_string()
    });
}

/// What the graph already holds on this question.
///
/// This is the read half of "reasons over a living cybergraph", and it works
/// the same way the write half does: concepts. Every past exchange was cast
/// with links from its answer to its recurring words, and those words hash to
/// the same particles today — so recall is particle overlap, not text search.
/// Deterministic, offline, and it gets better with every exchange cast,
/// because the weave IS the index.
///
/// Both eras of the chain are read — the pre-keypair neuron and the identity
/// — for the same reason com replays both: the record precedes the signer.
fn recall(world: &World, question: &str) -> Vec<String> {
    /// The most exchanges spoken into the prompt. Three keeps prefill short
    /// enough that recall costs tens of milliseconds at 0.6B speeds.
    const RECALL_MAX: usize = 3;
    /// An answer quoted into context is clipped: the point is the fact, not
    /// the essay it arrived in.
    const CLIP: usize = 400;

    let q_concepts: std::collections::HashSet<[u8; 32]> =
        soma_kernel::concepts_of(question, "")
            .iter()
            .map(|c| soma_kernel::particle_of(c))
            .collect();
    if q_concepts.is_empty() {
        return Vec::new();
    }

    let texts = super::content::load();
    let shared = world.resource::<SharedCell>();
    let me = world.resource::<Identity>().neuron;
    let cell = shared.cell.lock().expect("shared cell poisoned");

    // (score, order) — later exchanges win ties, so fresher knowledge is
    // preferred when the graph holds several takes on one concept.
    let mut hits: Vec<(usize, usize, String)> = Vec::new();
    let mut order = 0usize;
    for neuron in [super::local_neuron(), me] {
        let Some(chain) = cell.graph.chains.get(&neuron) else { continue };
        for sig in chain.entries.values() {
            let links = &sig.links;
            if links.len() < 2 || links[1].from != links[0].to {
                continue;
            }
            order += 1;
            let score = links[2..]
                .iter()
                .filter(|l| q_concepts.contains(&l.to))
                .count();
            if score == 0 {
                continue;
            }
            let (Some(q), Some(a)) = (texts.get(&links[0].to), texts.get(&links[1].to))
            else {
                continue;
            };
            let mut a_clip = a.clone();
            if a_clip.len() > CLIP {
                let mut end = CLIP;
                while !a_clip.is_char_boundary(end) {
                    end -= 1;
                }
                a_clip.truncate(end);
                a_clip.push_str("...");
            }
            hits.push((score, order, format!("Q: {q}\nA: {a_clip}")));
        }
    }

    hits.sort_by(|x, y| y.0.cmp(&x.0).then(y.1.cmp(&x.1)));
    hits.truncate(RECALL_MAX);
    hits.into_iter().map(|(_, _, t)| t).collect()
}

/// Receive soma's events: narrate the slow parts, and when the answer lands,
/// put it in the record and cast the links that make it knowledge.
fn poll_soma(
    soma: NonSend<soma_kernel::Soma>,
    shared: Res<SharedCell>,
    who: Res<Identity>,
    mut pending: ResMut<SomaPending>,
    mut thread: ResMut<SomaThread>,
    mut inbox: ResMut<ComInbox>,
    mut notice: ResMut<Notice>,
    mut status: ResMut<crate::worlds::models::MindStatus>,
) {
    // Drain everything that happened since last frame; events are rare and
    // tiny, the loop is almost always empty.
    while let Some(ev) = soma.poll() {
        match ev {
            soma_kernel::SomaEvent::Waking => notice.show("soma: waking (loading model)..."),
            soma_kernel::SomaEvent::Thinking => {
                inbox.0.push(ComSay::StreamStart);
                notice.show("soma: thinking...");
            }
            soma_kernel::SomaEvent::Delta(d) => {
                inbox.0.push(ComSay::StreamDelta(d));
            }
            soma_kernel::SomaEvent::Answer {
                question,
                answer,
                concepts,
                tokens,
                tok_per_s,
            } => {
                // The streamed text was raw generation; the final form is the
                // cleaned answer, and it replaces the stream in place.
                inbox.finish_stream(answer.clone());

                // The exchange becomes graph — see the module doc for the
                // shape. One atomic signal: thread, exchange, weave.
                let q = soma_kernel::particle_of(&question);
                let a = soma_kernel::particle_of(&answer);
                let from = thread.0.unwrap_or_else(soma_kernel::soma_anchor);
                let mut links = vec![(from, q), (q, a)];
                for c in &concepts {
                    links.push((a, soma_kernel::particle_of(c)));
                }
                let n_links = links.len();
                let neuron = who.neuron;
                let cast = {
                    let mut cell = shared.cell.lock().expect("shared cell poisoned");
                    cell.cast(neuron, links)
                };
                match cast {
                    Ok(_) => {
                        shared.bump();
                        thread.0 = Some(a);
                        status.last_tok_per_s = Some(tok_per_s);
                        notice.show(format!(
                            "soma: answered ({tokens} tok, {tok_per_s:.0} tok/s) - {n_links} links"
                        ));
                    }
                    Err(e) => {
                        // The answer stands in the record either way; only the
                        // graph write failed, and that is worth saying plainly.
                        inbox.say(Speaker::System, format!("(link failed: {e:?})"));
                        notice.show("soma: answered, link failed");
                    }
                }
                if !pending.0.is_empty() {
                    pending.0.remove(0);
                }
            }
            soma_kernel::SomaEvent::Error(e) => {
                inbox.say(Speaker::System, format!("soma error: {e}"));
                notice.show("soma: error");
                if !pending.0.is_empty() {
                    pending.0.remove(0);
                }
            }
            // Confirmed by the mind itself, so the models page shows what the
            // thread will actually run — not what the UI hopes it will.
            soma_kernel::SomaEvent::ModelChanged(path) => {
                status.model = Some(path);
                status.last_tok_per_s = None;
            }
        }
    }
}


