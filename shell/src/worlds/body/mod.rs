//! body — the machine itself, as a world.
//!
//! The main page of cyb: what this body is doing with its cores, its GPU,
//! its memory and its wire, and what that work earns. Telemetry is the
//! OS's own counters. Proving is zheng. Seer is the structure miner —
//! existing files, never new ones. The only declared (not measured)
//! number on the page, the PUSSY rate, says so out loud.

pub mod chainsync;
pub mod networks;
pub mod prover;
pub mod relay;
pub mod seer;
pub mod telemetry;

use bevy::prelude::*;
use prysm::theme;

use super::WorldState;
use super::cell;
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};
use rune_ast::Noun;
use rune_interp::{Host, InterpError};

pub struct BodyWorldPlugin;

/// The network hub, exposed as its own resource so the commander (com)
/// can drive the configurator without reaching into body internals.
#[derive(Resource, Clone)]
pub struct BodyLinkHub(pub networks::NetHub);

/// Live handles to the samplers; created once at build.
#[derive(Resource)]
struct BodyLink {
    telemetry: telemetry::Telemetry,
    prover: prover::Prover,
    seer: seer::Seer,
    pub(crate) nets: networks::NetHub,
    relay: relay::Relay,
    chainsync: chainsync::ChainSync,
}

/// The snapshot the page renders. Rewritten once a second while the body
/// world is open; every rewrite repaints via change detection.
#[derive(Resource, Default)]
struct BodyView {
    vitals: telemetry::Vitals,
    prover: prover::ProverStat,
    prover_intensity: String,
    seer: seer::SeerStat,
    seer_intensity: String,
    checkpoint_in: Option<u64>,
    chain: crate::worlds::sigma::chain::ChainMoneyState,
    nets: Vec<networks::NetState>,
    relayed: u64,
    relay_pending: u64,
    absorbed: u64,
}

#[derive(Component)]
struct BodyRoot;

#[derive(Component)]
struct BodyTables;

/// The prove/stop lever on the zheng card.
#[derive(Component)]
struct ProveButton;

#[derive(Component)]
struct SeerButton;

