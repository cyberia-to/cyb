//! vault — the secrets this body carries, sealed under the owner's words.
//!
//! Passwords, private keys, spell phrases, one-time codes, anything the
//! owner would rather the graph never learn. Entries arrive through the
//! commander (`vault add <name> <kind> <secret...>`) on a path that is
//! intercepted BEFORE com's history cast — the raw line is never echoed,
//! never remembered, never linked. The graph does not know the vault has
//! contents; only that the world exists.
//!
//! On the page: tap a row to copy its secret (the clipboard forgets it
//! after 30 seconds), hold `show` to read it, and `otp` entries tick
//! their live six digits the way an authenticator would.

pub mod store;

use std::time::Instant;

use bevy::prelude::*;
use prysm::theme;

use super::WorldState;
use super::cell;
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};
use prysm::molecules::action::ActionButton;
use rune_ast::Noun;
use rune_interp::{Host, InterpError};

pub struct VaultWorldPlugin;

/// The unsealed vault while the world is open, plus clipboard bookkeeping.
#[derive(Resource, Default)]
struct VaultView {
    key: Option<[u8; 32]>,
    entries: Vec<store::Entry>,
    /// Row index currently held open by its `show` chip.
    revealed: Option<usize>,
    /// What we put on the clipboard and when — so it can be taken back.
    copied: Option<(String, Instant)>,
    error: Option<String>,
}

#[derive(Component)]
struct VaultRoot;

/// Tap the row: the secret (or the live code) goes to the clipboard.
#[derive(Component)]
struct CopyRow(usize);

/// Tap to read; tap again to hide.
#[derive(Component)]
struct RevealChip(usize);

struct VaultHost {
    rows: Noun,
}

impl Host for VaultHost {
    fn perform(&mut self, act: u64, args: &Noun, _caps: &Noun) -> Result<Noun, InterpError> {
        if cell::act_is_query(act) && cell::query_name(args) == "table-body" {
            return Ok(self.rows.clone());
        }
        Ok(Noun::Atom(0))
    }
}

impl Plugin for VaultWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VaultView>()
            .add_systems(OnEnter(WorldState::Vault), unseal)
            .add_systems(OnExit(WorldState::Vault), seal_page)
            .add_systems(Update, forget_clipboard)
            .add_systems(
                Update,
                (tick_page, handle_copy, handle_reveal, handle_table_copy)
                    .run_if(in_state(WorldState::Vault)),
            );
    }
}

/// Entering the world derives the key and opens the file; the page is
/// rebuilt from this snapshot.
fn unseal(mut commands: Commands, mut view: ResMut<VaultView>) {
    load_view(&mut view);
    build_page(&mut commands, &view);
}

/// (Re)read the vault into the view. The FIRST entry is always the
/// identity spell — the twelve words behind the pussy address and every
/// testpussy it earned. It is read from `~/cyb/spell`, never copied
/// into vault.enc: one truth, one file, surfaced where secrets live.
fn load_view(view: &mut VaultView) {
    view.key = store::key();
    view.error = None;
    let mut entries = match view.key {
        Some(k) => match store::load(&k) {
            Ok(e) => e,
            Err(e) => {
                view.error = Some(e);
                Vec::new()
            }
        },
        None => {
            view.error = Some("no identity aboard - the vault needs ~/cyb/spell".into());
            Vec::new()
        }
    };
    if let Some(spell) = identity_spell() {
        entries.insert(
            0,
            store::Entry {
                name: "identity".into(),
                kind: "spell".into(),
                value: spell,
                created: 0,
            },
        );
    }
    view.entries = entries;
    view.revealed = None;
}

/// The twelve words, straight from the identity file.
fn identity_spell() -> Option<String> {
    std::fs::read_to_string(super::identity::spell_path())
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
}

/// Leaving drops the plaintext with the page. The copied-string tail stays
/// so the 30-second clipboard clear still happens from other worlds.
fn seal_page(
    mut commands: Commands,
    mut view: ResMut<VaultView>,
    roots: Query<Entity, With<VaultRoot>>,
) {
    view.entries.clear();
    view.key = None;
    view.revealed = None;
    for e in &roots {
        commands.entity(e).despawn();
    }
}

