use bevy::prelude::*;

use super::WorldState;
use super::legacy::LegacyContentLoaded;

pub struct SplashWorldPlugin;

#[derive(Component)]
struct SplashMarker;

#[derive(Resource)]
struct SplashState {
    elapsed: f32,
    content_ready_at: Option<f32>,
}

impl Plugin for SplashWorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(SplashState {
            elapsed: 0.0,
            content_ready_at: None,
        })
        .add_systems(OnEnter(WorldState::Splash), show_splash)
        .add_systems(OnExit(WorldState::Splash), hide_splash)
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
    mut state: ResMut<SplashState>,
    mut bar_query: Query<&mut Node, With<ProgressBar>>,
    content_loaded: Option<Res<LegacyContentLoaded>>,
    mut next_state: ResMut<NextState<WorldState>>,
) {
    state.elapsed += time.delta_secs();

    // Track when React app actually finished mounting
    if content_loaded.is_some() && state.content_ready_at.is_none() {
        state.content_ready_at = Some(state.elapsed);
        info!("Content ready at {:.1}s", state.elapsed);
    }

    // Progress:
    // Phase 1: slow logarithmic climb to ~80% (simulates real loading feel)
    // Phase 2: when content ready → quick fill to 100%
    // Phase 3: brief hold then transition
    let progress = match state.content_ready_at {
        Some(ready_at) => {
            let since_ready = state.elapsed - ready_at;
            let base = base_progress(ready_at);
            let remaining = 100.0 - base;
            let fill = (since_ready / 0.5).min(1.0);
            let p = base + remaining * ease_out(fill);

            if since_ready > 0.8 {
                next_state.set(WorldState::Legacy);
            }
            p
        }
        None => base_progress(state.elapsed),
    };

    for mut node in &mut bar_query {
        node.width = Val::Percent(progress);
    }
}

/// Logarithmic-feel progress: fast start then slows toward 80%
fn base_progress(elapsed: f32) -> f32 {
    // ln(1 + t) / ln(1 + max) * 80
    let max_time = 15.0; // assume max ~15s loading
    let t = elapsed.min(max_time);
    let progress = (1.0 + t).ln() / (1.0 + max_time).ln();
    progress * 80.0
}

fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t) * (1.0 - t)
}

fn hide_splash(mut commands: Commands, query: Query<Entity, With<SplashMarker>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
    info!("Splash dismissed");
}