#[derive(Component)]
struct SeerIntensityButton(&'static str);

/// The prover fleet's duty lever: max (all cores) / eco (half) / min (one).
#[derive(Component)]
struct ProverIntensityButton(&'static str);

impl Plugin for BodyWorldPlugin {
    fn build(&self, app: &mut App) {
        let nets = networks::NetHub::start();
        app.insert_resource(BodyLinkHub(nets.clone()));
        // The relay tails the one shared cell — WorldsPlugin opened it
        // before this plugin was added.
        let shared = app.world().resource::<super::SharedCell>().clone();
        app.insert_resource(BodyLink {
            telemetry: telemetry::Telemetry::start(),
            prover: prover::Prover::start(),
            seer: seer::Seer::start(),
            relay: relay::Relay::start(shared.clone(), nets.clone()),
            chainsync: chainsync::ChainSync::start(shared, nets.clone()),
            nets,
        })
        .init_resource::<BodyView>()
        .init_resource::<ProofMeter>()
        .add_systems(OnEnter(WorldState::Body), enter_body)
        .add_systems(
            Update,
            (
                tick_view,
                paint_live,
                paint_tables,
                paint_seer,
                rebuild_on_change,
                handle_prove_press,
                handle_prover_intensity_press,
                handle_seer_press,
                handle_seer_intensity_press,
            )
                .run_if(in_state(WorldState::Body)),
        );
        app.add_systems(Startup, (resume_proving, resume_seer));
        // Checkpoints tick in every world — proving does not stop when
        // the body page is closed, and neither does its meter.
        app.add_systems(Update, proof_checkpoint);
    }
}

/// The prover's standing order.
fn proving_wanted_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("proving")
}

fn resume_proving(
    link: Res<BodyLink>,
    shared: Res<super::SharedCell>,
    mut meter: ResMut<ProofMeter>,
    who: Res<super::identity::Identity>,
    mut inbox: ResMut<super::ComInbox>,
) {
    let wanted = std::fs::read_to_string(proving_wanted_file())
        .map(|s| s.trim() == "on")
        .unwrap_or(false);
    if wanted && !link.prover.is_running() {
        let axons = shared.cell.lock().expect("shared cell poisoned").axons();
        link.prover.prove(axons, link.nets.clone());
        cast_prove_start(&mut meter, &link, &shared, &who, &mut inbox);
    }
}

fn resume_seer(
    link: Res<BodyLink>,
    shared: Res<super::SharedCell>,
    who: Res<super::identity::Identity>,
    mut inbox: ResMut<super::ComInbox>,
) {
    if seer::wanted() && !link.seer.is_running() {
        link.seer.mine(shared.clone(), who.neuron);
        inbox.0.push(super::ComSay::Note(
            "seer began - linking existing files".into(),
        ));
    }
}

/// Every two minutes of proving, the new tickets become one weighted
/// link — `zheng -> pussy`, weight = tickets since the last checkpoint —
/// cast into the local cell like any other signal. The relay carries it,
/// one signal one block: a body that only proves still moves the chain,
/// and the proven work is metered ON the chain instead of only in a file.
///
/// The meter arms the moment the fleet sails (prove press / resume), and
/// the start itself is cast — the chain answers within one relay pass,
/// so pressing prove visibly moves h in seconds, not minutes.
const CHECKPOINT_EVERY_S: f32 = 120.0;

/// The proving meter: armed while the fleet rows.
#[derive(Resource, Default)]
pub(crate) struct ProofMeter {
    wait: f32,
    last_count: u64,
    armed: bool,
}

impl ProofMeter {
    /// Seconds until the next checkpoint cast (for the card).
    fn next_in(&self) -> u64 {
        (CHECKPOINT_EVERY_S - self.wait).max(0.0) as u64
    }
}

/// Arm the meter and cast the departure: proving began, weight 1.
fn cast_prove_start(
    meter: &mut ProofMeter,
    link: &BodyLink,
    shared: &super::SharedCell,
    who: &super::identity::Identity,
    inbox: &mut super::ComInbox,
) {
    meter.wait = 0.0;
    meter.last_count = link.prover.stat.lock().map(|s| s.lifetime).unwrap_or(0);
    meter.armed = true;
    super::content::remember("zheng");
    super::content::remember("pussy");
    let cast = {
        let mut cell = shared.cell.lock().expect("shared cell poisoned");
        cell.cast_weighted(
            who.neuron,
            [(
                super::content::particle_of("zheng"),
                super::content::particle_of("pussy"),
                1,
            )],
        )
    };
    if cast.is_ok() {
        shared.bump();
        inbox.0.push(super::ComSay::Note(
            "proving began - cast to the chain".into(),
        ));
    }
}

fn proof_checkpoint(
    time: Res<Time>,
    mut meter: ResMut<ProofMeter>,
    link: Res<BodyLink>,
    shared: Res<super::SharedCell>,
    who: Res<super::identity::Identity>,
    mut inbox: ResMut<super::ComInbox>,
) {
    if !meter.armed || !link.prover.is_running() {
        return;
    }
    meter.wait += time.delta_secs();
    if meter.wait < CHECKPOINT_EVERY_S {
        return;
    }
    meter.wait = 0.0;
    let lifetime = link.prover.stat.lock().map(|s| s.lifetime).unwrap_or(0);
    let delta = lifetime.saturating_sub(meter.last_count);
    if delta == 0 {
        return;
    }
    meter.last_count = lifetime;
    super::content::remember("zheng");
    super::content::remember("pussy");
    let cast = {
        let mut cell = shared.cell.lock().expect("shared cell poisoned");
        cell.cast_weighted(
            who.neuron,
            [(
                super::content::particle_of("zheng"),
                super::content::particle_of("pussy"),
                delta,
            )],
        )
    };
    match cast {
        Ok(_) => {
            shared.bump();
            inbox.0.push(super::ComSay::Note(format!(
                "proof checkpoint: {delta} tickets -> chain"
            )));
        }
        Err(e) => warn!("body: checkpoint cast failed: {e:?}"),
    }
}

/// Once a second, copy the live counters into the view. The page repaints
/// on the resource write; sub-second flicker would just burn battery.
fn tick_view(
    time: Res<Time>,
    mut timer: Local<f32>,
    link: Res<BodyLink>,
    meter: Res<ProofMeter>,
    money: Option<Res<crate::worlds::sigma::chain::ChainMoney>>,
    mut view: ResMut<BodyView>,
) {
    *timer += time.delta_secs();
    if *timer < 1.0 && !view.is_added() {
        return;
    }
    *timer = 0.0;
    view.vitals = link.telemetry.snapshot();
    view.prover = link
        .prover
        .stat
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    view.prover_intensity = prover::intensity();
    view.seer = link.seer.stat.lock().map(|s| s.clone()).unwrap_or_default();
    view.seer_intensity = seer::intensity();
    view.checkpoint_in = meter.armed.then(|| meter.next_in());
    if let Some(money) = money {
        view.chain = money.snapshot();
    }
    view.nets = link.nets.snapshot();
    view.relayed = link.relay.sent.load(std::sync::atomic::Ordering::Relaxed);
    view.relay_pending = link
        .relay
        .pending
        .load(std::sync::atomic::Ordering::Relaxed);
    view.absorbed = link
        .chainsync
        .absorbed
        .load(std::sync::atomic::Ordering::Relaxed);
}

fn enter_body(
    commands: Commands,
    view: Res<BodyView>,
    link: Res<BodyLink>,
    mut worlds: Query<(&crate::worlds::WorldUi, &mut Visibility)>,
) {
    if crate::worlds::reveal_world(WorldState::Body, &mut worlds) {
        return;
    }
    build_page(commands, view, link);
}

#[derive(Component, Clone, Copy)]
#[allow(dead_code)]
enum BodyStat {
    Cpu,
    CpuTop,
    Gpu,
    Mem,
    NetIo,
}

fn cpu_line(view: &BodyView) -> String {
    let v = &view.vitals;
    let watts = if v.cpu_mw > 0 {
        format!("  {:.1} W", v.cpu_mw as f32 / 1000.0)
    } else {
        String::new()
    };
    let cpu_task = match (view.prover.running, view.seer.running) {
        (true, true) => "   zheng + seer",
        (true, false) => "   zheng (proving)",
        (false, true) => "   seer (linking)",
        (false, false) => "",
    };
    format!(
        "cpu     {}  {:>3.0}%{}{}",
        bar(v.cpu_pct / 100.0),
        v.cpu_pct,
        watts,
        cpu_task
    )
}

fn cpu_top_line(view: &BodyView) -> String {
    let who = view
        .vitals
        .top
        .iter()
        .map(|t| format!("{} {:.0}%", t.name, t.cpu_pct))
        .collect::<Vec<_>>()
        .join("   ");
    format!("        {who}")
}

fn gpu_line(view: &BodyView) -> String {
    let v = &view.vitals;
    let watts = if v.gpu_mw > 0 {
        format!("  {:.1} W", v.gpu_mw as f32 / 1000.0)
    } else {
        String::new()
    };
    format!(
        "gpu     {}  {:>3.0}%{}",
        bar(v.gpu_pct / 100.0),
        v.gpu_pct,
        watts
    )
}

fn mem_line(view: &BodyView) -> String {
    let v = &view.vitals;
    format!(
        "memory  {}  {:.1} / {:.0} GB",
        bar(v.mem_used as f32 / v.mem_total as f32),
        gb(v.mem_used),
        gb(v.mem_total)
    )
}

fn net_io_line(view: &BodyView) -> String {
    let v = &view.vitals;
    format!(
        "network  down {}   up {}",
        rate(v.net_rx_bps),
        rate(v.net_tx_bps)
    )
}

struct BodyHost {
    resources: Noun,
    processes: Noun,
}

impl Host for BodyHost {
    fn perform(&mut self, act: u64, args: &Noun, _caps: &Noun) -> Result<Noun, InterpError> {
        if !cell::act_is_query(act) {
            return Ok(Noun::Atom(0));
        }
        match cell::query_name(args).as_str() {
            "resources" => Ok(self.resources.clone()),
            "processes" => Ok(self.processes.clone()),
            other => Err(cell::unknown_query(other)),
        }
    }
}

fn body_host(view: &BodyView) -> BodyHost {
    let v = &view.vitals;
    let mut resources = Vec::new();
    resources.push(cell::row(&[
        "cpu",
        &format!("{:.0}%", v.cpu_pct),
        "100",
        &if v.cpu_mw > 0 {
            format!("{:.1} W", v.cpu_mw as f32 / 1000.0)
        } else {
            String::new()
        },
    ]));
    if v.gpu_pct >= 0.0 {
        resources.push(cell::row(&[
            "gpu",
            &format!("{:.0}%", v.gpu_pct),
            "100",
            &if v.gpu_mw > 0 {
                format!("{:.1} W", v.gpu_mw as f32 / 1000.0)
            } else {
                String::new()
            },
        ]));
    }
    if v.mem_total > 0 {
        resources.push(cell::row(&[
            "memory",
            &cell::exact(v.mem_used),
            &cell::exact(v.mem_total),
            "B",
        ]));
    }
    resources.push(cell::row(&[
        "network",
        &format!("{:.0}", v.net_rx_bps),
        &format!("{:.0}", v.net_tx_bps),
        "B/s down / up",
    ]));
    let processes = v
        .top
        .iter()
        .map(|t| {
            cell::row(&[
                &t.name,
                &format!("{:.0}%", t.cpu_pct),
                &format!("{:.0} MB", t.rss_mb),
            ])
        })
        .collect();
    BodyHost {
        resources: cell::list(resources),
        processes: cell::list(processes),
    }
}

fn fill_body_tables(commands: &mut Commands, slot: Entity, view: &BodyView) {
    let mut host = body_host(view);
    match cell::load("body").and_then(|src| cell::eval(&src, &mut host)) {
        Ok(chunks) => cell::dispatch_page(commands, slot, &chunks),
        Err(e) => {
            commands.spawn((
                Text::new(e),
                TextFont {
                    font_size: theme::CAPTION,
                    ..default()
                },
                TextColor(theme::ACID_RED),
                ChildOf(slot),
            ));
        }
    }
}

fn paint_tables(
    mut commands: Commands,
    view: Res<BodyView>,
    slot: Query<Entity, With<BodyTables>>,
    children: Query<&Children>,
) {
    if !view.is_changed() {
        return;
    }
    let Ok(slot) = slot.single() else {
        return;
    };
    if let Ok(kids) = children.get(slot) {
        for c in kids.iter() {
            commands.entity(c).despawn();
        }
    }
    fill_body_tables(&mut commands, slot, &view);
}

fn paint_live(view: Res<BodyView>, mut q: Query<(&BodyStat, &mut Text)>) {
    if !view.is_changed() {
        return;
    }
    for (stat, mut t) in &mut q {
        **t = match stat {
            BodyStat::Cpu => cpu_line(&view),
            BodyStat::CpuTop => cpu_top_line(&view),
            BodyStat::Gpu => gpu_line(&view),
            BodyStat::Mem => mem_line(&view),
            BodyStat::NetIo => net_io_line(&view),
        };
    }
}

fn rebuild_on_change(
    mut commands: Commands,
    view: Res<BodyView>,
    link: Res<BodyLink>,
    roots: Query<Entity, With<BodyRoot>>,
    mut last: Local<Option<(usize, bool, bool)>>,
) {
    // Numbers tick every second; tearing the tree down to rewrite them
    // is the jerk on the phone. Only rebuild when the page's shape moves.
    let key = (view.nets.len(), view.prover.running, view.seer.running);
    if last.is_none() && !roots.is_empty() {
        *last = Some(key);
        return;
    }
    if Some(key) == *last || roots.is_empty() {
        return;
    }
    *last = Some(key);
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_page(commands, view.into(), link.into());
}

/// A declared conversion rate from `~/cyb/rates.toml` (`key = value`).
/// The file is written with defaults on first read so there is something
/// to edit; PUSSY has no market yet and the page never pretends otherwise.
pub fn declared_rate(key: &str, default: f64) -> f64 {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let path = std::path::Path::new(&home).join("cyb").join("rates.toml");
    match std::fs::read_to_string(&path) {
        Ok(text) => match text.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().parse::<f64>().ok())?
        }) {
            Some(v) => v,
            None => {
                // The file predates this rate: append it so the owner can
                // see and edit what the page is using.
                let _ = std::fs::write(&path, format!("{text}{key} = {default}\n"));
                default
            }
        },
        Err(_) => {
            let template = "# Declared conversion rates for the body page.\n\
                            # PUSSY has no market yet; these rates are yours to declare.\n\
                            [pussy]\n\
                            per_erg = 1000000\n\
                            per_proof = 1\n";
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(&path, template);
            default
        }
    }
}

