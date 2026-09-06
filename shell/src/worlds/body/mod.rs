//! body — the machine itself, as a world.
//!
//! The main page of cyb: what this body is doing with its cores, its GPU,
//! its memory and its wire, and what that work earns. Everything here is
//! measured or managed for real — the telemetry is the OS's own counters
//! ([`telemetry`]), the miner is a live erga child ([`miner`]), and the
//! only declared (not measured) number on the page, the PUSSY rate, says
//! so out loud.

pub mod telemetry;
#[cfg(target_os = "macos")]
pub mod miner;
pub mod networks;
pub mod prover;

use bevy::prelude::*;
use prysm::theme;

use super::WorldState;
use crate::shell::chrome::{ContentRoot, CHROME_BOTTOM_H, CHROME_TOP_H};

pub struct BodyWorldPlugin;

/// The network hub, exposed as its own resource so the commander (com)
/// can drive the configurator without reaching into body internals.
#[derive(Resource, Clone)]
pub struct BodyLinkHub(pub networks::NetHub);

/// Live handles to the samplers and the miner; created once at build.
#[derive(Resource)]
struct BodyLink {
    telemetry: telemetry::Telemetry,
    #[cfg(target_os = "macos")]
    miner: miner::Miner,
    prover: prover::Prover,
    pub(crate) nets: networks::NetHub,
}

/// The snapshot the page renders. Rewritten once a second while the body
/// world is open; every rewrite repaints via change detection.
#[derive(Resource, Default)]
struct BodyView {
    vitals: telemetry::Vitals,
    #[cfg(target_os = "macos")]
    miner: miner::MinerStat,
    #[cfg(target_os = "macos")]
    ours: bool,
    #[cfg(target_os = "macos")]
    intensity: String,
    prover: prover::ProverStat,
    nets: Vec<networks::NetState>,
}

#[derive(Component)]
struct BodyRoot;

/// The start/stop lever on the miner card.
#[derive(Component)]
struct MineButton;

/// One of the duty-cycle levers: writes erga's intensity file.
#[derive(Component)]
struct IntensityButton(&'static str);

/// The prove/stop lever on the zheng card.
#[derive(Component)]
struct ProveButton;

impl Plugin for BodyWorldPlugin {
    fn build(&self, app: &mut App) {
        let nets = networks::NetHub::start();
        app.insert_resource(BodyLinkHub(nets.clone()));
        app.insert_resource(BodyLink {
            telemetry: telemetry::Telemetry::start(),
            #[cfg(target_os = "macos")]
            miner: miner::Miner::start(),
            prover: prover::Prover::start(),
            nets,
        })
        .init_resource::<BodyView>()
        .add_systems(OnEnter(WorldState::Body), build_page)
        .add_systems(OnExit(WorldState::Body), destroy_page)
        .add_systems(
            Update,
            (
                tick_view,
                rebuild_on_change,
                handle_mine_press,
                handle_intensity_press,
                handle_prove_press,
            )
                .run_if(in_state(WorldState::Body)),
        );
        #[cfg(target_os = "macos")]
        app.add_systems(Startup, resume_mining);
        app.add_systems(Startup, resume_proving);
    }
}

/// The owner's standing order: `~/cyb/mining` holds "on" while mining is
/// wanted. The body re-reads it at boot and resumes — a restart of cyb is
/// not a decision to stop earning.
fn mining_wanted_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("mining")
}

#[cfg(target_os = "macos")]
fn resume_mining(link: Res<BodyLink>, mut notice: ResMut<super::Notice>) {
    let wanted = std::fs::read_to_string(mining_wanted_file())
        .map(|s| s.trim() == "on")
        .unwrap_or(false);
    if wanted && !link.miner.is_ours() {
        match link.miner.mine() {
            Ok(()) => notice.show("resuming the mine - the body remembers"),
            Err(e) => notice.show(format!("miner: {e}")),
        }
    }
}

/// The prover's standing order, twin of `~/cyb/mining`.
fn proving_wanted_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("proving")
}

fn resume_proving(link: Res<BodyLink>, shared: Res<super::SharedCell>) {
    let wanted = std::fs::read_to_string(proving_wanted_file())
        .map(|s| s.trim() == "on")
        .unwrap_or(false);
    if wanted && !link.prover.is_running() {
        let axons = shared.cell.lock().expect("shared cell poisoned").axons();
        link.prover.prove(axons, link.nets.clone());
    }
}

