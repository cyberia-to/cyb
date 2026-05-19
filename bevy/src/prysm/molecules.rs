use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use super::theme;
use super::atoms::{GlassDepth, glass_bg, saber_h};

#[derive(Component)]
pub struct TabItem {
    pub index: usize,
}

#[derive(Component)]
pub struct Commander;

#[derive(Resource, Default)]
pub struct ActiveTab(pub usize);

pub fn spawn_commander(
    parent: &mut ChildSpawnerCommands,
    active_tab: usize,
    labels: &[&str],
) {
    parent
        .spawn((
            Commander,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|c| {
            saber_h(c);
            c.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    padding: UiRect::all(Val::Px(theme::G)),
                    column_gap: Val::Px(theme::G),
                    ..default()
                },
                glass_bg(GlassDepth::Background),
            ))
            .with_children(|row| {
                for (i, &label) in labels.iter().enumerate() {
                    row.spawn((
                        TabItem { index: i },
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            padding: UiRect::axes(Val::Px(theme::G * 2.0), Val::Px(theme::G * 0.5)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|tab| {
                        tab.spawn((
                            Text::new(label),
                            TextFont { font_size: theme::CAPTION, ..default() },
                            TextColor(if i == active_tab {
                                theme::TEXT_PRIMARY
                            } else {
                                theme::TEXT_DIM
                            }),
                        ));
                        if i == active_tab {
                            tab.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Px(2.0),
                                    margin: UiRect::top(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(theme::ACID_BLUE),
                            ));
                        }
                    });
                }
            });
        });
}

#[derive(Component)]
pub struct ButtonPrysm;

pub fn spawn_button(parent: &mut ChildSpawnerCommands, label: &str) -> Entity {
    parent
        .spawn((
            ButtonPrysm,
            Button,
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Px(theme::G * 2.0), Val::Px(theme::G)),
                ..default()
            },
            glass_bg(GlassDepth::Midground),
        ))
        .with_children(|b| {
            saber_h(b);
            b.spawn((
                Text::new(label),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(theme::TEXT_PRIMARY),
            ));
            saber_h(b);
        })
        .id()
}

#[derive(Component)]
pub struct TextInput {
    pub value:   String,
    pub focused: bool,
}

#[derive(Component)]
pub struct CursorBlink(pub f32);

pub fn text_input_system(
    mut keyboard_events: MessageReader<KeyboardInput>,
    mut q:               Query<(&mut TextInput, &mut Text)>,
) {
    for (mut input, mut text) in &mut q {
        if !input.focused { continue; }

        for ev in keyboard_events.read() {
            if !ev.state.is_pressed() { continue; }
            match (&ev.logical_key, &ev.text) {
                (Key::Backspace, _) => {
                    input.value.pop();
                }
                (Key::Enter, _) => {}
                (_, Some(t)) => {
                    for ch in t.chars() {
                        if !ch.is_ascii_control() {
                            input.value.push(ch);
                        }
                    }
                }
                _ => {}
            }
        }

        let display = format!("{}|", input.value);
        **text = display;
    }
}

pub fn spawn_input(parent: &mut ChildSpawnerCommands, placeholder: &str) -> Entity {
    let placeholder = placeholder.to_string();
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(theme::G)),
                ..default()
            },
            glass_bg(GlassDepth::Midground),
        ))
        .with_children(|p| {
            p.spawn((
                TextInput { value: String::new(), focused: false },
                Text::new(format!("{}|", placeholder)),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(theme::TEXT_DIM),
            ));
        })
        .id()
}
