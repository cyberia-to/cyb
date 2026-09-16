use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use prysm::theme;

use crate::shell::clipboard::{read_clipboard, write_clipboard};
use crate::shell::platform::{SafeArea, SoftInput};
use crate::worlds::{Notice, WorldState};

/// Logical-pixel heights of the persistent chrome bars (address bar top, commander bottom).
pub const CHROME_TOP_H: f32 = 36.0;
/// Height of the notice band that sits just under the address bar.
const NOTICE_H: f32 = 26.0;
/// Commander field height.
const COMMANDER_H: f32 = 40.0;
/// World-tab strip height — a thumb target, not a text link.
const TABS_H: f32 = 48.0;
/// Both bottom rows plus the gap between them.
pub const CHROME_BOTTOM_H: f32 = COMMANDER_H + 6.0 + TABS_H;

/// The commander folded into a payment form: two fields, one act.
#[derive(Clone, Default)]
pub struct PayDraft {
    pub to: String,
    pub amount: String,
    /// 0 = recipient, 1 = amount.
    pub field: u8,
    /// Cmd+A marked the active field selected: the next keystroke or
    /// paste replaces its whole content, the way a real text field's
    /// select-all behaves.
    pub selected: bool,
}

#[derive(Resource)]
pub struct ChromeState {
    pub focused: bool,
    /// Some = the commander is a pay form right now.
    pub pay: Option<PayDraft>,
    /// Set when something other than the keyboard decided the line is done —
    /// the soft keyboard's "go", which arrives as text rather than a key.
    pub submit_now: bool,
    pub just_submitted: bool, // true for one frame after commander Enter
    pub text: String,
    /// Cmd+A marked the commander text selected — same select-all-then-
    /// replace behavior as the pay form's own `PayDraft::selected`.
    pub selected: bool,
    key_cursor: bevy::ecs::message::MessageCursor<KeyboardInput>,
}

impl Default for ChromeState {
    fn default() -> Self {
        Self {
            focused: false,
            pay: None,
            submit_now: false,
            just_submitted: false,
            text: String::new(),
            selected: false,
            key_cursor: Default::default(),
        }
    }
}

/// The shell prompt com publishes — cwd and all. The commander wears it, so
/// the one input in cyb reads as com's prompt line wherever it is drawn.
#[derive(Resource, Default)]
pub struct ComPrompt(pub String);

/// The two chrome bars, so the safe-area system can pad them.
#[derive(Component)]
struct ChromeTopBar;
#[derive(Component)]
struct ChromeBottomBar;

/// A world's root node. The chrome owns the screen bands the bars cover —
/// tag a world root with this and its top/bottom track the bars' true
/// heights (chrome + system safe area), so world content never sits under
/// the panels no matter the device insets.
#[derive(Component)]
pub struct ContentRoot;

#[derive(Component)]
struct ChromeCamera;
#[derive(Component)]
struct AddressBarText;
#[derive(Component)]
pub struct CommanderContainer;
#[derive(Component)]
struct CommanderPrompt;
#[derive(Component)]
struct CommanderText;
#[derive(Component)]
struct CommanderSubmit;

/// The second field of the pay form (amount), hidden in normal mode.
#[derive(Component)]
struct PayAmountBox;
#[derive(Component)]
struct PayAmountText;
/// A tap-target for one world — the touch counterpart of Cmd+1..4, and the
/// only world navigation Android has until gestures land.
#[derive(Component)]
struct WorldNavButton(WorldState);

pub struct ChromePlugin;

impl Plugin for ChromePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChromeState>()
            .init_resource::<ComPrompt>()
            .add_systems(Startup, spawn_chrome)
            .add_systems(OnEnter(WorldState::Com), unfocus_chrome)
            .add_systems(
                Update,
                (
                    clear_chrome_submitted, // must be first
                    handle_commander_click,
                    handle_commander_submit,
                    handle_world_buttons,
                    handle_chrome_input,
                    update_address_bar,
                    update_commander_display,
                    sync_commander_focus_style,
                    apply_safe_area,
                    show_notice,
                    show_identity,
                    request_soft_input,
                    sync_commander_prompt,
                )
                    .chain(),
            );
    }
}