/// Once a second, copy the live counters into the view. The page repaints
/// on the resource write; sub-second flicker would just burn battery.
fn tick_view(
    time: Res<Time>,
    mut timer: Local<f32>,
    link: Res<BodyLink>,
    mut view: ResMut<BodyView>,
) {
    *timer += time.delta_secs();
    if *timer < 1.0 && !view.is_added() {
        return;
    }
    *timer = 0.0;
    view.vitals = link.telemetry.snapshot();
    view.prover = link.prover.stat.lock().map(|s| s.clone()).unwrap_or_default();
    view.nets = link.nets.snapshot();
    #[cfg(target_os = "macos")]
    {
        view.miner = link.miner.stat.lock().map(|s| s.clone()).unwrap_or_default();
        view.ours = link.miner.is_ours();
        view.intensity = link.miner.intensity();
    }
}

fn rebuild_on_change(
    mut commands: Commands,
    view: Res<BodyView>,
    link: Res<BodyLink>,
    roots: Query<Entity, With<BodyRoot>>,
) {
    if !view.is_changed() || view.is_added() || roots.is_empty() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_page(commands, view.into(), link.into());
}

fn destroy_page(mut commands: Commands, q: Query<Entity, With<BodyRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
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
                ..default()
            },
            ChildOf(root),
        ))
        .id();

    let text = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont { font_size: size, ..default() },
            TextColor(color),
            ChildOf(parent),
        ));
    };

    text(&mut commands, page, "body".into(), theme::H2, theme::TEXT_PRIMARY);

    // ── resources ───────────────────────────────────────────────────────
    text(&mut commands, page, "resources".into(), theme::CAPTION, theme::TEXT_DIM);

    let v = &view.vitals;
    let watts = |mw: u32| {
        if mw > 0 {
            format!("  {:.1} W", mw as f32 / 1000.0)
        } else {
            String::new()
        }
    };

    let cpu_task = if view.prover.running { "   zheng (proving)" } else { "" };
    text(
        &mut commands,
        page,
        format!(
            "cpu     {}  {:>3.0}%{}{}",
            bar(v.cpu_pct / 100.0),
            v.cpu_pct,
            watts(v.cpu_mw),
            cpu_task
        ),
        theme::BODY,
        theme::TEXT_PRIMARY,
    );
    if !v.top.is_empty() {
        let who = v
            .top
            .iter()
            .map(|t| format!("{} {:.0}%", t.name, t.cpu_pct))
            .collect::<Vec<_>>()
            .join("   ");
        text(&mut commands, page, format!("        {who}"), theme::CAPTION, theme::TEXT_DIM);
    }

    if v.gpu_pct >= 0.0 {
        #[allow(unused_mut)]
        let mut gpu_task = String::new();
        #[cfg(target_os = "macos")]
        if view.miner.running || view.miner.external {
            gpu_task = "   erga (mining)".into();
        }
        text(
            &mut commands,
            page,
            format!(
                "gpu     {}  {:>3.0}%{}{}",
                bar(v.gpu_pct / 100.0),
                v.gpu_pct,
                watts(v.gpu_mw),
                gpu_task
            ),
            theme::BODY,
            theme::TEXT_PRIMARY,
        );
    }

    if v.mem_total > 0 {
        text(
            &mut commands,
            page,
            format!(
                "memory  {}  {:.1} / {:.0} GB",
                bar(v.mem_used as f32 / v.mem_total as f32),
                gb(v.mem_used),
                gb(v.mem_total)
            ),
            theme::BODY,
            theme::TEXT_PRIMARY,
        );
    }

    text(
        &mut commands,
        page,
        format!("network  down {}   up {}", rate(v.net_rx_bps), rate(v.net_tx_bps)),
        theme::BODY,
        theme::TEXT_PRIMARY,
    );

    // ── networks: the chains this body follows ──────────────────────────
    if !view.nets.is_empty() {
        commands.spawn((
            Text::new("networks"),
            TextFont { font_size: theme::CAPTION, ..default() },
            TextColor(theme::TEXT_DIM),
            Node { margin: UiRect::top(Val::Px(theme::G * 2.0)), ..default() },
            ChildOf(page),
        ));
        for n in &view.nets {
            let (line, color) = if n.height > 0 {
                let ago = n
                    .last_sync
                    .map(|t| format!("{}s ago", t.elapsed().as_secs()))
                    .unwrap_or_default();
                let stale = if n.ok { "" } else { "  (last step failed)" };
                (
                    format!(
                        "{:8} h={}  root {}  synced {ago}{stale}   in {}  out {}",
                        n.name,
                        n.height,
                        networks::short_root(&n.root),
                        human_size(n.rx),
                        human_size(n.tx),
                    ),
                    if n.ok { theme::TEXT_PRIMARY } else { theme::ACID_YELLOW },
                )
            } else {
                (
                    format!("{:8} {}  -  {}", n.name, n.url, n.last_step),
                    theme::TEXT_DIM,
                )
            };
            text(&mut commands, page, line, theme::BODY, color);
        }
        text(
            &mut commands,
            page,
            "net add <name> <url>  |  net set  |  net rm".into(),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
    }

    // ── work: every way this body earns ─────────────────────────────────
    #[allow(unused_mut)]
    let mut pussy_day = 0.0f64;
    #[cfg(target_os = "macos")]
    {
        pussy_day += build_miner_card(&mut commands, page, &view);
    }
    pussy_day += build_prover_card(&mut commands, page, &view);

    if pussy_day > 0.0 {
        text(
            &mut commands,
            page,
            format!(
                "total  {pussy_day:.0} PUSSY/day   -   rates declared in ~/cyb/rates.toml"
            ),
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
            TextFont { font_size: size, ..default() },
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
                "tickets {} verified  last {:.1}ms  graph n={} axons {}{}",
                p.tickets, p.last_ms, p.n, p.axons, fails
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
    }

    let lever = commands
        .spawn((
            ProveButton,
            Button,
            Node {
                padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.5)),
                border: UiRect::all(Val::Px(1.0)),
                align_self: AlignSelf::FlexStart,
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
            BorderColor::all(if p.running { theme::ACID_GREEN } else { theme::BORDER }),
            ChildOf(card),
        ))
        .id();
    text(
        commands,
        lever,
        if p.running { "stop" } else { "prove" }.into(),
        theme::CAPTION,
        if p.running { theme::ACID_GREEN } else { theme::TEXT_PRIMARY },
    );

    let per_proof = declared_rate("per_proof", 1.0);
    let accrued = p.lifetime as f64 * per_proof;
    let day_rate = p.tickets_per_min() * 60.0 * 24.0 * per_proof;
    if p.lifetime > 0 || p.running {
        let pace = if p.running && day_rate > 0.0 {
            format!("   ~ {day_rate:.0} PUSSY/day at this pace")
        } else {
            String::new()
        };
        text(
            commands,
            card,
            format!(
                "accrued {accrued:.0} PUSSY ({} proofs, {per_proof:.0}/proof declared){pace}",
                p.lifetime
            ),
            theme::BODY,
            theme::ACID_GREEN,
        );
        text(
            commands,
            card,
            "offline onramp: proven work is metered now, the chain pays when it lands".into(),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
    }
    if p.running { day_rate } else { 0.0 }
}

#[cfg(target_os = "macos")]
fn build_miner_card(commands: &mut Commands, page: Entity, view: &BodyView) -> f64 {
    let text = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont { font_size: size, ..default() },
            TextColor(color),
            ChildOf(parent),
        ));
    };

    commands.spawn((
        Text::new("work"),
        TextFont { font_size: theme::CAPTION, ..default() },
        TextColor(theme::TEXT_DIM),
        Node { margin: UiRect::top(Val::Px(theme::G * 2.0)), ..default() },
        ChildOf(page),
    ));

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

    let m = &view.miner;
    let (state, color) = if m.running && view.ours {
        (format!("mining - {:.2} MH/s", m.rate_mhs()), theme::ACID_GREEN)
    } else if m.external {
        ("running outside cyb (its own window)".to_string(), theme::ACID_YELLOW)
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
    text(commands, head, "erga - ERGO on the gpu".into(), theme::BODY, theme::TEXT_PRIMARY);
    text(commands, head, state, theme::BODY, color);

    if view.ours && m.running {
        text(
            commands,
            card,
            format!(
                "accepted {}  rejected {}  height {}  {}",
                m.accepted,
                m.rejected,
                m.height,
                if m.device.is_empty() { m.status.clone() } else { m.device.clone() }
            ),
            theme::CAPTION,
            theme::TEXT_DIM,
        );
        if !m.status.is_empty() && !m.device.is_empty() {
            text(commands, card, m.status.clone(), theme::CAPTION, theme::TEXT_DIM);
        }
    }

    // Levers row: start/stop + intensity.
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

    let lever = |commands: &mut Commands,
                 parent: Entity,
                 label: String,
                 active: bool|
     -> Entity {
        let b = commands
            .spawn((
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.5)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(if active { theme::ACID_GREEN } else { theme::BORDER }),
                ChildOf(parent),
            ))
            .id();
        commands.spawn((
            Text::new(label),
            TextFont { font_size: theme::CAPTION, ..default() },
            TextColor(if active { theme::ACID_GREEN } else { theme::TEXT_PRIMARY }),
            ChildOf(b),
        ));
        b
    };

    let mine_label = if view.ours { "stop" } else { "mine" };
    let b = lever(commands, levers, mine_label.into(), view.ours);
    commands.entity(b).insert(MineButton);

    text(commands, levers, "intensity".into(), theme::CAPTION, theme::TEXT_DIM);
    for mode in ["max", "eco", "min"] {
        let b = lever(commands, levers, mode.into(), view.intensity == mode);
        commands.entity(b).insert(IntensityButton(mode));
    }

    // ── earnings ────────────────────────────────────────────────────────
    let mut pussy_day = 0.0;
    if let Some(erg_day) = m.erg_per_day() {
        let pussy = erg_day * declared_rate("per_erg", 1_000_000.0);
        pussy_day = pussy;
        let usd = if m.price_usd > 0.0 {
            format!("   (${:.2}/day)", erg_day * m.price_usd)
        } else {
            String::new()
        };
        text(
            commands,
            card,
            format!("est {erg_day:.4} ERG/day  =  {pussy:.0} PUSSY/day{usd}"),
            theme::BODY,
            theme::ACID_GREEN,
        );
    } else if view.ours && m.running {
        let why = if m.difficulty <= 0.0 {
            "est: waiting for network difficulty..."
        } else {
            "est: waiting for the first measured rate..."
        };
        text(commands, card, why.into(), theme::CAPTION, theme::TEXT_DIM);
    }
    pussy_day
}

