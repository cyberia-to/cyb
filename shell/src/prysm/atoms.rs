use bevy::prelude::*;
use super::theme;

#[derive(Clone, Copy, Debug)]
pub enum GlassDepth {
    Subtle,
    Background,
    Midground,
    Foreground,
}

pub fn glass_bg(depth: GlassDepth) -> BackgroundColor {
    let alpha = match depth {
        GlassDepth::Subtle     => theme::SUBTLE,
        GlassDepth::Background => theme::BACKGROUND,
        GlassDepth::Midground  => theme::MIDGROUND,
        GlassDepth::Foreground => theme::FOREGROUND,
    };
    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, alpha))
}

#[derive(Component)]
pub struct Glass;

#[derive(Component)]
pub struct Saber;

pub fn saber_h(commands: &mut ChildSpawnerCommands) {
    commands.spawn((
        Saber,
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(1.0),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.10)),
    ));
}
