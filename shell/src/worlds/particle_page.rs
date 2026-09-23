//! Spacetime: `cyb://particle/<hex>` — identity, rank, axons. No spark.

use bevy::prelude::*;
use mir::bevy::resources::{GpuBuffers, WarpTarget};
use prysm::molecules::particle_card;
use prysm::theme;

use super::content;
use super::graph::BrainIndex;
use crate::now::{Now, NowKind};
use crate::shell::chrome::{CHROME_BOTTOM_H, CHROME_TOP_H, ContentRoot};

pub struct ParticlePagePlugin;

#[derive(Component)]
struct ParticlePage;

#[derive(Component)]
struct AxonRow(usize);

impl Plugin for ParticlePagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (sync_page, walk_axon).run_if(now_meta))
            .add_systems(Update, despawn_idle.run_if(not(now_meta)));
    }
}

fn now_meta(now: Res<Now>) -> bool {
    now.kind == NowKind::Meta
}

fn despawn_idle(mut commands: Commands, pages: Query<Entity, With<ParticlePage>>) {
    for e in &pages {
        commands.entity(e).despawn();
    }
}

fn sync_page(
    mut commands: Commands,
    now: Res<Now>,
    index: Option<Res<BrainIndex>>,
    gpu: Option<Res<GpuBuffers>>,
    pages: Query<Entity, With<ParticlePage>>,
) {
    if !now.is_changed() && !pages.is_empty() {
        return;
    }
    for e in &pages {
        commands.entity(e).despawn();
    }
    let Some(p) = now.hash else { return };
    let hash = *p.as_bytes();
    let idx = now.idx;
    let title = idx
        .and_then(|i| {
            index
                .as_ref()
                .and_then(|ix| ix.labels.get(i).cloned().flatten())
        })
        .or_else(|| content::lookup(&hash))
        .unwrap_or_else(|| p.short_hex());
    let focus = idx
        .and_then(|i| index.as_ref().and_then(|ix| ix.focus.get(i).copied()))
        .unwrap_or(0.0);
    let degree = match (idx, gpu.as_ref()) {
        (Some(i), Some(g)) => g
            .csr
            .as_ref()
            .map(|csr| neighbor_count(csr, i))
            .unwrap_or(0),
        _ => 0,
    };

    let root = commands
        .spawn((
            ParticlePage,
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
                row_gap: Val::Px(theme::G * 1.5),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            ScrollPosition::default(),
            crate::worlds::scroll::PersistScroll("particle"),
            ChildOf(root),
        ))
        .id();
    particle_card::spawn(
        &mut commands,
        page,
        &title,
        &p.short_hex(),
        Some(&format!("focus {focus:.4}   axons {degree}")),
    );

    let Some(i) = idx else { return };
    let Some(gpu) = gpu else { return };
    let Some(csr) = gpu.csr.as_ref() else { return };
    let Some(index) = index else { return };
    // A hub particle can have thousands of axons. Spawning a button for
    // each one hangs the UI thread — the click that opened this page.
    const AXON_CAP: usize = 32;
    let all: Vec<(usize, f32)> = neighbors(csr, i).collect();
    let extra = all.len().saturating_sub(AXON_CAP);
    for (n_idx, weight) in all.into_iter().take(AXON_CAP) {
        let name = index
            .labels
            .get(n_idx)
            .cloned()
            .flatten()
            .unwrap_or_else(|| {
                index
                    .hashes
                    .get(n_idx)
                    .map(|h| file::Particle::from_bytes(*h).short_hex())
                    .unwrap_or_else(|| n_idx.to_string())
            });
        commands
            .spawn((
                AxonRow(n_idx),
                Button,
                Node {
                    width: Val::Percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    padding: UiRect::axes(Val::Px(theme::G * 1.5), Val::Px(theme::G * 0.75)),
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(theme::DARK_BASE),
                BorderColor::all(theme::BORDER),
                ChildOf(page),
            ))
            .with_children(|row| {
                row.spawn((
                    Text::new(name),
                    TextFont {
                        font_size: theme::BODY,
                        ..default()
                    },
                    TextColor(theme::TEXT_PRIMARY),
                ));
                row.spawn((
                    Text::new(format!("{weight:.2}")),
                    TextFont {
                        font_size: theme::CAPTION,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                ));
            });
    }
    if extra > 0 {
        commands.spawn((
            Text::new(format!("{extra} more axons")),
            TextFont {
                font_size: theme::CAPTION,
                ..default()
            },
            TextColor(theme::TEXT_DIM),
            ChildOf(page),
        ));
    }
}

fn walk_axon(
    interactions: Query<(&Interaction, &AxonRow), Changed<Interaction>>,
    index: Option<Res<BrainIndex>>,
    mut now: ResMut<Now>,
    warp: Option<ResMut<WarpTarget>>,
) {
    let Some(index) = index else { return };
    for (i, row) in &interactions {
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(&hash) = index.hashes.get(row.0) else {
            continue;
        };
        now.stand(hash, Some(row.0));
        if let Some(mut warp) = warp {
            warp.particle_idx = Some(row.0 as u32);
        }
        return;
    }
}

fn neighbor_count(csr: &mir::graph::Csr, idx: usize) -> usize {
    neighbors(csr, idx).count()
}

fn neighbors(csr: &mir::graph::Csr, idx: usize) -> impl Iterator<Item = (usize, f32)> + '_ {
    let (a, b) = (
        csr.row_ptr.get(idx).copied().unwrap_or(0) as usize,
        csr.row_ptr.get(idx + 1).copied().unwrap_or(0) as usize,
    );
    (a..b.min(csr.col_idx.len())).map(|e| {
        (
            csr.col_idx[e] as usize,
            csr.values.get(e).copied().unwrap_or(0.0),
        )
    })
}
