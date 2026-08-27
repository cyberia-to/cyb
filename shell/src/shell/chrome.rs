use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use prysm::theme;

use crate::shell::clipboard::read_clipboard;
use crate::shell::platform::{SafeArea, SoftInput};
use crate::worlds::WorldState;

/// Logical-pixel heights of the persistent chrome bars (address bar top, commander bottom).
pub const CHROME_TOP_H: f32 = 36.0;
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

/// The two chrome bars, so the safe-area system can pad them.
#[derive(Component)]
struct ChromeTopBar;
#[derive(Component)]
struct ChromeBottomBar;

#[derive(Component)]
struct ChromeCamera;
#[derive(Component)]
struct AddressBarText;
#[derive(Component)]
pub struct CommanderContainer;
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
            .add_systems(Startup, spawn_chrome)
            .add_systems(OnEnter(WorldState::Terminal), unfocus_chrome)
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
                    request_soft_input,
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
                    Text::new("cyb://graph"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(Color::srgba(0.55, 0.55, 0.70, 1.0)),
                ));
                bar.spawn((
                    Text::new("master"),
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
                                ..default()
                            },
                            BackgroundColor(theme::DARK_BASE),
                            BorderColor::all(theme::BORDER),
                            Interaction::default(),
                        ))
                        .with_children(|cmd| {
                            cmd.spawn((
                                Text::new("> "),
                                TextFont { font_size: 14.0, ..default() },
                                TextColor(Color::srgb(0.21, 0.84, 0.68)),
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
                                border: UiRect::all(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(theme::DARK_BASE),
                            BorderColor::all(theme::BORDER),
                            Interaction::default(),
                        ))
                        .with_children(|b| {
                            b.spawn((
                                Text::new("\u{25B8}"),
                                TextFont { font_size: 18.0, ..default() },
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
                            ("mir", WorldState::Graph),
                            ("term", WorldState::Terminal),
                            ("cell", WorldState::Cell),
                            ("sigma", WorldState::Sigma),
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
    mut top: Query<&mut Node, (With<ChromeTopBar>, Without<ChromeBottomBar>)>,
    mut bottom: Query<&mut Node, (With<ChromeBottomBar>, Without<ChromeTopBar>)>,
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
}

/// The commander is the only text surface in the chrome: focus it and the soft
/// keyboard comes up, leave it and the keyboard goes away.
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

fn update_address_bar(
    world_state: Res<State<WorldState>>,
    mut q: Query<&mut Text, With<AddressBarText>>,
) {
    if !world_state.is_changed() {
        return;
    }
    let uri = match world_state.get() {
        WorldState::Graph => "cyb://graph",
        WorldState::Terminal => "cyb://terminal",
        WorldState::Cell => "cyb://landing",
        WorldState::Sigma => "cyb://sigma",
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
            "graph" => Some(WorldState::Graph),
            "terminal" => Some(WorldState::Terminal),
            "landing" | "cell" => Some(WorldState::Cell),
            "sigma" | "money" => Some(WorldState::Sigma),
            _ => None,
        };
        if let Some(t) = target {
            world.resource_mut::<NextState<WorldState>>().set(t);
        } else if !cmd.is_empty() {
            // Unknown command → forward to nushell in terminal world
            world
                .resource_mut::<NextState<WorldState>>()
                .set(WorldState::Terminal);
            if let Some(mut p) = world.get_resource_mut::<crate::worlds::PendingShellCmd>() {
                p.0 = Some(cmd);
            }
        }
    }
}
