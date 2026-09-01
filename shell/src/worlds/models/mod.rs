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

/// Minds this cyb knows how to fetch: HF repo → glia import → `~/llm`.
/// Every entry here has been walked end to end — downloaded, imported, and
/// answered a question — before being offered. A button that might work is
/// worse than no button.
const CATALOG: &[FetchEntry] = &[
    FetchEntry {
        label: "qwen3-0.6b",
        hf_id: "Qwen/Qwen3-0.6B",
        download: "1.4 GB download, 0.8 GB installed",
    },
    FetchEntry {
        label: "qwen3-1.7b",
        hf_id: "Qwen/Qwen3-1.7B",
        download: "3.4 GB download, 2.1 GB installed",
    },
];

struct FetchEntry {
    label: &'static str,
    hf_id: &'static str,
    download: &'static str,
}

/// A row that fetches a catalog model when pressed.
#[derive(Component)]
struct FetchRow(usize);

/// The one in-flight fetch, if any. One at a time on purpose: two parallel
/// multi-gigabyte downloads help nobody.
#[derive(Resource, Default)]
pub struct FetchState {
    // Mutex only to make the channel Sync for the resource bound; a single
    // system ever touches it.
    rx: Option<std::sync::Mutex<std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>>>,
    label: String,
}

/// Measured anchors for the speed estimate on this stack (honeycrisp on
/// Apple Silicon, the backend soma actually runs): (weight GB, tok/s).
/// Between anchors the estimate interpolates in log-log space, because
/// throughput is nowhere near a single constant over weight — small models
/// pay fixed overheads, large ones fall off the fast path. Order-of-
/// magnitude honesty, not a benchmark.
const SPEED_ANCHORS: &[(f32, f32)] = &[(0.43, 230.0), (2.2, 57.0), (15.7, 1.5)];

fn est_tok_per_s(bytes: u64) -> f32 {
    let gb = (bytes as f32 / 1e9).max(0.05);
    let first = SPEED_ANCHORS[0];
    let last = SPEED_ANCHORS[SPEED_ANCHORS.len() - 1];
    if gb <= first.0 {
        return first.1;
    }
    if gb >= last.0 {
        // Extrapolate below the last anchor at its local slope.
        return last.1 * last.0 / gb;
    }
    for w in SPEED_ANCHORS.windows(2) {
        let (g0, t0) = w[0];
        let (g1, t1) = w[1];
        if gb <= g1 {
            let f = (gb.ln() - g0.ln()) / (g1.ln() - g0.ln());
            return (t0.ln() + f * (t1.ln() - t0.ln())).exp();
        }
    }
    last.1
}

fn speed_hint(bytes: u64) -> String {
    let est = est_tok_per_s(bytes);
    if est < 4.0 {
        format!("~{est:.0} tok/s - slow")
    } else {
        format!("~{est:.0} tok/s")
    }
}

