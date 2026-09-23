//! Spacetime: `cyb://file/<hex>` — particle + spark surface.

use bevy::prelude::*;
use prysm::molecules::file as file_frame;
use prysm::theme;
use spark::Surface;

use super::content;
use super::graph::BrainIndex;
use crate::now::{Now, NowKind, load_file};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};

pub struct FilePagePlugin;

fn despawn_idle(mut commands: Commands, pages: Query<Entity, With<FilePage>>) {
    for e in &pages {
        commands.entity(e).despawn();
    }
}

#[derive(Component)]
struct FilePage;

impl Plugin for FilePagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            sync_page.run_if(|now: Res<Now>| now.kind == NowKind::File),
        )
        .add_systems(
            Update,
            despawn_idle.run_if(|now: Res<Now>| now.kind != NowKind::File),
        );
    }
}

fn sync_page(
    mut commands: Commands,
    now: Res<Now>,
    index: Option<Res<BrainIndex>>,
    pages: Query<Entity, With<FilePage>>,
) {
    if !now.is_changed() && !pages.is_empty() {
        return;
    }
    for e in &pages {
        commands.entity(e).despawn();
    }
    let Some(p) = now.hash else { return };
    let hash = *p.as_bytes();
    let title = now
        .idx
        .and_then(|i| {
            index
                .as_ref()
                .and_then(|ix| ix.labels.get(i).cloned().flatten())
        })
        .or_else(|| content::lookup(&hash))
        .unwrap_or_else(|| p.short_hex());
    let surface = load_file(hash).and_then(|f| spark::open(&f).ok().flatten());

    let root = commands
        .spawn((
            FilePage,
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
            GlobalZIndex(8),
        ))
        .id();
    let page = commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                max_width: Val::Px(theme::MEASURE),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(theme::G * 3.0)),
                row_gap: Val::Px(theme::G * 2.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            ChildOf(root),
        ))
        .id();
    let spark_name = match &surface {
        Some(Surface::Text(_)) => "text",
        Some(Surface::Image { .. }) => "image",
        None => "—",
    };
    let frame = file_frame::spawn(
        &mut commands,
        page,
        &title,
        &p.short_hex(),
        Some(&format!("spark {spark_name}")),
    );
    match surface {
        Some(Surface::Text(body)) => {
            commands.spawn((
                Text::new(body),
                TextFont {
                    font_size: theme::BODY,
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
                ChildOf(frame),
            ));
        }
        Some(Surface::Image { kind, .. }) => {
            commands.spawn((
                Text::new(format!("image spark ({kind:?}) — pixels next")),
                TextFont {
                    font_size: theme::CAPTION,
                    ..default()
                },
                TextColor(theme::TEXT_DIM),
                ChildOf(frame),
            ));
        }
        None => {}
    }
}
