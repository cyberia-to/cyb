//! Sigma world — Bevy money surface (balance, send, events, sense).
//!
//! Opens the same durable graph as `cy` (`~/cyb/graph.log`) and drives
//! [`MoneyWallet`]. Hotkey: Cmd+4 · address `cyb://sigma`.

use bevy::prelude::*;
use cyb_core::{MoneyEvent, MoneyWallet, money_to_sense};
use prysm::theme;

use super::{ComInbox, Notice, SharedCell, Speaker, WorldState};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H};

pub struct SigmaWorldPlugin;

#[derive(Component)]
struct SigmaRoot;

#[derive(Component)]
struct SigmaBalanceLabel;

#[derive(Component)]
struct SigmaStatusLabel;

#[derive(Component)]
enum SigmaBtn {
    Fund,
    Send,
    Finalize,
    Refresh,
}

/// Live money state for the sigma world. The graph itself lives in
/// [`SharedCell`]; sigma keeps only the wallet that signs into it.
#[derive(Resource)]
pub struct SigmaState {
    wallet: MoneyWallet,
    token: [u8; 32],
    peer: [u8; 32],
    balance: u64,
    tip_h: u64,
    grade4: bool,
    status: String,
}

impl SigmaState {
    /// Built against the shared cell rather than `Default`, because the tip
    /// and balance are read out of the same graph everyone else writes.
    fn new(shared: &SharedCell) -> Self {
        let neuron = super::local_neuron();
        let cell = shared.cell.lock().expect("shared cell poisoned");
        let mut wallet = MoneyWallet::new(neuron).with_tip_prover();
        wallet.sync_tip_local(&cell);
        // PUSSY, not CYB: the nearest chain this cyb will actually join —
        // small state, no financial stakes, halted with its snapshot in hand.
        let token = label_particle("PUSSY");
        let peer = label_particle("bob");
        let balance = wallet.balance(&cell, &neuron, &token);
        let tip_h = wallet.tip().height;
        let grade4 = wallet.grade4();
        Self {
            wallet,
            token,
            peer,
            balance,
            tip_h,
            grade4,
            status: format!("neuron {}", hex3(&neuron)),
        }
    }
}

impl Plugin for SigmaWorldPlugin {
    fn build(&self, app: &mut App) {
        // Built here, not in a Startup system: `CYB_WORLD=sigma` makes this
        // the boot world, and insert_state applies the initial transition
        // during plugin build — OnEnter runs before any Startup schedule, so
        // the state must already exist. WorldsPlugin registered SharedCell
        // just before this plugin.
        let shared = app.world().resource::<SharedCell>().clone();
        app.insert_resource(SigmaState::new(&shared));
        app
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
                // The root's bottom edge is the chrome's top edge, so clipping
                // here is what makes the page pass under the bars instead of
                // drawing over them.
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("sigma / money"),
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
                    ("send 10 -> bob", SigmaBtn::Send),
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

            // No event list here. Everything this page does is said in com,
            // where it can be scrolled back through; repeating the last twelve
            // lines beside the buttons was two records and one of them always
            // stale. What is left on the page is the state — balance, tip —
            // and what just happened arrives as a notice under the address bar.
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
    shared: Res<SharedCell>,
    mut inbox: ResMut<ComInbox>,
    mut notice: ResMut<Notice>,
) {
    for (interaction, btn) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let peer = state.peer;
        let token = state.token;

        // What the press meant, in the words someone would use for it. This is
        // the left-hand side of the record in com; everything the wallet says
        // back lands on the right.
        let intent = match btn {
            SigmaBtn::Fund     => "fund 100 PUSSY".to_string(),
            SigmaBtn::Send     => "send 10 PUSSY to bob".to_string(),
            SigmaBtn::Finalize => "finalize the block".to_string(),
            SigmaBtn::Refresh  => "refresh the tip".to_string(),
        };
        inbox.say(Speaker::User, intent);

        let mut said: Vec<String> = Vec::new();
        let wallet = &mut state.wallet;
        let mut cell_guard = shared.cell.lock().expect("shared cell poisoned");
        let cell = &mut *cell_guard;
        match btn {
            SigmaBtn::Fund => {
                wallet.fund_for_test(cell, token, 100);
                said.push("funded +100 PUSSY".into());
                drain_sense_parts(wallet, &mut said);
            }
            SigmaBtn::Send => {
                if !wallet.grade4() {
                    wallet.sync_tip_local(cell);
                }
                match wallet.send(cell, peer, token, 10) {
                    Ok((sig, ev)) => {
                        let ok = ev.verify(wallet.tip());
                        said.push(format!("sent 10 -> bob  sig {} final={ok}", hex3(&sig)));
                        drain_sense_parts(wallet, &mut said);
                    }
                    Err(e) => said.push(format!("send failed: {e:?}")),
                }
            }
            SigmaBtn::Finalize => {
                wallet.finalize_block(cell);
                let ready = wallet.mature_settles();
                let h = wallet.tip().height;
                let g4 = wallet.grade4();
                said.push(format!("finalize h={h} grade4={g4} matured={}", ready.len()));
            }
            SigmaBtn::Refresh => {
                wallet.sync_tip_local(cell);
                said.push("refreshed tip".into());
            }
        }

        // The first line is the summary of what happened; the rest is the
        // detail. The summary is what surfaces as a notice, so you learn the
        // transfer went through without leaving the page you are on.
        if let Some(first) = said.first() {
            notice.show(first.clone());
        }
        drop(cell_guard);
        shared.bump();
        for line in said {
            inbox.say(Speaker::System, line);
        }
        refresh_numbers(&mut state, &shared);
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

fn refresh_sigma_labels(
    state: Res<SigmaState>,
    mut bal: Query<
        &mut Text,
        (With<SigmaBalanceLabel>, Without<SigmaStatusLabel>),
    >,
    mut status: Query<
        &mut Text,
        (With<SigmaStatusLabel>, Without<SigmaBalanceLabel>),
    >,
) {
    if !state.is_changed() {
        return;
    }
    if let Ok(mut t) = bal.single_mut() {
        *t = Text::new(balance_text(&state));
    }
    if let Ok(mut t) = status.single_mut() {
        *t = Text::new(format!(
            "{} / tip h={} grade4={}",
            state.status,
            state.tip_h,
            if state.grade4 { "yes" } else { "no" }
        ));
    }
}

fn refresh_numbers(state: &mut SigmaState, shared: &SharedCell) {
    let n = state.wallet.neuron;
    let cell = shared.cell.lock().expect("shared cell poisoned");
    state.balance = state.wallet.balance(&cell, &n, &state.token);
    state.tip_h = state.wallet.tip().height;
    state.grade4 = state.wallet.grade4();
}

fn format_event(e: &MoneyEvent) -> String {
    match e {
        MoneyEvent::TransferOut { amount, to, .. } => {
            format!("out {amount} -> {}", hex3(to))
        }
        MoneyEvent::TransferIn { amount, from, .. } => {
            format!("in {amount} <- {}", hex3(from))
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
    format!("{} PUSSY", state.balance)
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