fn unfocus_chrome(mut chrome: ResMut<ChromeState>) {
    chrome.focused = false;
    chrome.just_submitted = false;
    chrome.text.clear();
}

fn clear_chrome_submitted(mut chrome: ResMut<ChromeState>) {
    chrome.just_submitted = false;
}

fn spawn_chrome(mut commands: Commands) {
    let cam = commands
        .spawn((
            ChromeCamera,
            Camera2d,
            Camera {
                order: 100,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            IsDefaultUiCamera,
        ))
        .id();

    // Fills the hole between chrome bars when a world is not yet drawn.
    // Camera does not clear; without this the swapchain shows a black flash.
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(theme::DARK_BASE),
        GlobalZIndex(-100),
        UiTargetCamera(cam),
    ));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            UiTargetCamera(cam),
        ))
        .with_children(|root| {
            // ── Notice band ────────────────────────────────────────────
            //
            // Absolutely positioned so it costs the worlds no room: it is
            // there for a few seconds and then it is not, and nothing below
            // should move when it comes and goes.
            root.spawn((
                NoticeBand,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    right: Val::Px(0.0),
                    top: Val::Px(CHROME_TOP_H),
                    height: Val::Px(NOTICE_H),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    // One row, and never wider than the screen: a notice that
                    // runs off both edges says less than no notice at all.
                    overflow: Overflow::clip(),
                    display: Display::None,
                    ..default()
                },
                // Global, not local: the world roots are separate top-level
                // nodes with opaque backgrounds that start at exactly this
                // line, and a local z only orders against siblings.
                GlobalZIndex(10),
            ))
            .with_children(|band| {
                band.spawn((
                    NoticeText,
                    Text::new(""),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(theme::ACID_GREEN),
                ));
            });

            // ── Address Bar (top, full width) ───────────────────────────
            root.spawn((
                ChromeTopBar,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(36.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    border: UiRect::bottom(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(theme::BORDER),
            ))
            .with_children(|bar| {
                bar.spawn((
                    AddressBarText,
                    Text::new("cyb://brain"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.55, 0.55, 0.70, 1.0)),
                ));
                bar.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(16.0),
                    ..default()
                })
                .with_children(|right| {
                    right.spawn((
                        IdentityLabel,
                        Text::new(""),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.35, 0.35, 0.50, 0.8)),
                    ));
                    // Which cyb this window IS — hash + build minute, the
                    // OUTERMOST top-right mark. Green enough to find at a
                    // glance; the question "am I looking at the build I
                    // just made?" should never need a terminal.
                    right.spawn((
                        Text::new(env!("CYB_VERSION")),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.13, 0.72, 0.45, 1.0)),
                    ));
                });
            });

            // ── Bottom chrome: commander above, world tabs on the very
            // bottom edge — the row a thumb reaches without moving the hand.
            root.spawn((
                ChromeBottomBar,
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                // Opaque: whatever a world draws slides under this bar, never
                // through it.
                BackgroundColor(theme::DARK_BASE),
            ))
            .with_children(|bottom| {
                // Upper row: commander, full width beside the shortcut hint.
                bottom
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(10.0),
                        padding: UiRect {
                            left: Val::Px(16.0),
                            right: Val::Px(16.0),
                            top: Val::Px(0.0),
                            bottom: Val::Px(6.0),
                        },
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            CommanderContainer,
                            Node {
                                flex_grow: 1.0,
                                height: Val::Px(COMMANDER_H),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(14.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(COMMANDER_H / 2.0)),
                                ..default()
                            },
                            BackgroundColor(theme::DARK_BASE),
                            BorderColor::all(theme::BORDER),
                            Interaction::default(),
                        ))
                        .with_children(|cmd| {
                            cmd.spawn((
                                CommanderPrompt,
                                Text::new("> "),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(theme::ACID_BLUE),
                            ));
                            cmd.spawn((
                                CommanderText,
                                Text::new("ask, search, transact..."),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(0.30, 0.30, 0.40, 0.55)),
                            ));
                        });

                        // The pay form's second field: appears when the
                        // commander folds into "to | amount".
                        row.spawn((
                            PayAmountBox,
                            Node {
                                width: Val::Px(220.0),
                                height: Val::Px(COMMANDER_H),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(14.0)),
                                border: UiRect::all(Val::Px(1.0)),
                                border_radius: BorderRadius::all(Val::Px(COMMANDER_H / 2.0)),
                                display: Display::None,
                                ..default()
                            },
                            BackgroundColor(theme::DARK_BASE),
                            BorderColor::all(theme::BORDER),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                PayAmountText,
                                Text::new("amount"),
                                TextFont {
                                    font_size: 14.0,
                                    ..default()
                                },
                                TextColor(Color::srgba(0.30, 0.30, 0.40, 0.55)),
                            ));
                        });

                        // Submit. On desktop Enter does this and the glyph is
                        // a hint; on Android the soft keyboard's "go" never
                        // reaches the app — GameTextInput consumes it — so
                        // this button is the only way to commit a line.
                        row.spawn((
                            CommanderSubmit,
                            Node {
                                width: Val::Px(COMMANDER_H),
                                height: Val::Px(COMMANDER_H),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                border_radius: BorderRadius::all(Val::Px(COMMANDER_H / 2.0)),
                                ..default()
                            },
                            BackgroundColor(Color::srgba(0.21, 0.84, 0.68, 0.14)),
                            Interaction::default(),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new(">"),
                                TextFont {
                                    font_size: 17.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.21, 0.84, 0.68)),
                            ));
                        });
                    });

                // Lower row: the world tabs, spread across the full width so
                // each is a thumb-sized target.
                bottom
                    .spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(TABS_H),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceEvenly,
                            border: UiRect::top(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(theme::DARK_BASE),
                        BorderColor::all(theme::BORDER),
                    ))
                    .with_children(|tabs| {
                        for (label, world) in [
                            ("body", WorldState::Body),
                            ("brain", WorldState::Graph),
                            ("log", WorldState::Com),
                            ("robot", WorldState::Robot),
                            ("sigma", WorldState::Sigma),
                            ("models", WorldState::Models),
                            ("vault", WorldState::Vault),
                            ("memory", WorldState::Memory),
                            ("oracle", WorldState::Oracle),
                        ] {
                            tabs.spawn((
                                WorldNavButton(world),
                                Node {
                                    flex_grow: 1.0,
                                    height: Val::Percent(100.0),
                                    flex_direction: FlexDirection::Row,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::NONE),
                                Interaction::default(),
                            ))
                            .with_children(|btn| {
                                btn.spawn((
                                    Text::new(label),
                                    TextFont {
                                        font_size: 14.0,
                                        ..default()
                                    },
                                    TextColor(Color::srgba(0.21, 0.84, 0.68, 0.55)),
                                ));
                            });
                        }
                    });
            });
        });
}