/// A ten-slot text meter: `[====......]`.
fn bar(frac: f32) -> String {
    let filled = (frac.clamp(0.0, 1.0) * 10.0).round() as usize;
    format!("[{}{}]", "=".repeat(filled), ".".repeat(10 - filled))
}

fn gb(bytes: u64) -> f64 {
    bytes as f64 / 1e9
}

/// Seconds as a compact age: 47s, 12m, 3h.
fn ago(secs: u64) -> String {
    if secs >= 3600 {
        format!("{}h", secs / 3600)
    } else if secs >= 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

fn human_size(bytes: u64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1e6)
    } else if bytes >= 1_000 {
        format!("{:.1} KB", bytes as f64 / 1e3)
    } else {
        format!("{bytes} B")
    }
}

fn rate(bps: f64) -> String {
    if bps >= 1e6 {
        format!("{:.1} MB/s", bps / 1e6)
    } else if bps >= 1e3 {
        format!("{:.0} KB/s", bps / 1e3)
    } else {
        format!("{bps:.0} B/s")
    }
}

fn build_page(mut commands: Commands, view: Res<BodyView>, _link: Res<BodyLink>) {
    let root = commands
        .spawn((
            BodyRoot,
            crate::worlds::WorldUi(WorldState::Body),
            ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(CHROME_TOP_H),
                bottom: Val::Px(CHROME_BOTTOM_H),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
        ))
        .id();

    let page = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(theme::MEASURE),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                row_gap: Val::Px(theme::G),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            ChildOf(root),
        ))
        .id();

    let text = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            ChildOf(parent),
        ));
    };

    text(
        &mut commands,
        page,
        "body".into(),
        theme::H2,
        theme::TEXT_PRIMARY,
    );

    let tables = commands
        .spawn((
            BodyTables,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(theme::G),
                ..default()
            },
            ChildOf(page),
        ))
        .id();
    fill_body_tables(&mut commands, tables, &view);

    // ── networks: the chains this body follows ──────────────────────────
    if !view.nets.is_empty() {
        commands.spawn((
            Text::new("networks"),
            TextFont {
                font_size: theme::CAPTION,
                ..default()
            },
            TextColor(theme::TEXT_DIM),
            Node {
                margin: UiRect::top(Val::Px(theme::G * 2.0)),
                ..default()
            },
            ChildOf(page),
        ));
        for n in &view.nets {
            let (line, color) = if n.height > 0 {
                let step = n
                    .last_sync
                    .map(|t| format!("{}s", t.elapsed().as_secs()))
                    .unwrap_or_default();
                // The step is our probe; the block is the chain's pulse.
                let block = match n.last_advance {
                    Some(t) => format!("block {} ago", ago(t.elapsed().as_secs())),
                    None => "no new block while watching".to_string(),
                };
                // Watchdog: a probe older than three cadences is a stall,
                // whatever the last step said.
                let probe_age = n
                    .last_sync
                    .map(|t| t.elapsed().as_secs())
                    .unwrap_or(u64::MAX);
                let stalled = probe_age > 45;
                let stale = if !n.ok {
                    format!("  ({})", n.last_step)
                } else if stalled {
                    format!("  (stalled {})", ago(probe_age))
                } else {
                    String::new()
                };
                (
                    format!(
                        "{:8} h={}  root {}  step {step}  {block}{stale}   in {}  out {}",
                        n.name,
                        n.height,
                        networks::short_root(&n.root),
                        human_size(n.rx),
                        human_size(n.tx),
                    ),
                    if n.ok && !stalled {
                        theme::TEXT_PRIMARY
                    } else {
                        theme::ACID_YELLOW
                    },
                )
            } else {
                (
                    format!("{:8} {}  -  {}", n.name, n.url, n.last_step),
                    theme::TEXT_DIM,
                )
            };
            text(&mut commands, page, line, theme::BODY, color);
        }
        let stuck = if view.relay_pending > 0 {
            format!("  ({} waiting)", view.relay_pending)
        } else {
            String::new()
        };
        text(
            &mut commands,
            page,
            format!(
                "relayed {} out, absorbed {} in this session{stuck}   -   net add <name> <url> | net set | net rm",
                view.relayed, view.absorbed
            ),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
    }

    // ── work: every way this body earns ─────────────────────────────────
    let pussy_day = build_prover_card(&mut commands, page, &view);
    build_seer_card(&mut commands, page, &view);

    if pussy_day > 0.0 {
        text(
            &mut commands,
            page,
            format!("total  {pussy_day:.0} PUSSY/day   -   rates declared in ~/cyb/rates.toml"),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
    }
}

/// The zheng card: PUSSY earned by proving — sumcheck sampling over this
/// cyb's own graph, HyperNova folding, a verified ticket or nothing.
fn build_prover_card(commands: &mut Commands, page: Entity, view: &BodyView) -> f64 {
    let text = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            ChildOf(parent),
        ));
    };

    let card = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G * 1.5)),
                border: UiRect::all(Val::Px(1.0)),
                row_gap: Val::Px(theme::G * 0.75),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
            BorderColor::all(theme::BORDER),
            ChildOf(page),
        ))
        .id();

    let p = &view.prover;
    let (state, color) = if p.running {
        (
            format!("proving - {:.0} tickets/min", p.tickets_per_min()),
            theme::ACID_GREEN,
        )
    } else {
        ("idle".to_string(), theme::TEXT_DIM)
    };

    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ChildOf(card),
        ))
        .id();
    text(
        commands,
        head,
        "zheng - PUSSY on sampling + folding".into(),
        theme::BODY,
        theme::TEXT_PRIMARY,
    );
    text(commands, head, state, theme::BODY, color);

    if p.running {
        let fails = if p.failed > 0 {
            format!("  FAILED {}", p.failed)
        } else {
            String::new()
        };
        text(
            commands,
            card,
            format!(
                "tickets {} verified  last {:.1}ms  workers {}  graph n={} axons {}{}",
                p.tickets, p.last_ms, p.workers, p.n, p.axons, fails
            ),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
        let beacon = match &p.beacon {
            Some((name, h, root)) => format!(
                "beacon {name} h={h} {}  -  tickets bind to the last block",
                networks::short_root(root)
            ),
            None => "beacon: none yet - tickets unbound until a network answers".into(),
        };
        text(commands, card, beacon, theme::CAPTION, theme::TEXT_DIM);
        if let Some(secs) = view.checkpoint_in {
            text(
                commands,
                card,
                format!("next work checkpoint -> chain in {secs}s"),
                theme::CAPTION,
                theme::TEXT_DIM,
            );
        }
    }

    let levers = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(theme::G),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(card),
        ))
        .id();
    let lever = |commands: &mut Commands, parent: Entity, label: String, active: bool| -> Entity {
        let b = commands
            .spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.5)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(if active {
                    theme::ACID_GREEN
                } else {
                    theme::BORDER
                }),
                ChildOf(parent),
            ))
            .id();
        commands.spawn((
            Text::new(label),
            TextFont {
                font_size: theme::CAPTION,
                ..default()
            },
            TextColor(if active {
                theme::ACID_GREEN
            } else {
                theme::TEXT_PRIMARY
            }),
            ChildOf(b),
        ));
        b
    };
    let b = lever(
        commands,
        levers,
        if p.running { "stop" } else { "prove" }.into(),
        p.running,
    );
    commands.entity(b).insert(ProveButton);
    text(
        commands,
        levers,
        "fleet".into(),
        theme::CAPTION,
        theme::TEXT_DIM,
    );
    for mode in ["max", "eco", "min"] {
        let b = lever(commands, levers, mode.into(), view.prover_intensity == mode);
        commands.entity(b).insert(ProverIntensityButton(mode));
    }

    let per_proof = declared_rate("per_proof", 1.0);
    let day_rate = p.tickets_per_min() * 60.0 * 24.0 * per_proof;
    if p.lifetime > 0 || p.running {
        // The money line is the CHAIN's number — the same ChainMoney sigma
        // shows — never lifetime x rate: proofs made before the chain
        // listened (or between the last checkpoint and a shutdown) were
        // never minted, and a wallet must not claim what the ledger
        // does not hold.
        let money = &view.chain;
        if money.height > 0 {
            let pace = if p.running && day_rate > 0.0 {
                format!("   ~ {day_rate:.0} PUSSY/day at this pace")
            } else {
                String::new()
            };
            text(
                commands,
                card,
                format!("earned on chain {} PUSSY{pace}", money.balance),
                theme::BODY,
                theme::ACID_GREEN,
            );
        } else {
            text(
                commands,
                card,
                "earnings: waiting for the chain...".into(),
                theme::CAPTION,
                theme::TEXT_DIM,
            );
        }
        text(
            commands,
            card,
            format!(
                "proofs {} all-time (work counter; only checkpointed work mints - {per_proof:.0}/proof)",
                p.lifetime
            ),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
    }
    if p.running { day_rate } else { 0.0 }
}

