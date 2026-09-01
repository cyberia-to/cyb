use std::sync::Arc;
use bevy::prelude::*;

use mir::graph::{Csr, ParticleIndex, Cyberlink};
use mir::bevy::resources::{GpuBuffers, GraphCamera, GraphWorldConfig};
use mir::bevy::world::GraphWorldState;
use prysm::theme;

use super::{SharedCell, WorldState};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H};
use crate::shell::platform::SafeArea;

pub struct GraphBridgePlugin;

impl Plugin for GraphBridgePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BrainIndex>()
            .add_systems(Startup, insert_graph_config)
            .add_systems(
                Update,
                place_labels.run_if(in_state(WorldState::Graph)),
            )
            .add_systems(OnExit(WorldState::Graph), hide_labels)
            // Refresh on entering brain, so links cast since the last visit —
            // sigma's money, soma's answers — are in the picture. mir reads
            // the config in its own OnEnter, which the state sync below
            // triggers a frame after this one runs.
            .add_systems(OnEnter(WorldState::Graph), insert_graph_config)
            .add_systems(Update, (sync_graph_state, sync_camera_inset));
    }
}

/// Rebuild mir's graph from the cell. The cybergraph is the graph brain is
/// *for*; the synthetic constellation only stands in while the cell is still
/// empty, so a fresh cyb has something to orbit.
fn insert_graph_config(
    mut commands: Commands,
    shared: Res<SharedCell>,
    mut index: ResMut<BrainIndex>,
) {
    let axons = shared.cell.lock().expect("shared cell poisoned").axons();
    let csr = if axons.is_empty() {
        // Nothing yet — and honestly nothing, not a demo constellation. The
        // graph seeds itself from use: the first world switch casts the
        // first attention link, so this state survives only until the owner
        // moves. A fake graph taught the eye to ignore the real one.
        *index = BrainIndex::default();
        Csr::empty()
    } else {
        let links: Vec<Cyberlink> = axons
            .iter()
            .map(|&(from, to, weight)| Cyberlink {
                neuron: [0u8; 32],
                from,
                to,
                token: 0,
                amount: weight.max(1) as u128,
                valence: 1,
                block: 1,
            })
            .collect();
        let vocab = ParticleIndex::build(links.iter().copied());

        // The real focusing engine, not a stand-in: tru's φ* over the same
        // links mir will draw, attention seconds and knowledge stakes as
        // conviction, neutral market. Deterministic fixed-point inside;
        // floats only leave for display.
        let focus_by_hash: std::collections::HashMap<[u8; 32], f32> = {
            let tru_links = axons.iter().map(|&(from, to, w)| {
                tru::Link::stake(from, to, w.max(1) as u128)
            });
            let g = tru::FocusingGraph::build(tru_links, &tru::Context::none());
            let result = tru::compute_focusing(&g, &tru::FocusingParams::default());
            g.node_ids()
                .iter()
                .zip(result.focus.iter())
                .map(|(id, fx)| (*id, fx.to_f64() as f32))
                .collect()
        };
        *index = BrainIndex::from_vocab(&vocab, &focus_by_hash);
        Csr::build(links.into_iter(), &vocab)
    };
    info!(
        "brain: graph from {} ({} axons, {} labels)",
        if axons.is_empty() { "synthetic demo" } else { "cybergraph" },
        axons.len(),
        index.labels.iter().flatten().count(),
    );
    commands.insert_resource(GraphWorldConfig { graph: Arc::new(csr) });
}

// ── labels ───────────────────────────────────────────────────────────────────

/// What brain knows about the particles it is showing: their order (the CSR's
/// row order) and, where anything knows the words behind a hash, the words.
///
/// A graph of anonymous spheres proves nothing to the person who just asked a
/// question. The whole point of linking an answer is that you can go to brain
/// and see it — which needs the text back, not the hash.
#[derive(Resource, Default)]
struct BrainIndex {
    labels: Vec<Option<String>>,
    /// tru's φ* focus per particle, same indexing as `labels`. Text in brain
    /// is *earned*: only the particles the graph's own attention ranks
    /// highest get their words drawn. Everything else stays geometry until
    /// you fly close — a name you did not earn is noise you cannot unsee.
    focus: Vec<f32>,
}

/// Longest label drawn in the graph. Enough to recognise the sentence you
/// typed; the full text lives in com's record.
const LABEL_CHARS: usize = 40;

impl BrainIndex {
    fn from_vocab(
        vocab: &ParticleIndex,
        focus_by_hash: &std::collections::HashMap<[u8; 32], f32>,
    ) -> Self {
        let sidecar = super::content::load();
        let (mut labels, mut focus) = (Vec::new(), Vec::new());
        for hash in vocab.anchor() {
            labels.push(if let Some(text) = sidecar.get(hash) {
                Some(shorten(text))
            } else {
                decode_ascii_particle(hash)
            });
            focus.push(focus_by_hash.get(hash).copied().unwrap_or(0.0));
        }
        Self { labels, focus }
    }

    /// The φ* floor a particle must clear for its label to be drawn: the
    /// K-th highest focus among particles that have text at all.
    fn label_floor(&self, k: usize) -> f32 {
        let mut ranked: Vec<f32> = self
            .labels
            .iter()
            .zip(self.focus.iter())
            .filter(|(l, _)| l.is_some())
            .map(|(_, f)| *f)
            .collect();
        if ranked.len() <= k {
            return f32::NEG_INFINITY;
        }
        ranked.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        ranked[k - 1]
    }
}