// ── Interaction handlers ────────────────────────────────────────────────────

fn handle_commander_click(
    q: Query<&Interaction, (Changed<Interaction>, With<CommanderContainer>)>,
    mut chrome: ResMut<ChromeState>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed {
            chrome.focused = true;
        }
    }
}

fn handle_commander_submit(
    q: Query<&Interaction, (Changed<Interaction>, With<CommanderSubmit>)>,
    mut chrome: ResMut<ChromeState>,
) {
    for interaction in &q {
        if *interaction == Interaction::Pressed && !chrome.text.trim().is_empty() {
            chrome.submit_now = true;
        }
    }
}

fn handle_world_buttons(
    mut q: Query<(&Interaction, &WorldNavButton, &mut BackgroundColor), Changed<Interaction>>,
    current: Res<State<WorldState>>,
    mut next: ResMut<NextState<WorldState>>,
) {
    for (interaction, button, mut bg) in &mut q {
        match interaction {
            Interaction::Pressed => {
                if *current.get() != button.0 {
                    next.set(button.0);
                }
            }
            Interaction::Hovered => {
                *bg = BackgroundColor(Color::srgba(0.21, 0.84, 0.68, 0.08));
            }
            Interaction::None => {
                *bg = BackgroundColor(Color::NONE);
            }
        }
    }
}

