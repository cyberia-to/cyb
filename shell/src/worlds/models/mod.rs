//! The models world — which mind is running, and which minds are on disk.
//!
//! soma answers with whatever model it woke on, and until this page existed
//! the only way to know which one that was, was to read a log line. A mind
//! you cannot inspect or change from where you stand is somebody else's
//! mind. This page is the answer: the active model marked, the others one
//! press away, the switch taking effect on the next question — weights load
//! lazily, so switching costs nothing until it is used.
//!
//! The choice persists in `~/cyb/model` — device configuration, like
//! identity: which weights this body runs is a fact about the body, not
//! knowledge about the world, so it does not belong in the graph.

use bevy::prelude::*;
use prysm::theme;

use super::WorldState;
use crate::shell::chrome::{ContentRoot, CHROME_BOTTOM_H, CHROME_TOP_H};

pub struct ModelsWorldPlugin;

#[derive(Component)]
struct ModelsRoot;

/// A row that activates the model at this path when pressed.
#[derive(Component)]
struct ModelRow(std::path::PathBuf);

/// What the mind is running right now, and how the last answer went.
/// Updated by the soma bridge; `None` on bodies that carry no mind.
#[derive(Resource, Default)]
pub struct MindStatus {
    pub model: Option<std::path::PathBuf>,
    pub last_tok_per_s: Option<f32>,
}

impl Plugin for ModelsWorldPlugin {
    fn build(&self, app: &mut App) {
        // The status is a value, not a discovery — resolve it at build so the
        // page can never race a Startup system and open saying "no mind" on
        // a machine that has one.
        let status = MindStatus {
            #[cfg(target_os = "macos")]
            model: Some(soma_kernel::default_model_path()),
            #[cfg(not(target_os = "macos"))]
            model: None,
            last_tok_per_s: None,
        };
        app.insert_resource(status)
            .add_systems(OnEnter(WorldState::Models), build_page)
            .add_systems(OnExit(WorldState::Models), destroy_page)
            .add_systems(
                Update,
                (rebuild_on_change, handle_model_press).run_if(in_state(WorldState::Models)),
            );
    }
}

/// Every `.model` on this machine, active first, then by size — the model
/// you are most likely to reach for is the one you can afford to run.
fn models_on_disk(active: Option<&std::path::Path>) -> Vec<(std::path::PathBuf, u64)> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let dir = std::path::Path::new(&home).join("llm");
    let mut found: Vec<(std::path::PathBuf, u64)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) == Some("model") {
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                found.push((path, size));
            }
        }
    }
    // The active model belongs on the list even if it lives outside ~/llm
    // (SOMA_MODEL can point anywhere).
    if let Some(a) = active {
        if !found.iter().any(|(p, _)| p == a) && a.exists() {
            let size = std::fs::metadata(a).map(|m| m.len()).unwrap_or(0);
            found.push((a.to_path_buf(), size));
        }
    }
    found.sort_by_key(|(p, size)| (Some(p.as_path()) != active, *size));
    found
}

fn build_page(mut commands: Commands, status: Res<MindStatus>) {
    let root = commands
        .spawn((
            ModelsRoot,
            ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(CHROME_TOP_H),
                bottom: Val::Px(CHROME_BOTTOM_H),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
        ))
        .id();

    let page = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(theme::MEASURE),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(theme::G),
                padding: UiRect::all(Val::Px(theme::G * 2.0)),
                ..default()
            },
            ChildOf(root),
        ))
        .id();

    commands.spawn((
        Text::new("models"),
        TextFont { font_size: theme::H2, ..default() },
        TextColor(Color::srgb(0.7, 0.95, 0.8)),
        ChildOf(page),
    ));

    let status_line = match (&status.model, status.last_tok_per_s) {
        (Some(m), Some(v)) => format!(
            "mind: {}  /  last answer {v:.0} tok/s",
            file_label(m)
        ),
        (Some(m), None) => format!("mind: {}  /  wakes on the first question", file_label(m)),
        (None, _) => "this body carries no mind - models run on the desktop".into(),
    };
    commands.spawn((
        Text::new(status_line),
        TextFont { font_size: theme::CAPTION, ..default() },
        TextColor(theme::TEXT_DIM),
        ChildOf(page),
    ));

    let active = status.model.clone();
    let list = models_on_disk(active.as_deref());
    if list.is_empty() {
        commands.spawn((
            Text::new("no .model files in ~/llm - glia import builds them"),
            TextFont { font_size: theme::BODY, ..default() },
            TextColor(theme::TEXT_DIM),
            ChildOf(page),
        ));
    }
    for (path, size) in list {
        let is_active = active.as_deref() == Some(path.as_path());
        let row = commands
            .spawn((
                ModelRow(path.clone()),
                Button,
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(if is_active { theme::ACID_GREEN } else { theme::BORDER }),
            ))
            .insert(ChildOf(page))
            .id();
        commands.spawn((
            Text::new(file_label(&path)),
            TextFont { font_size: theme::BODY, ..default() },
            TextColor(if is_active { theme::ACID_GREEN } else { theme::TEXT_PRIMARY }),
            ChildOf(row),
        ));
        commands.spawn((
            Text::new(if is_active {
                format!("{}  active", human_size(size))
            } else {
                human_size(size)
            }),
            TextFont { font_size: theme::CAPTION, ..default() },
            TextColor(theme::TEXT_DIM),
            ChildOf(row),
        ));
    }
}

fn destroy_page(mut commands: Commands, q: Query<Entity, With<ModelsRoot>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

/// The page redraws itself when the mind's status moves under it — a switch
/// confirmed, a first answer timed.
fn rebuild_on_change(
    mut commands: Commands,
    status: Res<MindStatus>,
    roots: Query<Entity, With<ModelsRoot>>,
) {
    if !status.is_changed() || status.is_added() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_page(commands, status.into());
}

#[cfg_attr(not(target_os = "macos"), allow(unused_variables, unused_mut))]
fn handle_model_press(
    mut interactions: Query<(&Interaction, &ModelRow), Changed<Interaction>>,
    mut status: ResMut<MindStatus>,
    mut notice: ResMut<super::Notice>,
    #[cfg(target_os = "macos")] soma: NonSend<soma_kernel::Soma>,
) {
    for (interaction, row) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        #[cfg(target_os = "macos")]
        {
            // The choice outlives the session; the swap reaches the mind's
            // queue now and the weights load on the next question.
            let path = row.0.clone();
            if let Some(dir) = soma_kernel::chosen_model_file().parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(
                soma_kernel::chosen_model_file(),
                path.to_string_lossy().as_bytes(),
            );
            soma.use_model(&path);
            notice.show(format!(
                "mind: {} - wakes on the next question",
                file_label(&path)
            ));
            status.model = Some(path);
            status.last_tok_per_s = None;
        }
        #[cfg(not(target_os = "macos"))]
        {
            notice.show("this body carries no mind yet");
        }
    }
}

fn file_label(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string()
}

fn human_size(bytes: u64) -> String {
    let gb = bytes as f64 / 1e9;
    if gb >= 1.0 {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / 1e6)
    }
}