/// Repaint once a second — the otp codes and their countdowns are live.
fn tick_page(
    time: Res<Time>,
    mut timer: Local<f32>,
    mut reload: Local<f32>,
    mut commands: Commands,
    mut view: ResMut<VaultView>,
    roots: Query<Entity, With<VaultRoot>>,
) {
    // Every couple of seconds, re-read the store: a `vault add`/`rm` typed
    // while the page is open shows up without leaving the world.
    *reload += time.delta_secs();
    if *reload >= 2.0 {
        *reload = 0.0;
        let before: Vec<String> = view.entries.iter().map(|e| e.name.clone()).collect();
        let revealed = view.revealed;
        let mut fresh = std::mem::take(&mut *view);
        load_view(&mut fresh);
        let after: Vec<String> = fresh.entries.iter().map(|e| e.name.clone()).collect();
        fresh.revealed = if before == after { revealed } else { None };
        fresh.copied = view.copied.take();
        *view = fresh;
        if before != after {
            for e in &roots {
                commands.entity(e).despawn();
            }
            build_page(&mut commands, &view);
            *timer = 0.0;
            return;
        }
    }
    *timer += time.delta_secs();
    let has_otp = view.entries.iter().any(|e| e.kind == "otp");
    if *timer < 1.0 || (!has_otp && !view.is_changed()) {
        if view.is_changed() && !view.is_added() {
            // A reveal or copy changed the view mid-second: repaint now.
        } else {
            return;
        }
    }
    *timer = 0.0;
    for e in &roots {
        commands.entity(e).despawn();
    }
    build_page(&mut commands, &view);
}

fn build_page(commands: &mut Commands, view: &VaultView) {
    let root = commands
        .spawn((
            VaultRoot,
            crate::worlds::WorldUi(WorldState::Vault),
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
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                row_gap: Val::Px(theme::G),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            ChildOf(root),
        ))
        .id();

    let text = |commands: &mut Commands, parent: Entity, s: String, size: f32, color: Color| {
        commands.spawn((
            Text::new(s),
            TextFont {
                font_size: size,
                ..default()
            },
            TextColor(color),
            ChildOf(parent),
        ));
    };

    text(
        commands,
        page,
        "vault".into(),
        theme::H2,
        theme::TEXT_PRIMARY,
    );

    if let Some(err) = &view.error {
        text(commands, page, err.clone(), theme::BODY, theme::ACID_RED);
        return;
    }

    text(
        commands,
        page,
        "tap a name to copy (clears in 30s). tap show to read, tap again to hide".into(),
        theme::CAPTION,
        theme::TEXT_DIM,
    );

    if view.entries.is_empty() {
        text(
            commands,
            page,
            "empty - seal something:  vault add <name> <kind> <secret>   \
             kinds: password key spell otp custom"
                .into(),
            theme::BODY,
            theme::TEXT_DIM,
        );
        return;
    }

    let unix = store::now();
    let rows = cell::list(
        view.entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let value = if entry.kind == "otp" {
                    match store::totp(&entry.value, unix) {
                        Some((code, left_s)) => format!("{code}  {left_s}s"),
                        None => "bad otp".into(),
                    }
                } else if view.revealed == Some(i) {
                    entry.value.clone()
                } else {
                    mask(&entry.value)
                };
                cell::row(&[&entry.name, &entry.kind, &value, &format!("vault:{i}")])
            })
            .collect(),
    );
    let mut host = VaultHost { rows };
    match cell::load("vault").and_then(|src| cell::eval(&src, &mut host)) {
        Ok(chunks) => cell::dispatch_page(commands, page, &chunks),
        Err(e) => text(commands, page, e, theme::CAPTION, theme::ACID_RED),
    }
}

fn mask(value: &str) -> String {
    "*".repeat(value.chars().count().min(12))
}

/// Tap a row: its secret (or the code of the moment) goes to the clipboard
/// with a 30-second fuse.
fn handle_table_copy(
    interactions: Query<(&Interaction, &ActionButton), Changed<Interaction>>,
    mut view: ResMut<VaultView>,
    mut notice: ResMut<super::Notice>,
) {
    for (i, btn) in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(idx) = btn.target_ref.strip_prefix("vault:") else {
            continue;
        };
        let Ok(n) = idx.parse::<usize>() else {
            continue;
        };
        let Some(entry) = view.entries.get(n) else {
            continue;
        };
        let payload = if entry.kind == "otp" {
            match store::totp(&entry.value, store::now()) {
                Some((code, _)) => code,
                None => continue,
            }
        } else {
            entry.value.clone()
        };
        match crate::shell::clipboard::write_clipboard(&payload) {
            Ok(()) => {
                notice.show(format!("{} copied - clears in 30s", entry.name));
                view.copied = Some((payload, Instant::now()));
                view.revealed = if view.revealed == Some(n) {
                    None
                } else {
                    Some(n)
                };
            }
            Err(e) => notice.show(format!("clipboard: {e}")),
        }
    }
}

