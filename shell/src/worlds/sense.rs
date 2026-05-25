use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;

use super::WorldState;
use crate::prysm::{
    theme,
    atoms::{GlassDepth, glass_bg},
    molecules::{spawn_commander, spawn_button, TextInput},
};

pub struct SenseWorldPlugin;

#[derive(Component)]
struct SenseMarker;

#[derive(Component)]
struct MessageItem {
    is_user: bool,
}

#[derive(Component)]
struct MessageText;

#[derive(Component)]
struct SendButton;

#[derive(Component)]
struct SenseInputMarker;

#[derive(Component)]
struct MessagesContainer;

#[derive(Component)]
struct GliaStatus;

#[derive(Component)]
struct GliaTask(Task<String>);

#[derive(Resource, Default)]
struct ChatHistory {
    pub messages: Vec<ChatMessage>,
}

#[derive(Clone)]
struct ChatMessage {
    pub text:    String,
    pub is_user: bool,
}

#[derive(Component)]
struct InferenceTask(pub Task<Option<String>>);

impl Plugin for SenseWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatHistory>()
            .add_systems(OnEnter(WorldState::Sense), show_sense)
            .add_systems(OnExit(WorldState::Sense), hide_sense)
            .add_systems(
                Update,
                (handle_send, poll_inference_tasks, update_chat_display, poll_glia_task)
                    .run_if(in_state(WorldState::Sense)),
            );
    }
}

fn show_sense(mut commands: Commands, history: Res<ChatHistory>) {
    let glia_task = AsyncComputeTaskPool::get().spawn(async move {
        call_model_check()
    });

    commands
        .spawn((
            SenseMarker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Px(theme::G * 2.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|h| {
                h.spawn((
                    Text::new("sense"),
                    TextFont { font_size: theme::H2, ..default() },
                    TextColor(theme::TEXT_PRIMARY),
                ));
            });

            root.spawn((
                MessagesContainer,
                Node {
                    flex_grow: 1.0,
                    width: Val::Percent(100.0),
                    max_width: Val::Px(640.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(theme::G),
                    overflow: Overflow::scroll_y(),
                    padding: UiRect::all(Val::Px(theme::G)),
                    ..default()
                },
                glass_bg(GlassDepth::Background),
            ))
            .with_children(|msgs| {
                for msg in &history.messages {
                    spawn_message(msgs, msg.text.clone(), msg.is_user);
                }
            });

            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    max_width: Val::Px(640.0),
                    row_gap: Val::Px(theme::G * 0.5),
                    margin: UiRect::vertical(Val::Px(theme::G)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|col| {
                col.spawn((
                    GliaStatus,
                    Text::new("checking glia…"),
                    TextFont { font_size: theme::MICRO, ..default() },
                    TextColor(theme::TEXT_DIM),
                ));

                col.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(theme::G),
                        width: Val::Percent(100.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        SenseInputMarker,
                        Button,
                        TextInput {
                            value: String::new(),
                            focused: true,
                            placeholder: "ask anything…".to_string(),
                        },
                        Node {
                            flex_grow: 1.0,
                            padding: UiRect::all(Val::Px(theme::G)),
                            ..default()
                        },
                        glass_bg(GlassDepth::Midground),
                    ))
                    .with_children(|p| {
                        p.spawn((
                            Text::new("ask anything…|"),
                            TextFont { font_size: theme::BODY, ..default() },
                            TextColor(theme::TEXT_DIM),
                        ));
                    });

                    let btn = spawn_button(bar, "send");
                    bar.commands().entity(btn).insert(SendButton);
                });
            });

            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|footer| {
                spawn_commander(footer, 2, &["spells", "graph", "sense"]);
            });
        });

    commands.spawn(GliaTask(glia_task));
}

fn hide_sense(mut commands: Commands, q: Query<Entity, With<SenseMarker>>, tasks: Query<Entity, With<GliaTask>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
    for e in &tasks {
        commands.entity(e).despawn();
    }
}

fn poll_glia_task(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut GliaTask)>,
    mut status_q: Query<&mut Text, With<GliaStatus>>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            if let Ok(mut text) = status_q.single_mut() {
                **text = result;
            }
            commands.entity(entity).despawn();
        }
    }
}

