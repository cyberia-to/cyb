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
            .init_resource::<BrainStats>()
            .add_systems(OnEnter(WorldState::Graph), spawn_hud)
            .add_systems(OnExit(WorldState::Graph), despawn_hud)
            .add_systems(
                Update,
                refresh_hud.run_if(in_state(WorldState::Graph)),
            )
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
    mut stats: ResMut<BrainStats>,
) {
    let axons = shared.cell.lock().expect("shared cell poisoned").axons();
    let mut values: Option<std::sync::Arc<mir::epoch::GraphValues>> = None;
    *stats = BrainStats::default();
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
        // tru's full tri-kernel run: φ* plus its diffusion / springs / heat
        // decomposition and the syntropy of the whole distribution. One
        // computation feeds four consumers — label rank, particle radius,
        // particle colour, and the HUD.
        let tru_links = axons.iter().map(|&(from, to, w)| {
            tru::Link::stake(from, to, w.max(1) as u128)
        });
        let g = tru::FocusingGraph::build(tru_links, &tru::Context::none());
        let result = tru::compute_focusing(&g, &tru::FocusingParams::default());

        let mut by_hash: std::collections::HashMap<[u8; 32], (f32, [f32; 3])> =
            std::collections::HashMap::new();
        for (i, id) in g.node_ids().iter().enumerate() {
            let f = result.focus.get(i).map(|x| x.to_f64() as f32).unwrap_or(0.0);
            let k = [
                result.diffusion.get(i).map(|x| x.to_f64() as f32).unwrap_or(0.0),
                result.springs.get(i).map(|x| x.to_f64() as f32).unwrap_or(0.0),
                result.heat.get(i).map(|x| x.to_f64() as f32).unwrap_or(0.0),
            ];
            by_hash.insert(*id, (f, k));
        }
        let focus_by_hash: std::collections::HashMap<[u8; 32], f32> =
            by_hash.iter().map(|(h, (f, _))| (*h, *f)).collect();

        // Per-particle values in the CSR's own row order, for mir.
        let mut gv = mir::epoch::GraphValues::default();
        for hash in vocab.anchor() {
            let (f, k) = by_hash.get(hash).copied().unwrap_or((0.0, [0.0; 3]));
            gv.focus.push(f);
            gv.kernel.push(k);
        }
        values = Some(std::sync::Arc::new(gv));

        // The HUD's numbers, computed once here where everything is at hand.
        let world_particles: std::collections::HashSet<[u8; 32]> =
            [WorldState::Graph, WorldState::Com, WorldState::Robot, WorldState::Sigma, WorldState::Models]
                .into_iter()
                .map(|w| super::content::particle_of(super::attention::world_name(w)))
                .collect();
        let attention_secs: u64 = axons
            .iter()
            .filter(|(f, t, _)| world_particles.contains(f) && world_particles.contains(t))
            .map(|(_, _, w)| w)
            .sum();
        stats.particles = vocab.len();
        stats.axons = axons.len();
        stats.stake = axons.iter().map(|(_, _, w)| *w).sum();
        stats.attention_secs = attention_secs;
        stats.syntropy = result.syntropy.to_f64() as f32;
        stats.kernel_split = {
            let (mut d, mut sp, mut h) = (0f64, 0f64, 0f64);
            for (_, k) in by_hash.values() {
                d += k[0] as f64;
                sp += k[1] as f64;
                h += k[2] as f64;
            }
            let total = (d + sp + h).max(1e-12);
            [(d / total) as f32, (sp / total) as f32, (h / total) as f32]
        };
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        stats.graph_bytes = std::fs::metadata(
            std::path::Path::new(&home).join("cyb").join("graph.log"),
        )
        .map(|m| m.len())
        .unwrap_or(0);
        // Top particles by focus, with the words behind them when known.
        let texts = super::content::load();
        let mut ranked: Vec<([u8; 32], f32)> =
            focus_by_hash.iter().map(|(h, f)| (*h, *f)).collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        stats.top = ranked
            .into_iter()
            .take(5)
            .map(|(h, f)| {
                let name = texts
                    .get(&h)
                    .map(|t| {
                        let mut s: String = t.chars().take(18).collect();
                        if t.chars().count() > 18 {
                            s.push_str("..");
                        }
                        s
                    })
                    .unwrap_or_else(|| "?".into());
                (name, f)
            })
            .collect();

        *index = BrainIndex::from_vocab(&vocab, &focus_by_hash);
        Csr::build(links.into_iter(), &vocab)
    };
    info!(
        "brain: graph from {} ({} axons, {} labels)",
        if axons.is_empty() { "synthetic demo" } else { "cybergraph" },
        axons.len(),
        index.labels.iter().flatten().count(),
    );
    commands.insert_resource(GraphWorldConfig { graph: Arc::new(csr), values });
}

