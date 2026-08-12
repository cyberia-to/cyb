use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;

use crate::shell::clipboard::read_clipboard;
use crate::worlds::WorldState;

/// Logical-pixel heights of the persistent chrome bars (address bar top, commander bottom).
pub const CHROME_TOP_H: f32 = 36.0;
pub const CHROME_BOTTOM_H: f32 = 56.0; // commander 40px + bottom padding 8px + safe 8px

#[derive(Resource)]
pub struct ChromeState {
    pub focused: bool,
    pub just_submitted: bool, // true for one frame after commander Enter
    pub text: String,
    key_cursor: bevy::ecs::message::MessageCursor<KeyboardInput>,
}

impl Default for ChromeState {
    fn default() -> Self {
        Self {
            focused: false,
            just_submitted: false,
            text: String::new(),
            key_cursor: Default::default(),
        }
    }
}

#[derive(Component)]
struct ChromeCamera;
#[derive(Component)]
struct AddressBarText;
#[derive(Component)]
pub struct CommanderContainer;
#[derive(Component)]
struct CommanderText;
#[derive(Component)]
struct GraphNavButton;

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
                    handle_graph_button,
                    handle_chrome_input,
                    update_address_bar,
                    update_commander_display,
                    sync_commander_focus_style,
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
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(36.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.92)),
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

            // ── Bottom row: [graph ↗] | [commander 62%] | [⌘K] ─────────
            root.spawn(Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect {
                    left: Val::Px(16.0),
                    right: Val::Px(16.0),
                    bottom: Val::Px(8.0),
                    top: Val::Px(0.0),
                },
                ..default()
            })
            .with_children(|row| {
                // Left: graph navigation link
                row.spawn((
                    GraphNavButton,
                    Node {
                        flex_grow: 1.0,
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    Interaction::default(),
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("↗ mir"),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.21, 0.84, 0.68, 0.55)),
                    ));
                });

                // Center: commander input — 62% width
                row.spawn((
                    CommanderContainer,
                    Node {
                        width: Val::Percent(62.0),
                        height: Val::Px(40.0),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(14.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.10, 0.94)),
                    Interaction::default(),
                ))
                .with_children(|cmd| {
                    cmd.spawn((
                        Text::new("> "),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.21, 0.84, 0.68)),
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

                // Right: shortcut hint
                row.spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::FlexEnd,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|right| {
                    right.spawn((
                        Text::new("⌘K"),
                        TextFont {
                            font_size: 12.0,
                            ..default()
                        },
                        TextColor(Color::srgba(0.35, 0.35, 0.45, 0.55)),
                    ));
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

fn handle_graph_button(
    mut q: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<GraphNavButton>),
    >,
    current: Res<State<WorldState>>,
    mut next: ResMut<NextState<WorldState>>,
) {
    for (interaction, mut bg) in &mut q {
        match interaction {
            Interaction::Pressed => {
                if *current.get() != WorldState::Graph {
                    next.set(WorldState::Graph);
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

fn sync_commander_focus_style(
    chrome: Res<ChromeState>,
    mut q: Query<(Entity, &mut BackgroundColor), With<CommanderContainer>>,
    mut commands: Commands,
) {
    if !chrome.is_changed() {
        return;
    }
    for (entity, mut bg) in &mut q {
        if chrome.focused {
            *bg = BackgroundColor(Color::srgba(0.08, 0.07, 0.16, 0.98));
            commands.entity(entity).insert(Outline {
                width: Val::Px(1.0),
                offset: Val::Px(0.0),
                color: Color::srgba(0.96, 0.17, 0.99, 0.65),
            });
        } else {
            *bg = BackgroundColor(Color::srgba(0.05, 0.05, 0.10, 0.94));
            commands.entity(entity).remove::<Outline>();
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