// ── Focus style sync ────────────────────────────────────────────────────────

/// The commander's fill is black in both states — focus reads on the outline.
fn sync_commander_focus_style(
    chrome: Res<ChromeState>,
    q: Query<Entity, With<CommanderContainer>>,
    mut commands: Commands,
) {
    if !chrome.is_changed() {
        return;
    }
    for entity in &q {
        if chrome.focused {
            commands.entity(entity).insert(Outline {
                width: Val::Px(1.0),
                offset: Val::Px(0.0),
                color: Color::srgba(0.96, 0.17, 0.99, 0.65),
            });
        } else {
            commands.entity(entity).remove::<Outline>();
        }
    }
}

/// Grow the bars by whatever the system reserves, so the tab strip clears the
/// gesture pill and the address bar clears the status bar.
fn apply_safe_area(
    safe: Res<SafeArea>,
    mut top: Query<
        &mut Node,
        (
            With<ChromeTopBar>,
            Without<ChromeBottomBar>,
            Without<ContentRoot>,
        ),
    >,
    mut bottom: Query<
        &mut Node,
        (
            With<ChromeBottomBar>,
            Without<ChromeTopBar>,
            Without<ContentRoot>,
        ),
    >,
    mut content: Query<
        &mut Node,
        (
            With<ContentRoot>,
            Without<ChromeTopBar>,
            Without<ChromeBottomBar>,
        ),
    >,
) {
    // Writing Node marks it Changed even when the Val is equal, and Bevy
    // then relayouts the whole UI. On a phone that is a jerk every frame.
    let top_h = Val::Px(CHROME_TOP_H + safe.top);
    let top_pad = Val::Px(safe.top);
    let bot_pad = Val::Px(safe.bottom);
    let content_top = Val::Px(CHROME_TOP_H + safe.top);
    let content_bot = Val::Px(CHROME_BOTTOM_H + safe.bottom);
    for mut node in &mut top {
        if node.height != top_h {
            node.height = top_h;
        }
        if node.padding.top != top_pad {
            node.padding.top = top_pad;
        }
    }
    for mut node in &mut bottom {
        if node.padding.bottom != bot_pad {
            node.padding.bottom = bot_pad;
        }
    }
    for mut node in &mut content {
        if node.top != content_top {
            node.top = content_top;
        }
        if node.bottom != content_bot {
            node.bottom = content_bot;
        }
    }
}

#[derive(Component)]
struct IdentityLabel;

/// The top-right corner says who this cyb is — the short form of the
/// owner's address, from the mnemonic, not a hardcoded word.
fn show_identity(
    who: Res<crate::worlds::identity::Identity>,
    mut q: Query<&mut Text, With<IdentityLabel>>,
) {
    if !who.is_changed() {
        return;
    }
    for mut t in &mut q {
        **t = who.short();
    }
}

#[derive(Component)]
struct NoticeBand;

#[derive(Component)]
struct NoticeText;

