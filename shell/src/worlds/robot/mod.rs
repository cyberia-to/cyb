//! Robot world — the robot's own pages, live-loaded as prysm cells.
//!
//! A *cell* is one app screen authored in `rune`, decoded by `prysm`, rendered
//! as native Bevy entities. The binary is a frozen shell; the app is the set of
//! cells. Update the app = edit/publish a cell (no rebuild). See `root/cells.md`.
//!
//! Pipeline (the same seam the terminal uses for `rune <expr>`, but one-shot and
//! rendered into a centered page instead of a scrollback):
//!
//! ```text
//!   cell source ──parse/lower/eval──> result chunk-noun ──noun_to_chunks──> dispatch ──> Bevy UI
//! ```

use bevy::prelude::*;
use prysm::{dispatch, theme};
use tade::Chunk;

use super::WorldState;
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H};

const G: f32 = theme::G;

pub struct RobotWorldPlugin;

/// Root of the rendered page. Despawned (recursively) on world exit.
#[derive(Component)]
struct CellMarker;

impl Plugin for RobotWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(WorldState::Robot), setup_cell);
    }
}

// ── rune host ───────────────────────────────────────────────────────────────

/// No-op host: a cell's page is its *result noun*, not an `emit` side effect.
/// Acts (`emit`/`query`/…) are wired in later phases (P5). The interpreter still
/// requires a host, so this satisfies it and ignores every act.
struct CellHost;

impl rune_interp::Host for CellHost {
    fn perform(
        &mut self,
        _act: u64,
        _args: &rune_ast::Noun,
        _caps: &rune_ast::Noun,
    ) -> Result<rune_ast::Noun, rune_interp::InterpError> {
        Ok(rune_ast::Noun::Atom(0))
    }
}

// ── cell loading + rendering ──────────────────────────────────────────────────

/// Cells baked into the binary at build time — the source of last resort.
///
/// The dev path below reads the repo file so an edit shows on the next enter;
/// an installed app (macOS bundle, Android APK) has no repo, and before this
/// fallback Cmd+3 on any machine but the dev Mac rendered a load error. The
/// radio-backed `name → hash` resolver (cells.md P4) replaces this table.
const BUILTIN_CELLS: &[(&str, &str)] =
    &[("landing", include_str!("../../../../cells/landing.rune"))];

/// Resolve `cell://<name>` to its rune source.
///
/// P1: a local file under `cyb/cells/<name>.rune`, falling back to the copy
/// baked in at build time. Newlines are neutralised to spaces — the classic
/// rune register parses a call on one logical line, so a multi-line, readable
/// cell collapses to a parseable expression. (String literals hold no raw
/// newlines, so this never alters content.) The `name→file` and `name→hash`
/// resolvers (P2/P4) slot in behind this same seam.
fn load_cell(name: &str) -> Result<String, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../cells")
        .join(format!("{name}.rune"));
    std::fs::read_to_string(&path)
        .or_else(|e| {
            BUILTIN_CELLS
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, src)| (*src).to_string())
                .ok_or_else(|| format!("load_cell({name}): {e}"))
        })
        .map(|s| s.replace(['\n', '\r'], " "))
}

/// Parse → lower → interpret a cell source; decode the result to renderable chunks.
fn render_cell(src: &str) -> Result<Vec<Chunk>, String> {
    let ast = rune_parse::parse(src).map_err(|e| format!("parse: {}", e.message))?;
    let formula = rune_lower::lower(ast).map_err(|e| format!("lower: {}", e.message))?;
    let subject = rune_subject::Subject::minimal().to_noun();
    let mut host = CellHost;
    let result = rune_interp::eval_with_host(&subject, &formula, &mut host)
        .map_err(|e| format!("eval: {}", e.message))?;
    Ok(rune_prysm::noun_to_chunks(&result))
}

// ── world lifecycle ───────────────────────────────────────────────────────────

/// The being's name, chosen by the owner. Absent = still "Cyb".
pub fn name_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join("cyb").join("name")
}

pub fn being_name() -> String {
    std::fs::read_to_string(name_file())
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Cyb".into())
}

fn setup_cell(
    mut commands: Commands,
    mut worlds: Query<(&crate::worlds::WorldUi, &mut Visibility)>,
) {
    if crate::worlds::reveal_world(WorldState::Robot, &mut worlds) {
        return;
    }
    // The whole page is six lines, centered on both axes. Nothing else:
    // the being introduces itself, and the commander below hints at the
    // one thing worth doing here — giving it a name.
    let root = commands
        .spawn((
            CellMarker,
            crate::worlds::WorldUi(WorldState::Robot),
            crate::shell::chrome::ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(CHROME_TOP_H),
                bottom: Val::Px(CHROME_BOTTOM_H),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(G * 1.5),
                ..default()
            },
            BackgroundColor(theme::DARK_BASE),
        ))
        .id();

    let line = |commands: &mut Commands, text: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(text),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            ChildOf(root),
        ));
    };

    line(
        &mut commands,
        format!("I am {}", being_name()),
        theme::H1,
        theme::TEXT_PRIMARY,
    );
    line(
        &mut commands,
        "A digital being that works for you".into(),
        theme::H3,
        theme::TEXT_PRIMARY,
    );
    line(
        &mut commands,
        "Thinks how you think".into(),
        theme::H3,
        theme::TEXT_PRIMARY,
    );
    line(
        &mut commands,
        "Earns while you sleep".into(),
        theme::H3,
        theme::TEXT_PRIMARY,
    );
    line(
        &mut commands,
        "Remembers what you forget".into(),
        theme::H3,
        theme::TEXT_PRIMARY,
    );
    line(
        &mut commands,
        "Yours forever".into(),
        theme::H3,
        theme::ACID_GREEN,
    );
}
