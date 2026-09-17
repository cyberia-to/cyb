//! Sigma world — Bevy money surface (balance, send, events, sense).
//!
//! Opens the same durable graph as `cy` (`~/cyb/graph.log`) and drives
//! [`MoneyWallet`]. Hotkey: Cmd+4 · address `cyb://sigma`.

use bevy::prelude::*;
use cyb_core::{MoneyEvent, MoneyWallet, money_to_sense};
use prysm::theme;

use super::{ComInbox, Notice, SharedCell, Speaker, WorldState};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H};

pub mod chain;

pub struct SigmaWorldPlugin;

/// Markers for the on-chain money block.
#[derive(Component)]
struct ChainMoneyLabel;
#[derive(Component)]
struct ChainReceiptLabel;
#[derive(Component)]
struct ChainSendBtn;
#[derive(Component)]
struct ChainAddrBtn;

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
    fn new(shared: &SharedCell, neuron: [u8; 32]) -> Self {
        let cell = shared.cell.lock().expect("shared cell poisoned");
        let wallet = MoneyWallet::new(neuron).with_tip_prover();
        // Do not `sync_tip_local` here. That reads `bbg.state.root()`, which
        // Brakedown-commits every dimension. A lived-in `~/cyb/graph.log`
        // either panics (`Brakedown variable limit`) or freezes the splash
        // on the tensor-Merkle. The local tip is filled on first sigma
        // enter; on-chain money (ChainMoney) does not need it to boot.
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
        let neuron = app.world().resource::<super::identity::Identity>().neuron;
        app.insert_resource(SigmaState::new(&shared, neuron));
        app.init_resource::<chain::ChainMoney>();
        app.add_systems(
            OnEnter(WorldState::Sigma),
            (sync_local_tip, setup_sigma, refresh_chain_on_enter),
        )
        .add_systems(
            Update,
            (handle_chain_buttons, refresh_chain_labels).run_if(in_state(WorldState::Sigma)),
        )
        // The balance poll runs in EVERY world: sigma and the body's
        // zheng card read the same ChainMoney, so the money is one
        // number everywhere — coherent by construction.
        .add_systems(Update, poll_chain);
    }
}

/// Fill the local BBG tip now that the window is up. Never at plugin
/// build: `state.root()` is what killed 0.14.0 on a lived-in graph.log.
fn sync_local_tip(mut sigma: ResMut<SigmaState>, shared: Res<SharedCell>) {
    let cell = shared.cell.lock().expect("shared cell poisoned");
    let neuron = sigma.wallet.neuron;
    let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        sigma.wallet.sync_tip_local(&cell);
    }))
    .is_ok();
    if !ok {
        warn!("sigma: local tip skipped — graph exceeds the commitment cap");
        return;
    }
    sigma.tip_h = sigma.wallet.tip().height;
    sigma.grade4 = sigma.wallet.grade4();
    sigma.balance = sigma.wallet.balance(&cell, &neuron, &sigma.token);
}

fn setup_sigma(
    mut commands: Commands,
    _state: Res<SigmaState>,
    mut worlds: Query<(&crate::worlds::WorldUi, &mut Visibility)>,
) {
    if crate::worlds::reveal_world(WorldState::Sigma, &mut worlds) {
        return;
    }
    let top = CHROME_TOP_H + 12.0;
    let bottom = CHROME_BOTTOM_H + 12.0;
    commands
        .spawn((
            SigmaRoot,
            crate::worlds::WorldUi(WorldState::Sigma),
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

            // ── the chain: earned by proving, spendable now ────────────
            root.spawn((
                Text::new("on the chain (pussy) - earned by proven work"),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.6, 0.65)),
            ));
            root.spawn((
                ChainMoneyLabel,
                Text::new("querying the chain..."),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(Color::srgb(0.4, 0.95, 0.6)),
            ));
            root.spawn((
                ChainReceiptLabel,
                Text::new(""),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
                TextColor(Color::srgb(0.55, 0.6, 0.65)),
            ));
            // One lever: send. It folds the commander into a pay form
            // (recipient + amount). Everything else is automatic — the
            // balance polls itself, every pay finalizes in its own block.
            root.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                ..default()
            })
            .with_children(|row| {
                row.spawn((
                    ChainSendBtn,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(22.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(theme::DARK_BASE),
                    BorderColor::all(Color::srgb(0.2, 0.5, 0.35)),
                ))
                .with_children(|inner| {
                    inner.spawn((
                        Text::new("send"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.9, 0.85)),
                    ));
                });
                row.spawn((
                    ChainAddrBtn,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(8.0)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(theme::DARK_BASE),
                    BorderColor::all(theme::BORDER),
                ))
                .with_children(|inner| {
                    inner.spawn((
                        Text::new("receive"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.6, 0.65, 0.7)),
                    ));
                });
            });

            // No event list here. Everything this page does is said in com,
            // where it can be scrolled back through; repeating the last twelve
            // lines beside the buttons was two records and one of them always
            // stale. What is left on the page is the state — balance, tip —
            // and what just happened arrives as a notice under the address bar.
        });
}

