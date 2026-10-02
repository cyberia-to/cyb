//! Chat — the conversation, apart from the log.
//!
//! The commander is the same on every world: nushell commands run as the
//! shell, questions come here. Log keeps the chain table; this page is only
//! what you said and what came back.

use bevy::prelude::*;
use prysm::theme;

use super::scroll::PersistScroll;
use super::{ComSay, Speaker, WorldState};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};

/// Talk waiting to be drawn. Same shape as com's inbox; a separate queue
/// so sigma events and vault notes stay in the log.
#[derive(Resource, Default)]
pub struct ChatInbox(pub Vec<ComSay>);

impl ChatInbox {
    pub fn say(&mut self, who: Speaker, line: impl Into<String>) {
        self.0.push(ComSay::Line(who, line.into()));
    }

    pub fn start_stream(&mut self) {
        self.0.push(ComSay::StreamStart);
    }

    pub fn stream_status(&mut self, line: impl Into<String>) {
        self.0.push(ComSay::StreamStatus(line.into()));
    }

    pub fn finish_stream(&mut self, text: impl Into<String>) {
        self.0.push(ComSay::StreamEnd(text.into()));
    }
}

#[derive(Component)]
struct ChatRoot;

#[derive(Component)]
struct ChatThread;

#[derive(Resource, Default)]
struct ChatFollow(bool);

#[derive(Resource, Default)]
struct ChatStream {
    entity: Option<Entity>,
    /// Tokens so far. Empty means we are still in load/prefill.
    buf: String,
    /// Shown while `buf` is empty.
    status: String,
    live: bool,
}

pub struct ChatWorldPlugin;

impl Plugin for ChatWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatInbox>()
            .init_resource::<ChatFollow>()
            .init_resource::<ChatStream>()
            .add_systems(OnEnter(WorldState::Chat), enter)
            .add_systems(Update, (drain_inbox, pulse_stream, stick_bottom).chain());
    }
}

fn enter(mut commands: Commands, mut worlds: Query<(&crate::worlds::WorldUi, &mut Visibility)>) {
    if crate::worlds::reveal_world(WorldState::Chat, &mut worlds) {
        return;
    }
    let root = commands
        .spawn((
            ChatRoot,
            crate::worlds::WorldUi(WorldState::Chat),
            ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(CHROME_TOP_H),
                bottom: Val::Px(CHROME_BOTTOM_H),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
        ))
        .id();

    let mut scroller = crate::worlds::page::scroll_column();
    scroller.flex_grow = 1.0;
    scroller.flex_shrink = 1.0;
    scroller.flex_basis = Val::Px(0.0);
    scroller.min_height = Val::Px(0.0);
    scroller.height = Val::Percent(100.0);
    let scroll = commands
        .spawn((
            scroller,
            ScrollPosition::default(),
            PersistScroll("chat"),
            ChildOf(root),
        ))
        .id();

    let thread = commands
        .spawn((
            ChatThread,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(theme::G * 1.5),
                flex_shrink: 0.0,
                padding: UiRect::bottom(Val::Px(theme::G * 2.0)),
                ..default()
            },
            ChildOf(scroll),
        ))
        .id();

    commands.spawn((
        Text::new("type in the commander — from any world"),
        TextFont {
            font_size: theme::CAPTION,
            ..default()
        },
        TextColor(theme::TEXT_DIM),
        ChildOf(thread),
    ));
}

