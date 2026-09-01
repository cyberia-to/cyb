use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use prysm::theme;

use crate::shell::clipboard::read_clipboard;
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

#[derive(Resource)]
pub struct ChromeState {
    pub focused: bool,
    /// Set when something other than the keyboard decided the line is done —
    /// the soft keyboard's "go", which arrives as text rather than a key.
    pub submit_now: bool,
    pub just_submitted: bool, // true for one frame after commander Enter
    pub text: String,
    key_cursor: bevy::ecs::message::MessageCursor<KeyboardInput>,
}

impl Default for ChromeState {
    fn default() -> Self {
        Self {
            focused: false,
            submit_now: false,
            just_submitted: false,
            text: String::new(),
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
        ))
        .id();

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
                    TextFont { font_size: 13.0, ..default() },
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
                bar.spawn((
                    IdentityLabel,
                    Text::new(""),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.35, 0.35, 0.50, 0.8)),
                ));
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
                                TextFont { font_size: 14.0, ..default() },
                                TextColor(theme::ACID_BLUE),
                            ));
                            cmd.spawn((
                                CommanderText,
                                Text::new("ask, search, transact..."),
                                TextFont { font_size: 14.0, ..default() },
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
                                TextFont { font_size: 17.0, ..default() },
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
                            ("brain", WorldState::Graph),
                            ("com", WorldState::Com),
                            ("robot", WorldState::Robot),
                            ("sigma", WorldState::Sigma),
                            ("models", WorldState::Models),
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
                                    TextFont { font_size: 14.0, ..default() },
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
    mut q: Query<
        (&Interaction, &WorldNavButton, &mut BackgroundColor),
        Changed<Interaction>,
    >,
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
    mut top: Query<&mut Node, (With<ChromeTopBar>, Without<ChromeBottomBar>, Without<ContentRoot>)>,
    mut bottom: Query<&mut Node, (With<ChromeBottomBar>, Without<ChromeTopBar>, Without<ContentRoot>)>,
    mut content: Query<&mut Node, (With<ContentRoot>, Without<ChromeTopBar>, Without<ChromeBottomBar>)>,
) {
    // No is_changed() gate: SafeArea is written by another plugin, and if
    // that write landed after this system in the same frame the change would
    // be missed for good. Assigning an equal Val is free.
    for mut node in &mut top {
        node.height = Val::Px(CHROME_TOP_H + safe.top);
        node.padding.top = Val::Px(safe.top);
    }
    for mut node in &mut bottom {
        node.padding.bottom = Val::Px(safe.bottom);
    }
    // World roots live exactly between the bars — insets included, so
    // content never sits under either panel on any device.
    for mut node in &mut content {
        node.top = Val::Px(CHROME_TOP_H + safe.top);
        node.bottom = Val::Px(CHROME_BOTTOM_H + safe.bottom);
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
    time:      Res<Time>,
    safe:      Res<SafeArea>,
    mut notice: ResMut<Notice>,
    mut band:  Query<&mut Node, With<NoticeBand>>,
    mut text:  Query<&mut Text, With<NoticeText>>,
) {
    if notice.ttl > 0.0 {
        notice.ttl = (notice.ttl - time.delta_secs()).max(0.0);
    }
    let showing = notice.ttl > 0.0;

    for mut node in &mut band {
        node.top = Val::Px(CHROME_TOP_H + safe.top);
        node.display = if showing { Display::Flex } else { Display::None };
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
        if let Some(line) = input.text.split('\n').next().filter(|_| input.text.contains('\n')) {
            chrome.text = line.to_string();
            chrome.submit_now = true;
        } else {
            chrome.text = input.text.clone();
        }
    }
}

/// The commander shows com's prompt. Before com has ever run there is no
/// shell to ask, so the plain marker stands in.
fn sync_commander_prompt(
    prompt: Res<ComPrompt>,
    mut q: Query<&mut Text, With<CommanderPrompt>>,
) {
    if !prompt.is_changed() {
        return;
    }
    let shown = if prompt.0.is_empty() { "> ".to_string() } else { prompt.0.clone() };
    for mut text in &mut q {
        if **text != shown {
            **text = shown.clone();
        }
    }
}

fn update_address_bar(
    world_state: Res<State<WorldState>>,
    mut q: Query<&mut Text, With<AddressBarText>>,
) {
    if !world_state.is_changed() {
        return;
    }
    let uri = match world_state.get() {
        WorldState::Graph => "cyb://brain",
        WorldState::Com => "cyb://com",
        WorldState::Robot => "cyb://robot",
        WorldState::Sigma => "cyb://sigma",
        WorldState::Models => "cyb://models",
    };
    for mut text in &mut q {
        **text = uri.to_string();
    }
}

fn update_commander_display(
    chrome: Res<ChromeState>,
    mut q: Query<(&mut Text, &mut TextColor), With<CommanderText>>,
) {
    if !chrome.is_changed() {
        return;
    }
    for (mut text, mut color) in &mut q {
        if chrome.focused {
            let display = if chrome.text.is_empty() {
                "_".to_string()
            } else {
                format!("{}_", chrome.text)
            };
            **text = display;
            *color = TextColor(Color::srgb(0.95, 0.95, 0.95));
        } else {
            **text = "ask, search, transact...".to_string();
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
            return;
        }
    }

    let focused = world.resource::<ChromeState>().focused;
    if !focused {
        return;
    }

    // Cmd+V: paste clipboard into commander (single-line: strip newlines)
    {
        let keys = world.resource::<ButtonInput<KeyCode>>();
        if cmd_held && keys.just_pressed(KeyCode::KeyV) {
            if let Ok(text) = read_clipboard() {
                let mut chrome = world.resource_mut::<ChromeState>();
                for ch in text.chars() {
                    if ch == '\n' || ch == '\r' {
                        chrome.text.push(' ');
                    } else if !ch.is_control() {
                        chrome.text.push(ch);
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
                    chrome.text.pop();
                }
                Key::Character(c) => {
                    chrome.text.push_str(c.as_str());
                }
                Key::Space => {
                    chrome.text.push(' ');
                }
                _ => {}
            }
        }
    }

    if let Some(cmd) = pending_cmd {
        world.resource_mut::<ChromeState>().just_submitted = true;

        let target = match cmd.as_str() {
            // The surface is the brain; "graph" and "mir" are what it
            // was called before, and both still land here.
            "brain" | "graph" | "mir" => Some(WorldState::Graph),
            "com" | "terminal" => Some(WorldState::Com),
            "robot" | "cell" | "landing" => Some(WorldState::Robot),
            "sigma" | "money" => Some(WorldState::Sigma),
            "models" | "mind" => Some(WorldState::Models),
            _ => None,
        };
        if let Some(t) = target {
            world.resource_mut::<NextState<WorldState>>().set(t);
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