/// How many particles may wear their names at once. Enough to orient by,
/// few enough that each is readable — the rest of the graph speaks through
/// shape, size and pull until focus earns it a caption.
const LABEL_BUDGET: usize = 12;

fn shorten(text: &str) -> String {
    let mut s: String = text.chars().take(LABEL_CHARS).collect();
    if text.chars().count() > LABEL_CHARS {
        s.push_str("...");
    }
    s
}

/// sigma names particles by padding ASCII with zeros ("PUSSY", "bob"); those
/// hashes *are* their labels, no sidecar needed.
fn decode_ascii_particle(hash: &[u8; 32]) -> Option<String> {
    let end = hash.iter().position(|&b| b == 0)?;
    if end == 0 || !hash[end..].iter().all(|&b| b == 0) {
        return None;
    }
    let head = &hash[..end];
    head.iter()
        .all(|&b| b.is_ascii_graphic() || b == b' ')
        .then(|| String::from_utf8_lossy(head).into_owned())
}

#[derive(Component)]
struct ParticleLabel(usize);

/// Pin each labelled particle's words under it, every frame.
///
/// mir paints the graph into an image; the words are Bevy UI on top. The
/// projection is the same one the paint kernel uses, done here in logical
/// pixels because that is what UI nodes are measured in.
fn place_labels(
    mut commands: Commands,
    index: Res<BrainIndex>,
    gpu: Option<Res<GpuBuffers>>,
    cam: Option<Res<GraphCamera>>,
    mut existing: Query<(Entity, &ParticleLabel, &mut Node, &mut Text, &mut Visibility)>,
) {
    let (Some(gpu), Some(cam)) = (gpu, cam) else { return };
    let m = cam.view_proj();
    let [lw, lh] = cam.input_viewport;

    // Where each labelled particle lands on screen this frame.
    let floor = index.label_floor(LABEL_BUDGET);
    let mut spots: std::collections::HashMap<usize, Option<(f32, f32)>> =
        std::collections::HashMap::new();
    for (i, label) in index.labels.iter().enumerate() {
        if label.is_none() {
            continue;
        }
        // Focus decides who speaks. tru ranked this graph; the label budget
        // goes to the particles attention actually flows through.
        if index.focus.get(i).copied().unwrap_or(0.0) < floor {
            continue;
        }
        let base = i * 3;
        if base + 2 >= gpu.pos_cpu.len() {
            spots.insert(i, None);
            continue;
        }
        let (x, y, z) = (gpu.pos_cpu[base], gpu.pos_cpu[base + 1], gpu.pos_cpu[base + 2]);
        let w = m[0][3] * x + m[1][3] * y + m[2][3] * z + m[3][3];
        if w <= 0.0 {
            // Behind the camera; the label would project to nonsense.
            spots.insert(i, None);
            continue;
        }
        let cx = (m[0][0] * x + m[1][0] * y + m[2][0] * z + m[3][0]) / w;
        let cy = (m[0][1] * x + m[1][1] * y + m[2][1] * z + m[3][1]) / w;
        let sx = (cx * 0.5 + 0.5) * lw;
        let sy = (1.0 - (cy * 0.5 + 0.5)) * lh;
        let on_screen = (-40.0..lw + 40.0).contains(&sx) && (0.0..lh).contains(&sy);
        spots.insert(i, on_screen.then_some((sx, sy)));
    }

    // Move the labels that exist; note which particles still need one.
    for (_, label, mut node, mut text, mut vis) in &mut existing {
        match spots.remove(&label.0) {
            Some(Some((sx, sy))) => {
                node.left = Val::Px(sx + 8.0);
                node.top = Val::Px(sy + 6.0);
                *vis = Visibility::Visible;
                if let Some(Some(want)) = index.labels.get(label.0) {
                    if text.0 != *want {
                        text.0 = want.clone();
                    }
                }
            }
            _ => *vis = Visibility::Hidden,
        }
    }

    // First sighting of a particle: give it its label.
    for (i, spot) in spots {
        let Some(Some(label)) = index.labels.get(i) else { continue };
        let Some((sx, sy)) = spot else { continue };
        commands.spawn((
            ParticleLabel(i),
            Text::new(label.clone()),
            TextFont { font_size: 11.0, ..default() },
            TextColor(theme::TEXT_DIM),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(sx + 8.0),
                top: Val::Px(sy + 6.0),
                ..default()
            },
        ));
    }
}

fn hide_labels(mut q: Query<&mut Visibility, With<ParticleLabel>>) {
    for mut v in &mut q {
        *v = Visibility::Hidden;
    }
}



/// Tell mir which screen bands the chrome owns, so a thumb on the tab strip
/// or in the commander never reaches the camera.
fn sync_camera_inset(cam: Option<ResMut<GraphCamera>>, safe: Res<SafeArea>) {
    // Optional: mir only inserts the camera once the graph world runs, and
    // this system ticks from the first frame.
    let Some(mut cam) = cam else { return };
    let inset = [
        CHROME_TOP_H + safe.top,
        CHROME_BOTTOM_H + safe.bottom,
        0.0,
        0.0,
    ];
    if cam.input_inset != inset {
        cam.input_inset = inset;
    }
}

fn sync_graph_state(
    world_state:     Res<State<WorldState>>,
    graph_state_cur: Res<State<GraphWorldState>>,
    mut graph_state: ResMut<NextState<GraphWorldState>>,
) {
    let target = if *world_state.get() == WorldState::Graph {
        GraphWorldState::Active
    } else {
        GraphWorldState::Inactive
    };
    if *graph_state_cur.get() != target {
        graph_state.set(target);
    }
}