fn handle_prove_press(
    interactions: Query<&Interaction, (Changed<Interaction>, With<ProveButton>)>,
    link: Res<BodyLink>,
    shared: Res<super::SharedCell>,
    mut notice: ResMut<super::Notice>,
    mut meter: ResMut<ProofMeter>,
    who: Res<super::identity::Identity>,
    mut inbox: ResMut<super::ComInbox>,
) {
    for i in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        if link.prover.is_running() {
            link.prover.stop();
            meter.armed = false;
            let _ = std::fs::write(proving_wanted_file(), "off");
            notice.show("prover stopped - the count is kept");
        } else {
            let axons = shared.cell.lock().expect("shared cell poisoned").axons();
            link.prover.prove(axons, link.nets.clone());
            let _ = std::fs::write(proving_wanted_file(), "on");
            cast_prove_start(&mut meter, &link, &shared, &who, &mut inbox);
            notice.show("proving - the chain hears about it in seconds");
        }
    }
}

fn handle_prover_intensity_press(
    interactions: Query<(&Interaction, &ProverIntensityButton), Changed<Interaction>>,
    mut notice: ResMut<super::Notice>,
) {
    for (i, b) in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        prover::set_intensity(b.0);
        notice.show(format!("prover fleet -> {} (live)", b.0));
    }
}

