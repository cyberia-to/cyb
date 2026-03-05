use bevy::prelude::*;
use super::WorldState;

pub struct InterfaceWorldPlugin;

#[derive(Resource, Default)]
struct InterfaceTick(u64);

#[derive(Component)]
struct InterfaceMarker;

impl Plugin for InterfaceWorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InterfaceTick>()
            .add_systems(OnEnter(WorldState::Interface), show_interface)
            .add_systems(OnExit(WorldState::Interface), hide_interface)
            .add_systems(
                Update,
                rotate_cube.run_if(in_state(WorldState::Interface)),
            );
    }
}

fn show_interface(world: &mut World) {
    let existing: Vec<Entity> = world
        .query_filtered::<Entity, With<InterfaceMarker>>()
        .iter(world)
        .collect();

    if !existing.is_empty() {
        for entity in &existing {
            if let Some(mut vis) = world.get_mut::<Visibility>(*entity) {
                *vis = Visibility::Visible;
            }
            if let Some(mut cam) = world.get_mut::<Camera>(*entity) {
                cam.is_active = true;
            }
        }
        info!("Interface scene shown (persisted)");
        return;
    }

    let mut meshes = world.resource_mut::<Assets<Mesh>>();
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
    let cube_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.8, 0.3),
        ..default()
    });

    world.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(bevy::color::Color::BLACK),
            is_active: true,
            ..default()
        },
        Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        InterfaceMarker,
    ));

    world.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(cube_material),
        Transform::default(),
        InterfaceMarker,
    ));

    world.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, 0.5, 0.0)),
        InterfaceMarker,
    ));
}

fn hide_interface(world: &mut World) {
    let entities: Vec<Entity> = world
        .query_filtered::<Entity, With<InterfaceMarker>>()
        .iter(world)
        .collect();
    for entity in &entities {
        if let Some(mut vis) = world.get_mut::<Visibility>(*entity) {
            *vis = Visibility::Hidden;
        }
        if let Some(mut cam) = world.get_mut::<Camera>(*entity) {
            cam.is_active = false;
        }
    }
    info!("Interface scene hidden (state persisted)");
}

fn rotate_cube(
    time: Res<Time>,
    mut query: Query<&mut Transform, (With<InterfaceMarker>, With<Mesh3d>)>,
    mut tick: ResMut<InterfaceTick>,
) {
    tick.0 += 1;
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs());
    }
}