// ── labels ───────────────────────────────────────────────────────────────────

/// What brain knows about the particles it is showing: their order (the CSR's
/// row order) and, where anything knows the words behind a hash, the words.
///
/// A graph of anonymous spheres proves nothing to the person who just asked a
/// question. The whole point of linking an answer is that you can go to brain
/// and see it — which needs the text back, not the hash.
/// What the graph *is* right now, in numbers: the HUD's data. Filled where
/// the tri-kernel runs, shown in brain's corner — feeling the graph means
/// seeing its size, its weight, and where its focus pools, at a glance,
/// every time it changes.
#[derive(Resource, Default)]
pub struct BrainStats {
    pub particles: usize,
    pub axons: usize,
    pub graph_bytes: u64,
    /// Sum of all link amounts — stake in the broadest sense.
    pub stake: u64,
    /// The slice of stake that is measured attention: world → world dwell.
    pub attention_secs: u64,
    /// J(φ*) — how far focus has pulled away from uniform. Zero is a graph
    /// nobody has looked at; rising syntropy is a graph forming opinions.
    pub syntropy: f32,
    /// Global tri-kernel split (diffusion, springs, heat), summing to one.
    pub kernel_split: [f32; 3],
    /// Top particles by φ*, with their words where known.
    pub top: Vec<(String, f32)>,
}

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

// ── the HUD ─────────────────────────────────────────────────────────────────

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct HudText;

/// An ASCII bar, `width` characters at `frac` full. The font has no blocks
/// worth trusting; `=` and `.` are everywhere and read instantly.
fn bar(frac: f32, width: usize) -> String {
    let filled = ((frac.clamp(0.0, 1.0) * width as f32).round() as usize).min(width);
    format!("{}{}", "=".repeat(filled), ".".repeat(width - filled))
}

fn hud_text(stats: &BrainStats) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "{} particles   {} axons   {:.1} KB\n",
        stats.particles,
        stats.axons,
        stats.graph_bytes as f64 / 1024.0
    ));
    out.push_str(&format!(
        "stake {}   attention {}s   J {:.3}\n",
        stats.stake, stats.attention_secs, stats.syntropy
    ));
    let [d, s_, h] = stats.kernel_split;
    out.push_str(&format!(
        "D {} {:>2.0}%   S {} {:>2.0}%   H {} {:>2.0}%\n",
        bar(d, 8),
        d * 100.0,
        bar(s_, 8),
        s_ * 100.0,
        bar(h, 8),
        h * 100.0
    ));
    if !stats.top.is_empty() {
        out.push('\n');
        let max = stats.top.first().map(|(_, f)| *f).unwrap_or(1.0).max(1e-9);
        for (name, f) in &stats.top {
            out.push_str(&format!("{} {:<20}\n", bar(f / max, 10), name));
        }
    }
    out
}

fn spawn_hud(mut commands: Commands, stats: Res<BrainStats>) {
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(CHROME_TOP_H + 10.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            GlobalZIndex(5),
        ))
        .with_children(|hud| {
            hud.spawn((
                HudText,
                Text::new(hud_text(&stats)),
                TextFont { font_size: 11.0, ..default() },
                TextColor(prysm::theme::TEXT_DIM),
            ));
        });
}

fn despawn_hud(mut commands: Commands, q: Query<Entity, With<HudRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn refresh_hud(stats: Res<BrainStats>, mut q: Query<&mut Text, With<HudText>>) {
    if !stats.is_changed() {
        return;
    }
    for mut t in &mut q {
        **t = hud_text(&stats);
    }
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


