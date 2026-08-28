//! The soma bridge: a question typed into com becomes local inference, and
//! the exchange becomes cyberlinks.
//!
//! The flow, end to end:
//!
//! ```text
//! "? why is the sky blue"          typed anywhere the commander is
//!        │
//!        ▼
//! ComInbox (User, left)            the question enters the record
//!        │
//!        ▼
//! soma-kernel thread               glia runs the model, locally
//!        │
//!        ▼
//! ComInbox (System, right)         the answer enters the record
//!        │
//!        ▼
//! SharedCell::cast                 soma → question → answer, one signal
//! ```
//!
//! The cast is the point of the exercise. An answer that only scrolls by is
//! chat; an answer that lands as `particle(question) → particle(answer)` on
//! the local neuron's chain is knowledge the graph now holds — brain renders
//! it, sync will gossip it, and the same question asked of a different mind
//! can link a competing answer beside it.
//!
//! Desktop-only for now: the model runtime is heavy and the phone carries no
//! weights yet. The `ask` prefix still exists on Android — it answers
//! honestly that this body has no mind aboard.

use bevy::prelude::*;

use super::{local_neuron, ComInbox, Notice, SharedCell, Speaker};

pub struct SomaBridgePlugin;

/// Questions soma is currently thinking about, oldest first. Kept so the
/// answer can be attributed even though soma processes one ask at a time.
#[derive(Resource, Default)]
pub struct SomaPending(pub Vec<String>);

impl Plugin for SomaBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SomaPending>();
        #[cfg(target_os = "macos")]
        {
            app.insert_non_send_resource(soma_kernel::Soma::spawn(
                soma_kernel::SomaConfig::default(),
            ));
            app.add_systems(Update, poll_soma);
            // `SOMA_ASK="..."` asks the moment the app is up — the whole
            // pipeline (wake, think, answer, cast) exercised without a hand
            // on the keyboard. A smoke test that doubles as a demo.
            if let Ok(q) = std::env::var("SOMA_ASK") {
                if !q.trim().is_empty() {
                    app.add_systems(PostStartup, move |world: &mut World| {
                        let q = q.clone();
                        ask(world, &q);
                    });
                }
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
/// on the left, it is something *you* said — and the mind is nudged.
pub fn ask(world: &mut World, question: &str) {
    world
        .resource_mut::<ComInbox>()
        .say(Speaker::User, format!("? {question}"));

    #[cfg(target_os = "macos")]
    {
        world.resource_mut::<SomaPending>().0.push(question.to_string());
        world
            .non_send_resource::<soma_kernel::Soma>()
            .ask(question);
        world.resource_mut::<Notice>().show("soma: thinking...");
    }
    #[cfg(not(target_os = "macos"))]
    {
        world
            .resource_mut::<ComInbox>()
            .say(Speaker::System, "this body carries no mind yet — ask on the desktop");
        world.resource_mut::<Notice>().show("soma is not aboard");
    }
}

/// Receive soma's events: narrate the slow parts, and when the answer lands,
/// put it in the record and cast the links that make it knowledge.
#[cfg(target_os = "macos")]
fn poll_soma(
    soma: NonSend<soma_kernel::Soma>,
    shared: Res<SharedCell>,
    mut pending: ResMut<SomaPending>,
    mut inbox: ResMut<ComInbox>,
    mut notice: ResMut<Notice>,
) {
    // Drain everything that happened since last frame; events are rare and
    // tiny, the loop is almost always empty.
    while let Some(ev) = soma.poll() {
        match ev {
            soma_kernel::SomaEvent::Waking => notice.show("soma: waking (loading model)..."),
            soma_kernel::SomaEvent::Thinking => notice.show("soma: thinking..."),
            soma_kernel::SomaEvent::Answer {
                question,
                answer,
                tokens,
                tok_per_s,
            } => {
                inbox.say(Speaker::System, answer.clone());

                // The exchange becomes graph. One atomic signal:
                //   soma → question   (reachable from the well-known anchor)
                //   question → answer (the knowledge itself)
                let q = soma_kernel::particle_of(&question);
                let a = soma_kernel::particle_of(&answer);
                let anchor = soma_kernel::soma_anchor();
                let neuron = local_neuron();
                let cast = {
                    let mut cell = shared.cell.lock().expect("shared cell poisoned");
                    cell.cast(neuron, [(anchor, q), (q, a)])
                };
                match cast {
                    Ok(_) => {
                        shared.bump();
                        notice.show(format!(
                            "soma: answered ({tokens} tok, {tok_per_s:.0} tok/s) — linked"
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
        }
    }
}