fn handle_send(
    send_q:       Query<&Interaction, (Changed<Interaction>, With<SendButton>)>,
    keys:         Res<ButtonInput<KeyCode>>,
    mut input_q:  Query<&mut TextInput, With<SenseInputMarker>>,
    mut history:  ResMut<ChatHistory>,
    mut commands: Commands,
) {
    let send_pressed = send_q.iter().any(|i| *i == Interaction::Pressed);
    let enter_pressed = keys.just_pressed(KeyCode::Enter);

    if !send_pressed && !enter_pressed {
        return;
    }

    let Ok(mut input) = input_q.single_mut() else { return };
    let prompt = input.value.trim().to_string();
    if prompt.is_empty() { return; }
    input.value.clear();

    history.messages.push(ChatMessage { text: prompt.clone(), is_user: true });
    history.messages.push(ChatMessage { text: "...".to_string(), is_user: false });

    let task = AsyncComputeTaskPool::get().spawn(async move {
        call_cyb_llm(prompt)
    });
    commands.spawn(InferenceTask(task));
}

fn poll_inference_tasks(
    mut commands: Commands,
    mut tasks:   Query<(Entity, &mut InferenceTask)>,
    mut history: ResMut<ChatHistory>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            let reply = result.unwrap_or_else(|| {
                "cyb-llm serve not running. start it with: cyb-llm serve".to_string()
            });
            if let Some(last) = history.messages.last_mut() {
                if !last.is_user {
                    last.text = reply;
                }
            }
            commands.entity(entity).despawn();
        }
    }
}

fn update_chat_display(
    history:     Res<ChatHistory>,
    container_q: Query<Entity, With<MessagesContainer>>,
    mut commands: Commands,
) {
    if !history.is_changed() { return; }
    let Ok(container) = container_q.single() else { return };

    commands.entity(container).despawn_related::<Children>();
    let messages = history.messages.clone();
    commands.entity(container).with_children(|msgs| {
        for msg in &messages {
            spawn_message(msgs, msg.text.clone(), msg.is_user);
        }
    });
}

fn spawn_message(parent: &mut ChildSpawnerCommands, text: String, is_user: bool) {
    parent
        .spawn((
            MessageItem { is_user },
            Node {
                padding: UiRect::all(Val::Px(theme::G)),
                align_self: if is_user {
                    AlignSelf::FlexEnd
                } else {
                    AlignSelf::FlexStart
                },
                max_width: Val::Percent(80.0),
                ..default()
            },
            glass_bg(GlassDepth::Subtle),
        ))
        .with_children(|p| {
            p.spawn((
                Text::new(text),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(if is_user { theme::TEXT_PRIMARY } else { theme::TEXT_DIM }),
            ));
        });
}

const TARGET_MODEL: &str = "qwen3:0.6b";

fn model_available() -> bool {
    let mut resp = match ureq::get("http://localhost:11434/v1/models").call().ok() {
        Some(r) => r,
        None => return false,
    };
    let json: serde_json::Value = resp.body_mut().read_json().ok().unwrap_or_default();
    json["data"].as_array()
        .map(|arr| arr.iter().any(|m| m["id"].as_str() == Some(TARGET_MODEL)))
        .unwrap_or(false)
}

fn call_model_check() -> String {
    match ureq::get("http://localhost:11434/v1/models").call() {
        Err(_) => "glia offline — run: ollama serve".to_string(),
        Ok(mut r) => {
            let json: serde_json::Value = r.body_mut().read_json().ok().unwrap_or_default();
            let has_target = json["data"].as_array()
                .map(|arr| arr.iter().any(|m| m["id"].as_str() == Some(TARGET_MODEL)))
                .unwrap_or(false);
            if has_target {
                format!("via glia • {}", TARGET_MODEL)
            } else {
                format!("model missing — run: ollama pull {}", TARGET_MODEL)
            }
        }
    }
}

fn call_cyb_llm(prompt: String) -> Option<String> {
    let model = TARGET_MODEL;
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": false
    });
    let mut resp = ureq::post("http://localhost:11434/v1/chat/completions")
        .header("Content-Type", "application/json")
        .send_json(body)
        .ok()?;
    let json: serde_json::Value = resp.body_mut().read_json().ok()?;
    json["choices"][0]["message"]["content"].as_str().map(|s| s.to_string())
}
