//! Spacetime: `cyb://file/<hex>` — particle + spark surface.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
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
    mut images: ResMut<Assets<Image>>,
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
            crate::worlds::scroll::PersistScroll("file"),
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
            const CAP: usize = 8_000;
            let shown = if body.len() > CAP {
                format!("{}…", &body[..CAP])
            } else {
                body
            };
            commands.spawn((
                Text::new(shown),
                TextFont {
                    font_size: theme::BODY,
                    ..default()
                },
                TextColor(theme::TEXT_PRIMARY),
                ChildOf(frame),
            ));
        }
        Some(Surface::Image { kind, bytes }) => match decode_rgba(&bytes) {
            Some((width, height, rgba)) => {
                let image = Image::new(
                    Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    rgba,
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::RENDER_WORLD,
                );
                let handle = images.add(image);
                commands.spawn((
                    ImageNode {
                        image: handle,
                        ..default()
                    },
                    Node {
                        max_width: Val::Percent(100.0),
                        ..default()
                    },
                    ChildOf(frame),
                ));
            }
            None => {
                commands.spawn((
                    Text::new(format!("image spark ({kind:?}) — could not decode")),
                    TextFont {
                        font_size: theme::CAPTION,
                        ..default()
                    },
                    TextColor(theme::TEXT_DIM),
                    ChildOf(frame),
                ));
            }
        },
        None => {}
    }
}

/// Decode raw encoded image bytes (png/jpeg/gif/webp) to an RGBA8 buffer.
/// `None` on a malformed or empty image; the caller falls back to text.
fn decode_rgba(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    Some((width, height, rgba.into_raw()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_png(width: u32, height: u32) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(width, height, image::Rgba([12, 34, 56, 255]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn decodes_a_valid_png_to_matching_dimensions() {
        let bytes = encode_png(3, 2);
        let (w, h, rgba) = decode_rgba(&bytes).expect("valid png decodes");
        assert_eq!((w, h), (3, 2));
        assert_eq!(rgba.len(), (3 * 2 * 4) as usize);
        assert_eq!(&rgba[0..4], &[12, 34, 56, 255]);
    }

    #[test]
    fn rejects_garbage_bytes() {
        assert!(decode_rgba(&[0, 1, 2, 3, 4]).is_none());
    }

    #[test]
    fn rejects_empty_bytes() {
        assert!(decode_rgba(&[]).is_none());
    }
}