impl Plugin for ModelsWorldPlugin {
    fn build(&self, app: &mut App) {
        // The status is a value, not a discovery — resolve it at build so the
        // page can never race a Startup system and open saying "no mind" on
        // a machine that has one.
        let status = MindStatus {
            model: Some(soma_kernel::default_model_path()),
            last_tok_per_s: None,
        };
        app.insert_resource(status)
            .init_resource::<FetchState>()
            .add_systems(OnEnter(WorldState::Models), build_page)
            .add_systems(OnExit(WorldState::Models), destroy_page)
            .add_systems(Update, poll_fetch)
            .add_systems(
                Update,
                (rebuild_on_change, handle_model_press, handle_fetch_press)
                    .run_if(in_state(WorldState::Models)),
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

fn build_page(mut commands: Commands, status: Res<MindStatus>, fetch: Res<FetchState>) {
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
        (None, _) => "no model chosen".into(),
    };
    commands.spawn((
        Text::new(status_line),
        TextFont { font_size: theme::CAPTION, ..default() },
        TextColor(theme::TEXT_DIM),
        ChildOf(page),
    ));

    let active = status.model.clone();
    let list = models_on_disk(active.as_deref());
    let installed_labels: Vec<String> = list.iter().map(|(p, _)| file_label(p)).collect();
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
                format!("{}  /  {}  /  active", human_size(size), speed_hint(size))
            } else {
                format!("{}  /  {}", human_size(size), speed_hint(size))
            }),
            TextFont { font_size: theme::CAPTION, ..default() },
            TextColor(theme::TEXT_DIM),
            ChildOf(row),
        ));
    }

    // ── the catalog: minds not yet aboard ───────────────────────────────
    let fetchable: Vec<(usize, &FetchEntry)> = CATALOG
        .iter()
        .enumerate()
        .filter(|(_, e)| !installed_labels.contains(&e.label.to_string()))
        .collect();
    if !fetchable.is_empty() {
        commands.spawn((
            Text::new("available"),
            TextFont { font_size: theme::CAPTION, ..default() },
            TextColor(theme::TEXT_DIM),
            Node { margin: UiRect::top(Val::Px(theme::G * 2.0)), ..default() },
            ChildOf(page),
        ));
        for (i, entry) in fetchable {
            let busy = fetch.rx.is_some();
            let row = commands
                .spawn((
                    FetchRow(i),
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G)),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(theme::DARK_BASE),
                    BorderColor::all(theme::BORDER),
                ))
                .insert(ChildOf(page))
                .id();
            let fetching_this = busy && fetch.label == entry.label;
            commands.spawn((
                Text::new(if fetching_this {
                    format!("{}  (fetching...)", entry.label)
                } else {
                    entry.label.to_string()
                }),
                TextFont { font_size: theme::BODY, ..default() },
                TextColor(if fetching_this { theme::ACID_YELLOW } else { theme::TEXT_PRIMARY }),
                ChildOf(row),
            ));
            commands.spawn((
                Text::new(entry.download),
                TextFont { font_size: theme::CAPTION, ..default() },
                TextColor(theme::TEXT_DIM),
                ChildOf(row),
            ));
        }
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
    fetch: Res<FetchState>,
    roots: Query<Entity, With<ModelsRoot>>,
) {
    let moved = (status.is_changed() && !status.is_added())
        || (fetch.is_changed() && !fetch.is_added());
    if !moved {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_page(commands, status.into(), fetch.into());
}

fn handle_model_press(
    mut interactions: Query<(&Interaction, &ModelRow), Changed<Interaction>>,
    mut status: ResMut<MindStatus>,
    mut notice: ResMut<super::Notice>,
    soma: NonSend<soma_kernel::Soma>,
) {
    for (interaction, row) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
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
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            if est_tok_per_s(size) < 4.0 {
                notice.show(format!(
                    "mind: {} - {} on cpu; answers will crawl",
                    file_label(&path),
                    speed_hint(size)
                ));
            } else {
                notice.show(format!(
                    "mind: {} - wakes on the next question",
                    file_label(&path)
                ));
            }
            status.model = Some(path);
            status.last_tok_per_s = None;
        }
    }
}

/// Press a catalog row: fetch the weights and import them, off-thread.
/// The UI stays live; the notice narrates; the page repaints on completion.
#[cfg_attr(not(target_os = "macos"), allow(unused_variables, unused_mut))]
fn handle_fetch_press(
    mut interactions: Query<(&Interaction, &FetchRow), Changed<Interaction>>,
    mut fetch: ResMut<FetchState>,
    mut notice: ResMut<super::Notice>,
) {
    for (interaction, row) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        #[cfg(target_os = "macos")]
        {
            if fetch.rx.is_some() {
                notice.show(format!("already fetching {}", fetch.label));
                continue;
            }
            let entry = &CATALOG[row.0];
            let (tx, rx) = std::sync::mpsc::channel();
            let (label, hf_id) = (entry.label.to_string(), entry.hf_id.to_string());
            notice.show(format!("fetching {} ({})...", entry.label, entry.download));
            fetch.rx = Some(std::sync::Mutex::new(rx));
            fetch.label = label.clone();
            std::thread::Builder::new()
                .name("model-fetch".into())
                .spawn(move || {
                    let result = glia_import::hf::download_model(&hf_id)
                        .and_then(|dl| {
                            let dir = dl
                                .snapshot_dir()
                                .ok_or_else(|| "download produced no directory".to_string())?
                                .to_string_lossy()
                                .to_string();
                            glia_import::pipeline::import_snapshot(&dir, &label)
                        });
                    let _ = tx.send(result);
                })
                .expect("spawn fetch thread");
        }
        #[cfg(not(target_os = "macos"))]
        notice.show("this body carries no mind yet");
    }
}

/// Runs in every world: a fetch started from models finishes wherever you
/// happen to be, and says so.
fn poll_fetch(mut fetch: ResMut<FetchState>, mut notice: ResMut<super::Notice>) {
    let Some(rx) = &fetch.rx else { return };
    let result = rx.lock().expect("fetch channel poisoned").try_recv();
    match result {
        Ok(Ok(path)) => {
            notice.show(format!("{} is aboard - select it in models", file_label(&path)));
            fetch.rx = None;
            fetch.label.clear();
        }
        Ok(Err(e)) => {
            notice.show(format!("fetch failed: {e}"));
            fetch.rx = None;
            fetch.label.clear();
        }
        Err(std::sync::mpsc::TryRecvError::Empty) => {}
        Err(std::sync::mpsc::TryRecvError::Disconnected) => {
            notice.show("fetch thread died");
            fetch.rx = None;
            fetch.label.clear();
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
