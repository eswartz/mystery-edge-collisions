use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use std::time::Duration;
use rapier3d::prelude::Vector;
use bevy::log::{LogPlugin, tracing};
use bevy::scene::SceneInstanceReady;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
            .set(LogPlugin {
                filter: "info,wgpu_hal=off,avian3d=debug,calloop=off".to_string(),
                level: tracing::Level::INFO,
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../assets".to_string(),
                ..default()
            })
            ,
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
        ))
        .add_systems(Startup, (setup_graphics, spawn_scene))
        .add_systems(Update, fire_projectiles.run_if(repeating_with_delay(Duration::from_secs(2))))
        .run();
}
/// A condition that yields `true` every `duration`.
pub fn repeating_with_delay(duration: Duration) -> impl FnMut(Res<Time>) -> bool + Clone {
    let mut timer = Timer::new(duration, TimerMode::Repeating);
    move |time: Res<Time>| {
        timer.tick(time.delta());
        timer.is_finished()
    }
}

fn setup_graphics(mut commands: Commands) {
    commands.spawn((
        //Camera3d::default(),
        Transform::from_xyz(-30.0, 30.0, 100.0).looking_at(Vec3::new(0.0, 10.0, 0.0), Vec3::Y),
    ));

}

fn spawn_scene(
    mut commands: Commands,
//    model: Res<CollisionSceneSelection>,
    assets: Res<AssetServer>,
) {
  //  let scene_path = model.get_asset_path().to_owned();
    commands
        .spawn((
    //        DespawnOnExit(ProgramState::InGame),
//            WorldMarker,
            Visibility::Inherited,
            Transform::IDENTITY,
            AmbientLight {
                brightness: 2000.0,
                ..default()
            },
        ))
        .with_children(|commands| {
            commands
                .spawn(SceneRoot(assets.load::<Scene>("maps/level_0_reduced.glb#Scene0")))
                .observe(handle_setup_scene)
            ;
        });
}

/// glTF scene is instantiated. Set up colliders.
fn handle_setup_scene(
    event: On<SceneInstanceReady>,
    meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
    child_q: Query<&Children>,
    mesh_q: Query<&Mesh3d>,
    name_q: Query<&Name>,
) {
    // Add colliders.
    for ent in child_q.iter_descendants(event.entity) {
        let Ok(mesh3d) = mesh_q.get(ent) else { continue };
        let Some(mesh) = meshes.get(mesh3d.id()) else { continue };
        if mesh.count_vertices() > 500 { continue }    // text
        let Ok(name) = name_q.get(ent) else { continue };
        log::warn!("adding collider for {name}: {}", mesh3d.id());

        let verts = mesh.attribute(Mesh::ATTRIBUTE_POSITION).expect("need pos").as_float3().expect("3-dim")
            .iter()
            .map(|e| Vect::new(e[0], e[1], e[2]))
            .collect::<Vec<_>>();
        let tris = mesh
            .triangles()
            .expect("no tris").map(|tri| {
                let i0 = verts.iter().position(|v| *v == tri.vertices[0]).expect("did not find") as u32;
                let i1 = verts.iter().position(|v| *v == tri.vertices[1]).expect("did not find") as u32;
                let i2 = verts.iter().position(|v| *v == tri.vertices[2]).expect("did not find") as u32;
                [i0, i1, i2]
            })
            .collect::<Vec<_>>();
        dbg!(verts.len(), tris.len());
        // let collider = Collider::trimesh(verts, tris).expect("failed");
        let collider = Collider::trimesh_with_flags(verts, tris, TriMeshFlags::FIX_INTERNAL_EDGES).expect("failed");
        commands.entity(ent).insert(collider);

        commands.entity(ent).insert(
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::default(),
            }
        );
    }

    commands.run_system_cached(fire_projectiles);
}

fn fire_projectiles(
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
) {

    // let size = Vec3::new(2.0, 0.5, 0.5);
    let size = Vec3::new(1.0, 0.5, 0.5);
    let mesh = meshes.add(Cuboid::new(size.x, size.y, size.z));
    let mat = mats.add(Color::WHITE);

    const X1: f32 = 3.0;
    const Z1: f32 = 10.0;
    const SX: f32 = 3.0;
    const SZ: f32 = 9.5;

    // Projectiles point in selected positions and angles most likely
    // to demonstrate the issue in a visibly obvious way, given
    // the symmetry of movement and the asymmetry of response.
    for (pos, vel) in [
        (Vec3::new(-X1, 1.0, -Z1), Vector::new(-SX, -1.0, SZ)),
        (Vec3::new(X1, 1.0, -Z1), Vector::new(SX, -1.0, SZ)),
        (Vec3::new(-X1, 1.0, Z1), Vector::new(-SX, -1.0, -SZ)),
        (Vec3::new(X1, 1.0, Z1), Vector::new(SX, -1.0, -SZ)),
        (Vec3::new(5.0, 1.0, 0.0), Vector::new(-SX * 2.0, -1.0, 0.0)),
    ] {
        commands.spawn((
            Name::new("Projectile"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Collider::cuboid(size.x / 2., size.y / 2., size.z / 2.),
            RigidBody::Dynamic,
            AdditionalMassProperties::Mass(1000.0),
            Transform::from_translation(pos),
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::default(),
            },
            Velocity {
                linear: Vect::new(vel.x, vel.y, vel.z),
                angular: default(),
            }
        ));
    }

}
