//! Sigma world — Bevy money surface (balance, send, events, sense).
//!
//! Opens the same durable graph as `cy` (`~/cyb/graph.log`) and drives
//! [`MoneyWallet`]. Hotkey: Cmd+4 · address `cyb://sigma`.

use bevy::prelude::*;
use cyb_core::{Cell, MoneyEvent, MoneyWallet, money_to_sense};
use prysm::theme;

use super::WorldState;
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H};

pub struct SigmaWorldPlugin;

#[derive(Component)]
struct SigmaRoot;

#[derive(Component)]
struct SigmaBalanceLabel;

#[derive(Component)]
struct SigmaEventsLabel;

#[derive(Component)]
struct SigmaStatusLabel;

#[derive(Component)]
enum SigmaBtn {
    Fund,
    Send,
    Finalize,
    Refresh,
}

/// Live money state for the sigma world.
#[derive(Resource)]
pub struct SigmaState {
    cell: Cell,
    wallet: MoneyWallet,
    token: [u8; 32],
    peer: [u8; 32],
    balance: u64,
    tip_h: u64,
    grade4: bool,
    log: Vec<String>,
    status: String,
}

impl Default for SigmaState {
    fn default() -> Self {
        let neuron = neuron_from_identity();
        let path = default_graph_path();
        let mut cell = Cell::open(&path).unwrap_or_else(|_| Cell::ephemeral());
        let mut wallet = MoneyWallet::new(neuron).with_tip_prover();
        wallet.sync_tip_local(&cell);
        let token = label_particle("CYB");
        let peer = label_particle("bob");
        let balance = wallet.balance(&cell, &neuron, &token);
        let tip_h = wallet.tip().height;
        let grade4 = wallet.grade4();
        Self {
            cell,
            wallet,
            token,
            peer,
            balance,
            tip_h,
            grade4,
            log: vec!["sigma ready · fund / send / finalize".into()],
            status: format!("neuron {}…", hex3(&neuron)),
        }
    }
}

impl Plugin for SigmaWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SigmaState>()
            .add_systems(OnEnter(WorldState::Sigma), setup_sigma)
            .add_systems(OnExit(WorldState::Sigma), destroy_sigma)
            .add_systems(
                Update,
                (handle_sigma_buttons, refresh_sigma_labels).run_if(in_state(WorldState::Sigma)),
            );
    }
}

fn setup_sigma(mut commands: Commands, state: Res<SigmaState>) {
    let top = CHROME_TOP_H + 12.0;
    let bottom = CHROME_BOTTOM_H + 12.0;
    commands
        .spawn((
            SigmaRoot,
            crate::shell::chrome::ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                right: Val::Px(16.0),
                top: Val::Px(top),
                bottom: Val::Px(bottom),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("sigma · money"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.95, 0.8)),
            ));
            root.spawn((
                SigmaStatusLabel,
                Text::new(state.status.clone()),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.6, 0.65)),
            ));
            root.spawn((
                SigmaBalanceLabel,
                Text::new(balance_text(&state)),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.9, 0.4)),
            ));

            // buttons row
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|row| {
                for (label, btn) in [
                    ("fund +100", SigmaBtn::Fund),
                    ("send 10 → bob", SigmaBtn::Send),
                    ("finalize", SigmaBtn::Finalize),
                    ("refresh", SigmaBtn::Refresh),
                ] {
                    row.spawn((
                        btn,
                        Button,
                        Node {
                            padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::DARK_BASE),
                        BorderColor::all(theme::BORDER),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            Text::new(label),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.75, 0.9, 0.8)),
                        ));
                    });
                }
            });

            root.spawn((
                Text::new("events · sense"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.55, 0.6)),
            ));
            root.spawn((
                SigmaEventsLabel,
                Text::new(state.log.join("\n")),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.8, 0.85, 0.9)),
            ));
        });
}

fn destroy_sigma(mut commands: Commands, q: Query<Entity, With<SigmaRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn handle_sigma_buttons(
    mut interactions: Query<(&Interaction, &SigmaBtn), Changed<Interaction>>,
    mut state: ResMut<SigmaState>,
) {
    for (interaction, btn) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let peer = state.peer;
        let token = state.token;
        // Split-borrow wallet + cell (two distinct fields).
        let SigmaState {
            cell, wallet, log, ..
        } = &mut *state;
        match btn {
            SigmaBtn::Fund => {
                wallet.fund_for_test(cell, token, 100);
                log.push("funded +100 CYB".into());
                drain_sense_parts(wallet, log);
            }
            SigmaBtn::Send => {
                if !wallet.grade4() {
                    wallet.sync_tip_local(cell);
                }
                match wallet.send(cell, peer, token, 10) {
                    Ok((sig, ev)) => {
                        let ok = ev.verify(wallet.tip());
                        log.push(format!("sent 10 → bob  sig {}… final={ok}", hex3(&sig)));
                        drain_sense_parts(wallet, log);
                    }
                    Err(e) => log.push(format!("send failed: {e:?}")),
                }
            }
            SigmaBtn::Finalize => {
                wallet.finalize_block(cell);
                let ready = wallet.mature_settles();
                let h = wallet.tip().height;
                let g4 = wallet.grade4();
                log.push(format!(
                    "finalize h={h} grade4={g4} matured={}",
                    ready.len()
                ));
            }
            SigmaBtn::Refresh => {
                wallet.sync_tip_local(cell);
                log.push("refreshed tip".into());
            }
        }
        trim_log(&mut state.log);
        refresh_numbers(&mut state);
    }
}