fn handle_copy(
    interactions: Query<(&Interaction, &CopyRow), Changed<Interaction>>,
    mut view: ResMut<VaultView>,
    mut notice: ResMut<super::Notice>,
) {
    for (i, row) in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(entry) = view.entries.get(row.0) else {
            continue;
        };
        let payload = if entry.kind == "otp" {
            match store::totp(&entry.value, store::now()) {
                Some((code, _)) => code,
                None => continue,
            }
        } else {
            entry.value.clone()
        };
        match crate::shell::clipboard::write_clipboard(&payload) {
            Ok(()) => {
                notice.show(format!("{} copied - clears in 30s", entry.name));
                view.copied = Some((payload, Instant::now()));
            }
            Err(e) => notice.show(format!("clipboard: {e}")),
        }
    }
}

/// The fuse: 30 seconds after a copy, if the clipboard still holds our
/// secret, blank it. If the owner copied something else since, leave it.
fn forget_clipboard(mut view: ResMut<VaultView>) {
    let due = matches!(&view.copied, Some((_, at)) if at.elapsed().as_secs() >= 30);
    if !due {
        return;
    }
    let (secret, _) = view.copied.take().expect("checked above");
    if let Ok(current) = crate::shell::clipboard::read_clipboard() {
        if current == secret {
            let _ = crate::shell::clipboard::write_clipboard("");
        }
    }
}

/// Tap show to read; tap again to hide. Hold-to-reveal dies on a tap:
/// Pressed and Released fire in the same gesture, so the words vanished
/// before anyone could read them.
fn handle_reveal(
    interactions: Query<(&Interaction, &RevealChip), Changed<Interaction>>,
    mut view: ResMut<VaultView>,
) {
    for (i, chip) in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        view.revealed = if view.revealed == Some(chip.0) {
            None
        } else {
            Some(chip.0)
        };
    }
}

// ── the commander verbs ─────────────────────────────────────────────────
//
// Called from com BEFORE anything is echoed, remembered or cast. The
// return value is what com may say out loud — never the secret.

pub fn handle_command(rest: &str) -> String {
    let rest = rest.trim();
    if let Some(spec) = rest.strip_prefix("add ") {
        let mut it = spec.splitn(3, char::is_whitespace);
        let (Some(name), Some(kind), Some(value)) = (it.next(), it.next(), it.next()) else {
            return "vault add <name> <kind> <secret>   kinds: password key spell otp custom"
                .into();
        };
        if !store::KINDS.contains(&kind) {
            return format!("vault: unknown kind {kind} - use password key spell otp custom");
        }
        if name == "identity" {
            return "vault: identity is the built-in root entry - it lives in ~/cyb/spell".into();
        }
        let Some(key) = store::key() else {
            return "vault: no identity aboard (~/cyb/spell missing)".into();
        };
        let mut entries = match store::load(&key) {
            Ok(e) => e,
            Err(e) => return format!("vault: {e}"),
        };
        let value = value.trim().to_string();
        if entry_pos(&entries, name).is_some() {
            return format!("vault: {name} already sealed - `vault rm {name}` first");
        }
        entries.push(store::Entry {
            name: name.to_string(),
            kind: kind.to_string(),
            value,
            created: store::now(),
        });
        match store::save(&key, &entries) {
            Ok(()) => format!("vault: {name} ({kind}) sealed"),
            Err(e) => format!("vault: {e}"),
        }
    } else if let Some(name) = rest.strip_prefix("rm ") {
        let name = name.trim();
        let Some(key) = store::key() else {
            return "vault: no identity aboard".into();
        };
        let mut entries = match store::load(&key) {
            Ok(e) => e,
            Err(e) => return format!("vault: {e}"),
        };
        match entry_pos(&entries, name) {
            Some(i) => {
                entries.remove(i);
                match store::save(&key, &entries) {
                    Ok(()) => format!("vault: {name} gone"),
                    Err(e) => format!("vault: {e}"),
                }
            }
            None => format!("vault: no entry named {name}"),
        }
    } else {
        "vault add <name> <kind> <secret>  |  vault rm <name>  |  vault".into()
    }
}

fn entry_pos(entries: &[store::Entry], name: &str) -> Option<usize> {
    entries.iter().position(|e| e.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_never_contains_a_secret() {
        let said = handle_command("add onlynamekind");
        assert!(said.contains("vault add"));
        let said = handle_command("");
        assert!(said.contains("vault add"));
    }

    #[test]
    fn unknown_kind_is_refused() {
        let said = handle_command("add x pin 1234");
        assert!(said.contains("unknown kind"), "{said}");
        assert!(
            !said.contains("1234"),
            "secret leaked into the reply: {said}"
        );
    }

    #[test]
    fn masks_are_bounded() {
        assert_eq!(mask("ab"), "**");
        assert_eq!(mask(&"x".repeat(100)).len(), 12);
    }
}
