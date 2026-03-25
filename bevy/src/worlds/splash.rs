use bevy::prelude::*;

use super::WorldState;

pub struct SplashWorldPlugin;

/// Marker component for all splash entities. Public so Legacy can clean them up.
#[derive(Component)]
pub struct SplashMarker;

#[derive(Resource)]
struct SplashTimer(f32);

const SPLASH_DURATION: f32 = 2.0;

impl Plugin for SplashWorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SplashTimer(0.0))
            .add_systems(OnEnter(WorldState::Splash), show_splash)
            // NOTE: no OnExit cleanup — Legacy handles splash removal after WebView is ready
            .add_systems(
                Update,
                update_splash.run_if(in_state(WorldState::Splash)),
            );
    }
}

fn show_splash(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(bevy::color::Color::BLACK),
            order: 100,
            ..default()
        },
        SplashMarker,
    ));

    commands
        .spawn((
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
            SplashMarker,
        ))
        .with_children(|parent| {
            // "cyb" logo
            parent.spawn((
                Text::new("cyb"),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
                TextColor(bevy::color::Color::srgba(0.12, 0.80, 0.99, 0.9)),
                SplashMarker,
            ));

            // Progress bar container
            parent
                .spawn((
                    Node {
                        width: Val::Px(240.0),
                        height: Val::Px(2.0),
                        margin: UiRect::top(Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(bevy::color::Color::srgba(1.0, 1.0, 1.0, 0.08)),
                    SplashMarker,
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(bevy::color::Color::srgba(0.12, 0.80, 0.99, 0.6)),
                        SplashMarker,
                        ProgressBar,
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

    // Ease-out progress
    let progress = ease_out(t) * 100.0;

    for mut node in &mut bar_query {
        node.width = Val::Percent(progress);
    }

    if timer.0 >= SPLASH_DURATION {
        next_state.set(WorldState::Legacy);
    }
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}
