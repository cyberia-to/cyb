use bevy::prelude::*;
use super::WorldState;

pub struct GameWorldPlugin;

#[derive(Resource, Default)]
struct GameTick(u64);

#[derive(Component)]
struct GameSceneMarker;

impl Plugin for GameWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameTick>()
            .add_systems(OnEnter(WorldState::Game), setup_game_scene)
            .add_systems(OnExit(WorldState::Game), cleanup_game_scene)
            .add_systems(
                Update,
                rotate_cube.run_if(in_state(WorldState::Game)),
            );
    }
}

fn setup_game_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut tick: ResMut<GameTick>,
) {
    tick.0 = 0;

    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(bevy::color::Color::BLACK),
            ..default()
        },
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        GameSceneMarker,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.8, 0.3),
            ..default()
        })),
        Transform::default(),
        GameSceneMarker,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
        GameSceneMarker,
    ));
}

fn cleanup_game_scene(world: &mut World) {
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, With<GameSceneMarker>>()
        .iter(world)
        .collect();
    for entity in entities {
        world.despawn(entity);
    }
}

fn rotate_cube(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<GameSceneMarker>, With<Mesh3d>)>,
    mut tick: ResMut<GameTick>,
) {
    tick.0 += 1;
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs());
    }
}