fn handle_mine_press(
    mut interactions: Query<&Interaction, (Changed<Interaction>, With<MineButton>)>,
    link: Res<BodyLink>,
    mut notice: ResMut<super::Notice>,
) {
    for i in &mut interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        #[cfg(target_os = "macos")]
        {
            if link.miner.is_ours() {
                link.miner.stop();
                let _ = std::fs::write(mining_wanted_file(), "off");
                notice.show("miner stopped");
            } else {
                match link.miner.mine() {
                    Ok(()) => {
                        let _ = std::fs::write(mining_wanted_file(), "on");
                        notice.show("erga is waking - epoch table first");
                    }
                    Err(e) => notice.show(format!("miner: {e}")),
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = &link;
            notice.show("this body carries no miner yet");
        }
    }
}

fn handle_prove_press(
    interactions: Query<&Interaction, (Changed<Interaction>, With<ProveButton>)>,
    link: Res<BodyLink>,
    shared: Res<super::SharedCell>,
    mut notice: ResMut<super::Notice>,
) {
    for i in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        if link.prover.is_running() {
            link.prover.stop();
            let _ = std::fs::write(proving_wanted_file(), "off");
            notice.show("prover stopped - the count is kept");
        } else {
            let axons = shared.cell.lock().expect("shared cell poisoned").axons();
            link.prover.prove(axons, link.nets.clone());
            let _ = std::fs::write(proving_wanted_file(), "on");
            notice.show("proving over your graph - every ticket verified");
        }
    }
}

fn handle_intensity_press(
    mut interactions: Query<(&Interaction, &IntensityButton), Changed<Interaction>>,
    link: Res<BodyLink>,
    mut notice: ResMut<super::Notice>,
) {
    for (i, b) in &mut interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        #[cfg(target_os = "macos")]
        {
            link.miner.set_intensity(b.0);
            notice.show(format!("intensity -> {} (live, no restart)", b.0));
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (&link, &b);
        }
    }
}