fn drain_inbox(
    mut commands: Commands,
    mut inbox: ResMut<ChatInbox>,
    mut follow: ResMut<ChatFollow>,
    mut stream: ResMut<ChatStream>,
    thread: Query<Entity, With<ChatThread>>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
) {
    if inbox.0.is_empty() {
        return;
    }
    let Ok(thread) = thread.single() else {
        return;
    };
    let lines = std::mem::take(&mut inbox.0);
    for say in lines {
        match say {
            ComSay::Line(Speaker::User, text) => {
                spawn_bubble(&mut commands, thread, true, &text);
                follow.0 = true;
            }
            ComSay::Line(Speaker::System, text) | ComSay::Note(text) => {
                spawn_bubble(&mut commands, thread, false, &text);
                follow.0 = true;
            }
            ComSay::StreamStart => {
                stream.entity = Some(spawn_bubble(&mut commands, thread, false, "..."));
                stream.buf.clear();
                stream.status = "...".into();
                stream.live = true;
                follow.0 = true;
            }
            ComSay::StreamStatus(s) => {
                if stream.buf.is_empty() {
                    stream.status = s;
                    stream.live = true;
                }
                follow.0 = true;
            }
            ComSay::StreamDelta(d) => {
                stream.buf.push_str(&d);
                stream.live = true;
                follow.0 = true;
            }
            ComSay::StreamEnd(text) => {
                stream.live = false;
                stream.buf = text.clone();
                stream.status.clear();
                if let Some(e) = stream.entity.take() {
                    if let Ok((mut t, mut color)) = texts.get_mut(e) {
                        t.0 = text;
                        *color = TextColor(theme::TEXT_PRIMARY);
                    }
                } else {
                    spawn_bubble(&mut commands, thread, false, &text);
                }
                follow.0 = true;
            }
        }
    }
}

/// Rewrite the open bubble every frame so load/prefill is visibly alive:
/// a blinking cursor, even before the first token.
fn pulse_stream(
    time: Res<Time>,
    stream: Res<ChatStream>,
    mut follow: ResMut<ChatFollow>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
) {
    if !stream.live {
        return;
    }
    let Some(e) = stream.entity else {
        return;
    };
    let Ok((mut t, mut color)) = texts.get_mut(e) else {
        return;
    };
    follow.0 = true;
    let on = (time.elapsed_secs() * 2.5).fract() < 0.55;
    let cursor = if on { "▍" } else { " " };
    let (display, col) = if stream.buf.is_empty() {
        (format!("{} {}", stream.status, cursor), theme::TEXT_DIM)
    } else {
        (format!("{}{}", stream.buf, cursor), theme::TEXT_PRIMARY)
    };
    if t.0 != display {
        t.0 = display;
    }
    *color = TextColor(col);
}

fn spawn_bubble(commands: &mut Commands, thread: Entity, user: bool, text: &str) -> Entity {
    let row = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: if user {
                    JustifyContent::FlexStart
                } else {
                    JustifyContent::FlexEnd
                },
                ..default()
            },
            ChildOf(thread),
        ))
        .id();
    let bubble = commands
        .spawn((
            Node {
                max_width: Val::Percent(78.0),
                padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G)),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(if user {
                Color::srgba(0.13, 0.92, 0.51, 0.08)
            } else {
                Color::srgba(0.21, 0.84, 0.68, 0.10)
            }),
            BorderColor::all(if user {
                Color::srgba(0.13, 0.92, 0.51, 0.28)
            } else {
                Color::srgba(0.21, 0.84, 0.68, 0.35)
            }),
            ChildOf(row),
        ))
        .id();
    commands
        .spawn((
            Text::new(text.to_string()),
            TextFont {
                font_size: theme::BODY,
                ..default()
            },
            TextColor(if user {
                theme::TEXT_DIM
            } else {
                theme::TEXT_PRIMARY
            }),
            ChildOf(bubble),
        ))
        .id()
}

fn stick_bottom(
    mut follow: ResMut<ChatFollow>,
    mut q: Query<(&PersistScroll, &mut ScrollPosition, &ComputedNode)>,
) {
    if !follow.0 {
        return;
    }
    let mut done = false;
    for (p, mut pos, cn) in &mut q {
        if p.0 != "chat" {
            continue;
        }
        let s = cn.inverse_scale_factor();
        let max = (cn.content_size().y * s - cn.size().y * s).max(0.0);
        pos.y = max;
        done = max > 0.0;
    }
    if done {
        follow.0 = false;
    }
}