/// Show the current [`Notice`] under the address bar, then let it expire.
///
/// It follows the safe area like everything else in the chrome: the address
/// bar grows by the status-bar inset, and the band sits under whatever height
/// that leaves.
fn show_notice(
    time: Res<Time>,
    safe: Res<SafeArea>,
    mut notice: ResMut<Notice>,
    mut band: Query<&mut Node, With<NoticeBand>>,
    mut text: Query<&mut Text, With<NoticeText>>,
) {
    if notice.ttl > 0.0 {
        notice.ttl = (notice.ttl - time.delta_secs()).max(0.0);
    }
    let showing = notice.ttl > 0.0;

    let band_top = Val::Px(CHROME_TOP_H + safe.top);
    let band_display = if showing {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut band {
        if node.top != band_top {
            node.top = band_top;
        }
        if node.display != band_display {
            node.display = band_display;
        }
    }
    if showing {
        for mut t in &mut text {
            if t.0 != notice.text {
                **t = notice.text.clone();
            }
        }
    }
}

/// The commander is the only text surface in the chrome: focus it and the soft
/// keyboard comes up, leave it and the keyboard goes away.
#[cfg_attr(not(target_os = "android"), allow(unused_mut))]
fn request_soft_input(mut chrome: ResMut<ChromeState>, mut input: ResMut<SoftInput>) {
    if input.wanted != chrome.focused {
        input.wanted = chrome.focused;
    }
    // On Android the soft keyboard's text arrives through GameTextInput, not
    // as key events, so the IME buffer is the commander's line while focused.
    // Enter still comes through as a key event and `handle_chrome_input`
    // submits from `chrome.text` — which is why this mirrors rather than
    // appends.
    #[cfg(target_os = "android")]
    if chrome.focused && chrome.text != input.text {
        // The IME marks "go" with a newline. Everything before it is the
        // line; seeing one is Enter, since the key event itself is consumed
        // by GameTextInput and never reaches Bevy.
        if let Some(line) = input
            .text
            .split('\n')
            .next()
            .filter(|_| input.text.contains('\n'))
        {
            chrome.text = line.to_string();
            chrome.submit_now = true;
        } else {
            chrome.text = input.text.clone();
        }
    }
}

/// The commander shows com's prompt. Before com has ever run there is no
/// shell to ask, so the plain marker stands in.
fn sync_commander_prompt(prompt: Res<ComPrompt>, mut q: Query<&mut Text, With<CommanderPrompt>>) {
    if !prompt.is_changed() {
        return;
    }
    let shown = if prompt.0.is_empty() {
        "> ".to_string()
    } else {
        prompt.0.clone()
    };
    for mut text in &mut q {
        if **text != shown {
            **text = shown.clone();
        }
    }
}

fn update_address_bar(
    world_state: Res<State<WorldState>>,
    now: Res<crate::now::Now>,
    mut q: Query<&mut Text, With<AddressBarText>>,
) {
    if !world_state.is_changed() && !now.is_changed() {
        return;
    }
    let uri = match now.kind {
        crate::now::NowKind::Meta => now
            .hash
            .map(|p| format!("cyb://particle/{}", p.to_hex()))
            .unwrap_or_else(|| "cyb://particle".into()),
        crate::now::NowKind::File => now
            .hash
            .map(|p| format!("cyb://file/{}", p.to_hex()))
            .unwrap_or_else(|| "cyb://file".into()),
        crate::now::NowKind::World => match world_state.get() {
            WorldState::Body => "cyb://body".into(),
            WorldState::Graph => "cyb://brain".into(),
            WorldState::Com => "cyb://log".into(),
            WorldState::Robot => "cyb://robot".into(),
            WorldState::Sigma => "cyb://sigma".into(),
            WorldState::Models => "cyb://models".into(),
            WorldState::Vault => "cyb://vault".into(),
            WorldState::Memory => "cyb://memory".into(),
            WorldState::Oracle => "cyb://oracle".into(),
        },
    };
    for mut text in &mut q {
        **text = uri.clone();
    }
}

fn update_commander_display(
    chrome: Res<ChromeState>,
    world_state: Res<State<crate::worlds::WorldState>>,
    mut q: Query<(&mut Text, &mut TextColor), With<CommanderText>>,
    mut prompt_q: Query<
        &mut Text,
        (
            With<CommanderPrompt>,
            Without<CommanderText>,
            Without<PayAmountText>,
        ),
    >,
    mut amount_box: Query<&mut Node, With<PayAmountBox>>,
    mut amount_q: Query<(&mut Text, &mut TextColor), (With<PayAmountText>, Without<CommanderText>)>,
) {
    if !chrome.is_changed() && !world_state.is_changed() {
        return;
    }
    // `selected`: Cmd+A marked the whole field selected — no caret, a
    // bright highlight instead, the same signal a real text field gives.
    let cursor = |s: &str, active: bool, selected: bool| -> String {
        if active && selected {
            if s.is_empty() {
                " ".into()
            } else {
                s.to_string()
            }
        } else if active {
            format!("{s}_")
        } else if s.is_empty() {
            " ".into()
        } else {
            s.to_string()
        }
    };
    if let Some(pay) = &chrome.pay {
        // The commander is a pay form: field one is the recipient,
        // field two the amount. Enter walks forward, Escape backs out.
        for mut t in &mut prompt_q {
            **t = "pay to ".into();
        }
        for (mut text, mut color) in &mut q {
            **text = if pay.to.is_empty() && pay.field != 0 {
                "recipient".into()
            } else {
                cursor(&pay.to, pay.field == 0, pay.selected)
            };
            *color = TextColor(if pay.field == 0 && pay.selected {
                theme::ACID_YELLOW
            } else if pay.field == 0 {
                Color::srgb(0.95, 0.95, 0.95)
            } else {
                Color::srgba(0.6, 0.6, 0.65, 0.9)
            });
        }
        for mut node in &mut amount_box {
            node.display = Display::Flex;
        }
        for (mut text, mut color) in &mut amount_q {
            **text = if pay.amount.is_empty() && pay.field != 1 {
                "amount".into()
            } else {
                cursor(&pay.amount, pay.field == 1, pay.selected)
            };
            *color = TextColor(if pay.field == 1 && pay.selected {
                theme::ACID_YELLOW
            } else if pay.field == 1 {
                Color::srgb(0.95, 0.95, 0.95)
            } else {
                Color::srgba(0.6, 0.6, 0.65, 0.9)
            });
        }
        return;
    }
    for mut t in &mut prompt_q {
        **t = "> ".into();
    }
    for mut node in &mut amount_box {
        node.display = Display::None;
    }
    for (mut text, mut color) in &mut q {
        if chrome.focused {
            let display = if chrome.selected {
                if chrome.text.is_empty() {
                    " ".to_string()
                } else {
                    chrome.text.clone()
                }
            } else if chrome.text.is_empty() {
                "_".to_string()
            } else {
                format!("{}_", chrome.text)
            };
            **text = display;
            *color = TextColor(if chrome.selected {
                theme::ACID_YELLOW
            } else {
                Color::srgb(0.95, 0.95, 0.95)
            });
        } else {
            // The robot's landing asks for the one act that matters there.
            **text = if *world_state.get() == crate::worlds::WorldState::Robot {
                "name me:  name <your choice>".to_string()
            } else {
                "ask, search, transact...".to_string()
            };
            *color = TextColor(Color::srgba(0.30, 0.30, 0.40, 0.55));
        }
    }
}

// ── Keyboard input (exclusive system) ──────────────────────────────────────

pub fn handle_chrome_input(world: &mut World) {
    let cmd_held = {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        keys.pressed(KeyCode::SuperLeft) || keys.pressed(KeyCode::SuperRight)
    };

    // Cmd+K: focus commander
    {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        if cmd_held && keys.just_pressed(KeyCode::KeyK) {
            let mut chrome = world.resource_mut::<ChromeState>();
            chrome.focused = true;
            chrome.text.clear();
            chrome.selected = false;
            return;
        }
    }

    let focused = world.resource::<ChromeState>().focused;
    if !focused {
        return;
    }

    // Cmd+A: select the active field — the pay form's current field, or
    // the plain commander text. The next keystroke or paste replaces it
    // whole, same as any real text field's select-all.
    {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        if cmd_held && keys.just_pressed(KeyCode::KeyA) {
            let mut chrome = world.resource_mut::<ChromeState>();
            if let Some(pay) = chrome.pay.as_mut() {
                pay.selected = true;
            } else {
                chrome.selected = true;
            }
            return;
        }
    }

    // Cmd+C: copy the active field's whole content — there is no partial
    // selection here, only select-all, so copy always takes everything.
    {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        if cmd_held && keys.just_pressed(KeyCode::KeyC) {
            let chrome = world.resource::<ChromeState>();
            let text = match &chrome.pay {
                Some(pay) if pay.field == 1 => pay.amount.clone(),
                Some(pay) => pay.to.clone(),
                None => chrome.text.clone(),
            };
            if !text.is_empty() {
                let _ = write_clipboard(&text);
            }
            return;
        }
    }

    // Cmd+V: paste the clipboard into whichever field is active — the pay
    // form's recipient/amount when it is open, the plain commander text
    // otherwise. A prior Cmd+A replaces; anything else appends. Single
    // line: newlines fold to a space, control characters are dropped.
    {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        if cmd_held && keys.just_pressed(KeyCode::KeyV) {
            if let Ok(text) = read_clipboard() {
                let mut chrome = world.resource_mut::<ChromeState>();
                if let Some(pay) = chrome.pay.as_mut() {
                    let selected = pay.selected;
                    pay.selected = false;
                    if pay.field == 1 {
                        if selected {
                            pay.amount.clear();
                        }
                        pay.amount
                            .extend(text.chars().filter(|c| c.is_ascii_digit()));
                    } else {
                        if selected {
                            pay.to.clear();
                        }
                        pay.to.extend(
                            text.chars()
                                .filter(|c| !c.is_whitespace() && !c.is_control()),
                        );
                    }
                } else {
                    if chrome.selected {
                        chrome.text.clear();
                        chrome.selected = false;
                    }
                    for ch in text.chars() {
                        if ch == '\n' || ch == '\r' {
                            chrome.text.push(' ');
                        } else if !ch.is_control() {
                            chrome.text.push(ch);
                        }
                    }
                }
            }
            return;
        }
    }

    let mut cursor = world.resource::<ChromeState>().key_cursor.clone();
    let events: Vec<KeyboardInput> = {
        let messages = world.resource::<bevy::ecs::message::Messages<KeyboardInput>>();
        cursor.read(messages).cloned().collect()
    };
    world.resource_mut::<ChromeState>().key_cursor = cursor;

    let mut pending_cmd: Option<String> = None;

    // A submit that arrived as text rather than as a key press.
    {
        let mut chrome = world.resource_mut::<ChromeState>();
        if chrome.submit_now {
            chrome.submit_now = false;
            pending_cmd = Some(chrome.text.trim().to_string());
            chrome.text.clear();
            chrome.focused = false;
        }
    }

    {
        let mut chrome = world.resource_mut::<ChromeState>();
        for event in &events {
            if !event.state.is_pressed() {
                continue;
            }
            if cmd_held {
                continue;
            }
            // The pay form owns the keys while it is open: characters land
            // in the active field, Enter walks to the next field and then
            // submits, Escape folds the form away.
            if chrome.pay.is_some() {
                let mut submit: Option<String> = None;
                {
                    let pay = chrome.pay.as_mut().expect("checked");
                    match &event.logical_key {
                        Key::Enter | Key::Tab => {
                            if pay.field == 0 && !pay.to.is_empty() {
                                pay.field = 1;
                                pay.selected = false;
                            } else if pay.field == 1 && !pay.amount.is_empty() {
                                submit =
                                    Some(format!("pay {} {}", pay.to.trim(), pay.amount.trim()));
                            }
                        }
                        Key::Escape => {
                            chrome.pay = None;
                            chrome.focused = false;
                            continue;
                        }
                        Key::Backspace => {
                            if pay.selected {
                                if pay.field == 1 {
                                    pay.amount.clear();
                                } else {
                                    pay.to.clear();
                                }
                                pay.selected = false;
                            } else if pay.field == 1 && pay.amount.is_empty() {
                                pay.field = 0;
                            } else if pay.field == 1 {
                                pay.amount.pop();
                            } else {
                                pay.to.pop();
                            }
                        }
                        Key::Space if pay.field == 0 => {
                            // Addresses and labels carry no spaces; a space
                            // is a step to the amount.
                            if !pay.to.is_empty() {
                                pay.field = 1;
                                pay.selected = false;
                            }
                        }
                        Key::Character(c) => {
                            if pay.selected {
                                if pay.field == 1 {
                                    pay.amount.clear();
                                } else {
                                    pay.to.clear();
                                }
                                pay.selected = false;
                            }
                            for ch in c.chars().filter(|ch| !ch.is_control()) {
                                if pay.field == 1 {
                                    if ch.is_ascii_digit() {
                                        pay.amount.push(ch);
                                    }
                                } else if !ch.is_whitespace() {
                                    pay.to.push(ch);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if let Some(cmd) = submit {
                    pending_cmd = Some(cmd);
                    chrome.pay = None;
                    chrome.focused = false;
                    break;
                }
                continue;
            }
            match &event.logical_key {
                Key::Enter => {
                    pending_cmd = Some(chrome.text.trim().to_string());
                    chrome.text.clear();
                    chrome.focused = false;
                    break;
                }
                Key::Escape => {
                    chrome.text.clear();
                    chrome.focused = false;
                    break;
                }
                Key::Backspace => {
                    if chrome.selected {
                        chrome.text.clear();
                        chrome.selected = false;
                    } else {
                        chrome.text.pop();
                    }
                }
                Key::Character(c) => {
                    if chrome.selected {
                        chrome.text.clear();
                        chrome.selected = false;
                    }
                    chrome.text.push_str(c.as_str());
                }
                Key::Space => {
                    if chrome.selected {
                        chrome.text.clear();
                        chrome.selected = false;
                    }
                    chrome.text.push(' ');
                }
                _ => {}
            }
        }
    }

    if let Some(cmd) = pending_cmd {
        world.resource_mut::<ChromeState>().just_submitted = true;

        let target = match cmd.as_str() {
            // The machine itself — the main page.
            "body" | "machine" => Some(WorldState::Body),
            // The surface is the brain; "graph" and "mir" are what it
            // was called before, and both still land here.
            "brain" | "graph" | "mir" => Some(WorldState::Graph),
            // The world is the log — the record. "com" stays as a spoken
            // alias: it names the commander, the input that feeds this world.
            "log" | "com" | "terminal" => Some(WorldState::Com),
            "robot" | "cell" | "landing" => Some(WorldState::Robot),
            "sigma" | "money" => Some(WorldState::Sigma),
            "models" | "mind" => Some(WorldState::Models),
            "vault" | "secrets" => Some(WorldState::Vault),
            "memory" | "files" => Some(WorldState::Memory),
            "oracle" | "blocks" | "explorer" => Some(WorldState::Oracle),
            _ => None,
        };
        if let Some(t) = target {
            if let Some(mut now) = world.get_resource_mut::<crate::now::Now>() {
                now.kind = crate::now::NowKind::World;
            }
            world.resource_mut::<NextState<WorldState>>().set(t);
        } else if let Some(rest) = cmd
            .strip_prefix("cyb://particle/")
            .or_else(|| cmd.strip_prefix("particle/"))
        {
            if let Some(p) = file::Particle::from_hex(rest) {
                if let Some(mut now) = world.get_resource_mut::<crate::now::Now>() {
                    now.stand(*p.as_bytes(), None);
                    now.kind = crate::now::NowKind::Meta;
                }
            }
        } else if let Some(rest) = cmd
            .strip_prefix("cyb://file/")
            .or_else(|| cmd.strip_prefix("file/"))
        {
            if let Some(p) = file::Particle::from_hex(rest) {
                if let Some(mut now) = world.get_resource_mut::<crate::now::Now>() {
                    now.stand(*p.as_bytes(), None);
                    if now.kind == crate::now::NowKind::Meta {
                        // explicit file URL without a spark still shows meta
                    }
                }
            }
        } else if !cmd.is_empty() {
            // Anything that is not a world name is a shell line: com runs it
            // and holds the history, so submitting from any world lands there.
            world
                .resource_mut::<NextState<WorldState>>()
                .set(WorldState::Com);
            if let Some(mut p) = world.get_resource_mut::<crate::worlds::PendingShellCmd>() {
                p.0 = Some(cmd);
            }
        }
    }
}