fn drain_sense_parts(wallet: &mut MoneyWallet, log: &mut Vec<String>) {
    let neuron = wallet.neuron;
    let ev = wallet.drain_events();
    for e in &ev {
        log.push(format_event(e));
    }
    for n in money_to_sense(neuron, &ev) {
        log.push(format!(
            "NOTIFY {} amt={} {}",
            n.kind,
            n.amount,
            hex3(&n.reason)
        ));
    }
}

fn trim_log(log: &mut Vec<String>) {
    if log.len() > 40 {
        let drop = log.len() - 40;
        log.drain(0..drop);
    }
}

fn refresh_sigma_labels(
    state: Res<SigmaState>,
    mut bal: Query<
        &mut Text,
        (
            With<SigmaBalanceLabel>,
            Without<SigmaEventsLabel>,
            Without<SigmaStatusLabel>,
        ),
    >,
    mut events: Query<
        &mut Text,
        (
            With<SigmaEventsLabel>,
            Without<SigmaBalanceLabel>,
            Without<SigmaStatusLabel>,
        ),
    >,
    mut status: Query<
        &mut Text,
        (
            With<SigmaStatusLabel>,
            Without<SigmaBalanceLabel>,
            Without<SigmaEventsLabel>,
        ),
    >,
) {
    if !state.is_changed() {
        return;
    }
    if let Ok(mut t) = bal.single_mut() {
        *t = Text::new(balance_text(&state));
    }
    if let Ok(mut t) = events.single_mut() {
        let tail: Vec<_> = state.log.iter().rev().take(12).cloned().collect();
        let mut lines: Vec<_> = tail.into_iter().rev().collect();
        if lines.is_empty() {
            lines.push("(no events)".into());
        }
        *t = Text::new(lines.join("\n"));
    }
    if let Ok(mut t) = status.single_mut() {
        *t = Text::new(format!(
            "{} · tip h={} grade4={}",
            state.status,
            state.tip_h,
            if state.grade4 { "yes" } else { "no" }
        ));
    }
}

fn refresh_numbers(state: &mut SigmaState) {
    let n = state.wallet.neuron;
    state.balance = state.wallet.balance(&state.cell, &n, &state.token);
    state.tip_h = state.wallet.tip().height;
    state.grade4 = state.wallet.grade4();
}

fn format_event(e: &MoneyEvent) -> String {
    match e {
        MoneyEvent::TransferOut { amount, to, .. } => {
            format!("out {amount} → {}", hex3(to))
        }
        MoneyEvent::TransferIn { amount, from, .. } => {
            format!("in {amount} ← {}", hex3(from))
        }
        MoneyEvent::RewardCredited { amount, clock, .. } => format!("reward {:?} {amount}", clock),
        MoneyEvent::Finalized { signal, .. } => format!("final {}", hex3(signal)),
        MoneyEvent::TipAdvanced { height, grade4, .. } => {
            format!("tip h={height} g4={grade4}")
        }
        MoneyEvent::BalanceUpdated { amount, .. } => format!("bal={amount}"),
        MoneyEvent::FinalityFailed { reason, .. } => format!("fail {reason}"),
    }
}

fn balance_text(state: &SigmaState) -> String {
    format!("{} CYB", state.balance)
}

fn label_particle(label: &str) -> [u8; 32] {
    let mut p = [0u8; 32];
    let b = label.as_bytes();
    let n = b.len().min(32);
    p[..n].copy_from_slice(&b[..n]);
    p
}

fn hex3(b: &[u8]) -> String {
    b[..3.min(b.len())]
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn default_graph_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("graph.log")
}

/// Best-effort neuron from same mnemonic path as `cy` identity.
fn neuron_from_identity() -> [u8; 32] {
    // Stable per-device demo neuron when mnemonic stack is heavy in GUI path.
    // CLI identity remains authoritative; sigma opens shared graph by path.
    let host = std::env::var("USER").unwrap_or_else(|_| "cyb".into());
    let mut p = [0u8; 32];
    let bytes = host.as_bytes();
    let n = bytes.len().min(32);
    p[..n].copy_from_slice(&bytes[..n]);
    p[31] = 0x53; // 'S' tag sigma demo
    p
}