#[allow(dead_code)] // demo wallet retired; kept until MoneyWallet grows a real role
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
            SigmaBtn::Fund => "fund 100 PUSSY".to_string(),
            SigmaBtn::Send => "send 10 PUSSY to bob".to_string(),
            SigmaBtn::Finalize => "finalize the block".to_string(),
            SigmaBtn::Refresh => "refresh the tip".to_string(),
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
                said.push(format!(
                    "finalize h={h} grade4={g4} matured={}",
                    ready.len()
                ));
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

#[allow(dead_code)]
fn refresh_sigma_labels(
    state: Res<SigmaState>,
    mut bal: Query<&mut Text, (With<SigmaBalanceLabel>, Without<SigmaStatusLabel>)>,
    mut status: Query<&mut Text, (With<SigmaStatusLabel>, Without<SigmaBalanceLabel>)>,
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

/// Entering sigma asks the chain; the answer lands via the shared slot.
/// Defensive on purpose: with `CYB_WORLD=sigma` the initial OnEnter can
/// fire while plugins are still assembling, before body's resources
/// exist. Missing pieces mean "ask later", never a panic.
fn refresh_chain_on_enter(
    money: Option<Res<chain::ChainMoney>>,
    hub: Option<Res<crate::worlds::body::BodyLinkHub>>,
    who: Option<Res<crate::worlds::identity::Identity>>,
) {
    let (Some(money), Some(hub), Some(who)) = (money, hub, who) else {
        return;
    };
    if let Some(url) = chain::chain_url(&hub.0) {
        money.refresh(url, chain::neuron_hex(&who));
    }
}

fn handle_chain_buttons(
    sends: Query<&Interaction, (Changed<Interaction>, With<ChainSendBtn>)>,
    addrs: Query<&Interaction, (Changed<Interaction>, With<ChainAddrBtn>)>,
    chrome: Option<ResMut<crate::shell::chrome::ChromeState>>,
    who: Option<Res<crate::worlds::identity::Identity>>,
    notice: Option<ResMut<crate::worlds::Notice>>,
) {
    let send = sends.iter().any(|i| *i == Interaction::Pressed);
    let addr = addrs.iter().any(|i| *i == Interaction::Pressed);
    if !send && !addr {
        return;
    }
    if send {
        // The commander folds into the pay form: recipient, then amount.
        if let Some(mut chrome) = chrome {
            chrome.pay = Some(crate::shell::chrome::PayDraft::default());
            chrome.focused = true;
        }
    } else if let (Some(who), Some(mut notice)) = (who, notice) {
        let hex = chain::neuron_hex(&who);
        match crate::shell::clipboard::write_clipboard(&hex) {
            Ok(()) => notice.show("address copied - share it to receive"),
            Err(e) => notice.show(format!("clipboard: {e}")),
        }
    }
}

/// Repaint the chain block when the slot moves.
fn refresh_chain_labels(
    money: Option<Res<chain::ChainMoney>>,
    mut seen: Local<u64>,
    mut balance_q: Query<&mut Text, (With<ChainMoneyLabel>, Without<ChainReceiptLabel>)>,
    mut receipt_q: Query<&mut Text, (With<ChainReceiptLabel>, Without<ChainMoneyLabel>)>,
) {
    let Some(money) = money else { return };
    let s = money.snapshot();
    if s.version == *seen {
        return;
    }
    *seen = s.version;
    for mut t in &mut balance_q {
        **t = if s.busy && s.balance == 0 {
            "querying the chain...".into()
        } else {
            format!("{} PUSSY", s.balance)
        };
    }
    for mut t in &mut receipt_q {
        **t = if !s.error.is_empty() {
            format!("! {}", s.error)
        } else {
            s.receipt.clone()
        };
    }
}

/// While sigma is open the balance stays live: a poll every 15s, and the
/// FIRST poll fires immediately — which also covers the boot-into-sigma
/// case where OnEnter ran before body's resources existed.
fn poll_chain(
    time: Res<Time>,
    mut wait: Local<Option<f32>>,
    money: Option<Res<chain::ChainMoney>>,
    hub: Option<Res<crate::worlds::body::BodyLinkHub>>,
    who: Option<Res<crate::worlds::identity::Identity>>,
) {
    let due = match wait.as_mut() {
        None => {
            *wait = Some(0.0);
            true
        }
        Some(w) => {
            *w += time.delta_secs();
            if *w >= 15.0 {
                *w = 0.0;
                true
            } else {
                false
            }
        }
    };
    if !due {
        return;
    }
    let (Some(money), Some(hub), Some(who)) = (money, hub, who) else {
        return;
    };
    if money.snapshot().busy {
        return;
    }
    if let Some(url) = chain::chain_url(&hub.0) {
        money.refresh(url, chain::neuron_hex(&who));
    }
}
