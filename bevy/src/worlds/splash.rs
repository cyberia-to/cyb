use bevy::prelude::*;

use super::WorldState;

pub struct SplashWorldPlugin;

/// Marker for splash visual entities only — NOT the persistent camera.
#[derive(Component)]
pub struct SplashMarker;

/// Persistent camera used by all world states.
#[derive(Component)]
pub struct PrimaryCamera;

#[derive(Resource)]
struct SplashTimer(f32);

const SPLASH_DURATION: f32 = 2.0;

impl Plugin for SplashWorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SplashTimer(0.0))
            .add_systems(Startup, spawn_camera)
            .add_systems(OnEnter(WorldState::Splash), show_splash)
            .add_systems(OnExit(WorldState::Splash), hide_splash)
            .add_systems(
                Update,
                update_splash.run_if(in_state(WorldState::Splash)),
            );
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        PrimaryCamera,
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(bevy::color::Color::BLACK),
            order: 0,
            ..default()
        },
    ));
}

fn show_splash(mut commands: Commands) {
    commands
        .spawn((
            SplashMarker,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(30.0),
                ..default()
            },
            BackgroundColor(bevy::color::Color::BLACK),
        ))
        .with_children(|parent| {
            parent.spawn((
                SplashMarker,
                Text::new("cyb"),
                TextFont { font_size: 64.0, ..default() },
                TextColor(bevy::color::Color::srgba(0.12, 0.80, 0.99, 0.9)),
            ));

            parent
                .spawn((
                    SplashMarker,
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(2.0),
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(bevy::color::Color::srgba(1.0, 1.0, 1.0, 0.08)),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        SplashMarker,
                        ProgressBar,
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(bevy::color::Color::srgba(0.12, 0.80, 0.99, 0.6)),
                    ));
                });
        });
}

#[derive(Component)]
struct ProgressBar;

fn update_splash(
    time: Res<Time>,
    mut timer: ResMut<SplashTimer>,
    mut bar_query: Query<&mut Node, With<ProgressBar>>,
    mut next_state: ResMut<NextState<WorldState>>,
) {
    timer.0 += time.delta_secs();
    let t = (timer.0 / SPLASH_DURATION).min(1.0);
    let progress = ease_out(t) * 100.0;
    for mut node in &mut bar_query {
        node.width = Val::Percent(progress);
    }
    if timer.0 >= SPLASH_DURATION {
        next_state.set(WorldState::Spells);
    }
}

fn hide_splash(mut commands: Commands, q: Query<Entity, With<SplashMarker>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}