#[derive(Component)]
enum SeerLive {
    State,
    Detail,
}

fn seer_state_line(view: &BodyView) -> (String, Color) {
    let s = &view.seer;
    if !s.running {
        ("idle".into(), theme::TEXT_DIM)
    } else if s.idle {
        ("watching - no pair in reach".into(), theme::ACID_YELLOW)
    } else {
        (
            format!("linking - {:.0} casts/min", s.casts_per_min()),
            theme::ACID_GREEN,
        )
    }
}

fn seer_detail_line(view: &BodyView) -> String {
    let s = &view.seer;
    let pair = match &s.last_pair {
        Some((a, b)) => format!("  last {a} -> {b}  {:.3}", s.last_score),
        None => String::new(),
    };
    format!("casts {}  proposals {}{pair}", s.casts, s.proposals)
}

fn paint_seer(view: Res<BodyView>, mut q: Query<(&SeerLive, &mut Text, &mut TextColor)>) {
    if !view.is_changed() {
        return;
    }
    for (kind, mut t, mut c) in &mut q {
        match kind {
            SeerLive::State => {
                let (s, color) = seer_state_line(&view);
                **t = s;
                c.0 = color;
            }
            SeerLive::Detail => **t = seer_detail_line(&view),
        }
    }
}

/// The seer card: structure mining — unlinked pairs of files that already
/// exist. Never creates a file. Inverse-degree prior until tru φ* is wired.
fn build_seer_card(commands: &mut Commands, page: Entity, view: &BodyView) {
    let text = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            ChildOf(parent),
        ));
    };

    let card = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G * 1.5)),
                border: UiRect::all(Val::Px(1.0)),
                row_gap: Val::Px(theme::G * 0.75),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
            BorderColor::all(theme::BORDER),
            ChildOf(page),
        ))
        .id();

    let (state, color) = seer_state_line(view);
    let head = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            ChildOf(card),
        ))
        .id();
    text(
        commands,
        head,
        "seer - links among existing files".into(),
        theme::BODY,
        theme::TEXT_PRIMARY,
    );
    commands.spawn((
        SeerLive::State,
        Text::new(state),
        TextFont {
            font_size: theme::BODY,
            ..default()
        },
        TextColor(color),
        ChildOf(head),
    ));

    commands.spawn((
        SeerLive::Detail,
        Text::new(seer_detail_line(view)),
        TextFont {
            font_size: theme::CAPTION,
            ..default()
        },
        TextColor(theme::TEXT_DIM),
        ChildOf(card),
    ));

    let levers = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(theme::G),
                align_items: AlignItems::Center,
                ..default()
            },
            ChildOf(card),
        ))
        .id();
    let lever = |commands: &mut Commands, parent: Entity, label: String, active: bool| -> Entity {
        let b = commands
            .spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.5)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(if active {
                    theme::ACID_GREEN
                } else {
                    theme::BORDER
                }),
                ChildOf(parent),
            ))
            .id();
        commands.spawn((
            Text::new(label),
            TextFont {
                font_size: theme::CAPTION,
                ..default()
            },
            TextColor(if active {
                theme::ACID_GREEN
            } else {
                theme::TEXT_PRIMARY
            }),
            ChildOf(b),
        ));
        b
    };
    let running = view.seer.running;
    let b = lever(
        commands,
        levers,
        if running { "stop" } else { "mine" }.into(),
        running,
    );
    commands.entity(b).insert(SeerButton);
    text(
        commands,
        levers,
        "pace".into(),
        theme::CAPTION,
        theme::TEXT_DIM,
    );
    for mode in ["max", "eco", "min"] {
        let b = lever(commands, levers, mode.into(), view.seer_intensity == mode);
        commands.entity(b).insert(SeerIntensityButton(mode));
    }

    text(
        commands,
        card,
        "never creates a file  -  inverse-degree prior, not φ*".into(),
        theme::CAPTION,
        theme::TEXT_DIM,
    );
}

fn handle_seer_press(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SeerButton>)>,
    link: Res<BodyLink>,
    shared: Res<super::SharedCell>,
    who: Res<super::identity::Identity>,
    mut notice: ResMut<super::Notice>,
) {
    for i in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        if link.seer.is_running() {
            link.seer.stop();
            notice.show("seer stopped");
        } else {
            link.seer.mine(shared.clone(), who.neuron);
            notice.show("seer linking existing files");
        }
    }
}

fn handle_seer_intensity_press(
    interactions: Query<(&Interaction, &SeerIntensityButton), Changed<Interaction>>,
    mut notice: ResMut<super::Notice>,
) {
    for (i, b) in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        seer::set_intensity(b.0);
        notice.show(format!("seer pace -> {} (live)", b.0));
    }
}
