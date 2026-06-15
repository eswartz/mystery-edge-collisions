use std::hash::Hash;
use std::ops::Index;
use std::time::Duration;

use avian3d::math::Vector;
use avian3d::parry::shape::TypedShape;
use avian3d::prelude::*;
use bevy::asset::AssetLoadFailedEvent;
use bevy::color::palettes::css;
use bevy::log::LogPlugin;
use bevy::platform::collections::HashSet;
use bevy::scene::SceneInstanceReady;
use bevy::{color::palettes::css::*, prelude::*};
use bevy_skein::SkeinPlugin;

#[derive(States, Reflect, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[reflect(State, Default)]
#[type_path = "game"]
pub enum ProgramState {
    #[default]
    Initializing,
    Setup,
    InGame,
    Teardown,
}

/// Root of 3D content.
#[derive(Component)]
struct WorldMarker;

/// Root of UI content.
#[derive(Component)]
struct UiMarker;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[allow(unused)]
enum CollisionSceneSelection {
    #[default]
    Scene0,
    Scene0b,
    Scene1,
    Scene2,
    Scene3,
    Scene4,
    Scene5,
    Scene6,
    Scene7,
}

impl Index<usize> for CollisionSceneSelection {
    type Output = Self;

    fn index(&self, index: usize) -> &Self::Output {
        &Self::PLAYLIST[index]
    }
}
impl CollisionSceneSelection {
    pub(crate) const LEN: usize = 7;
    pub(crate) const PLAYLIST: [Self; Self::LEN] = [
        Self::Scene0,
        Self::Scene0b,
        Self::Scene1,
        Self::Scene2,
        Self::Scene3,
        Self::Scene4,
        // Self::Scene5,
        Self::Scene6,
        // Self::Scene7,
    ];
    pub(crate) fn position(&self) -> usize {
        // This is dumb
        Self::PLAYLIST
            .iter()
            .position(|other| self == other)
            .expect("empty list")
    }
    pub(crate) fn next(&self) -> Self {
        let pos = self.position();
        Self::PLAYLIST
            .get((pos + 1) % Self::LEN)
            .cloned()
            .expect("empty list")
    }
    pub(crate) fn prev(&self) -> Self {
        let pos = self.position();
        Self::PLAYLIST
            .get((pos + Self::LEN - 1) % Self::LEN)
            .cloned()
            .expect("empty list")
    }

    pub(crate) fn get_asset_path(&self) -> &str {
        match self {
            Self::Scene0 => "maps/level_0.glb#Scene0",
            Self::Scene0b => "maps/level_0_reduced.glb#Scene0",
            Self::Scene1 => "maps/level_0_broken.glb#Scene0",
            Self::Scene2 => "maps/level_0_edit_0.glb#Scene0",
            Self::Scene3 => "maps/level_0_edit_1.glb#Scene0",
            Self::Scene4 => "maps/level_0_edit_2.glb#Scene0",
            Self::Scene5 => "maps/level_0_edit_3.glb#Scene0",
            Self::Scene6 => "maps/level_0_edit_fixed.glb#Scene0",
            Self::Scene7 => "maps/level_0_edit_fixed_2.glb#Scene0",
        }
    }
}

/// Add to prompt moving the projectiles to their base location.
#[derive(Resource, Debug, Default)]
struct ResetProjectiles;

/// Add to prompt firing the projectiles with the given power.
#[derive(Resource, Debug, Default)]
struct FireProjectiles(f32);

// set from skein
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct ContentMarker;

#[derive(Component)]
struct Projectile {
    start: Vec3,
    vel: Vector,
}

#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
enum IncludeColliders {
    #[default]
    OnlyMeshes,
    All,
    NonMeshes,
}

impl IncludeColliders {
    fn next(self) -> Self {
        match self {
            IncludeColliders::OnlyMeshes => IncludeColliders::All,
            IncludeColliders::All => IncludeColliders::NonMeshes,
            IncludeColliders::NonMeshes => IncludeColliders::OnlyMeshes,
        }
    }
}

#[derive(Clone, Reflect, GizmoConfigGroup)]
#[reflect(Clone, Default)]
struct OurColliderGizmos {
    pub draw_face_normal: bool,
    pub draw_edge_normal: bool,
    pub draw_vert_normal: bool,
    pub face_normal_color: Option<Color>,
    pub edge_normal_color: Option<Color>,
    pub vert_normal_color: Option<Color>,
    pub scale: f32,
}

impl Default for OurColliderGizmos {
    fn default() -> Self {
        Self {
            draw_face_normal: true,
            draw_edge_normal: true,
            draw_vert_normal: true,
            face_normal_color: Some(css::BISQUE.with_alpha(0.25).into()),
            edge_normal_color: Some(css::CADET_BLUE.with_alpha(0.25).into()),
            vert_normal_color: Some(css::LAVENDER.with_alpha(0.5).into()),
            scale: 1.0,
        }
    }
}

fn main() -> AppExit {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(LogPlugin {
            filter: "info,wgpu_hal=off,avian3d=debug,calloop=off".to_string(),
            level: tracing::Level::INFO,
            ..default()
        }),
        SkeinPlugin::default(),
        PhysicsPlugins::default(),
    ))
    .add_plugins(avian3d::debug_render::PhysicsDebugPlugin::default())
    .insert_gizmo_config(
        PhysicsGizmos {
            collider_color: Some(ORANGE.with_alpha(0.25).into()),
            aabb_color: Some(Color::WHITE.with_alpha(0.25).into()),
            sleeping_color_multiplier: Some([0.1, 0.1, 0.1, 1.0]),
            contact_normal_color: Some(RED.with_alpha(0.5).into()),
            contact_normal_scale: ContactGizmoScale::Constant(2.0),
            ..default()
        },
        GizmoConfig {
            enabled: true,
            depth_bias: -0.1,   // ensure edges are drawn over meshes using them
            ..default()
        },
    )
    .insert_gizmo_config(
        OurColliderGizmos {
            ..default()
        },
        GizmoConfig {
            enabled: true,
            depth_bias: -0.5,   // ensure edges are drawn over meshes using them
            ..default()
        },
    )
    ;

    app.insert_state(ProgramState::default())
        .init_resource::<CollisionSceneSelection>()
        .init_resource::<IncludeColliders>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(ProgramState::Setup), (spawn_scene, make_ui))
        .add_systems(Update, set_in_game_soon.run_if(in_state(ProgramState::Setup)))
        .add_systems(Update, handle_load_failed)
        .add_systems(OnEnter(ProgramState::InGame), (queue_fire_projectiles, report_collider_mesh_info))
        .add_systems(
            OnEnter(ProgramState::Teardown),
            (remove_world, remove_ui, restart).chain(),
        )
        .add_systems(
            Update,
            (
                handle_keys.run_if(in_state(ProgramState::InGame)),
                reset_projectiles.run_if(resource_exists::<ResetProjectiles>),
                fire_projectiles.run_if(resource_exists::<FireProjectiles>),
            ),
        )
        .add_systems(
            PostUpdate,
            draw_collider_mesh_gizmos,
        )
    ;

    app.run()
}

fn handle_load_failed(reader: MessageReader<AssetLoadFailedEvent<Scene>>, mut commands: Commands) {
    if reader.is_empty() {
        return;
    }
    commands.set_state(ProgramState::InGame);
}

fn setup(mut commands: Commands, world_q: Query<Entity, With<WorldMarker>>) {
    for world in world_q.iter() {
        commands.entity(world).despawn();
    }

    commands.set_state(ProgramState::Setup);
}

fn spawn_scene(
    mut commands: Commands,
    model: Res<CollisionSceneSelection>,
    assets: Res<AssetServer>,
) {
    let scene_path = model.get_asset_path().to_owned();
    commands
        .spawn((
            DespawnOnExit(ProgramState::InGame),
            WorldMarker,
            Visibility::Inherited,
            Transform::IDENTITY,
            AmbientLight {
                brightness: 2000.0,
                ..default()
            },
        ))
        .with_children(|commands| {
            commands
                .spawn(SceneRoot(assets.load::<Scene>(scene_path)))
                .observe(handle_setup_scene)
            ;
        });
}

/// glTF scene is instantiated. Spawn our content.
fn handle_setup_scene(
    _event: On<SceneInstanceReady>,
    world_q: Single<Entity, With<WorldMarker>>,
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
        (Vec3::new(5.0, 1.0, 0.0), Vector::new(-SX, -1.0, 0.0)),
    ] {
        commands.spawn((
            ChildOf(*world_q),
            Name::new("Projectile"),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Collider::cuboid(size.x, size.y, size.z),
            RigidBody::Dynamic,
            Mass(1000.0),
            Transform::from_translation(pos),
            Friction::ZERO,
            Projectile { start: pos, vel },
        ));
    }

}

fn set_in_game_soon(mut commands: Commands,
    time: Res<Time>,
    // program_state: Res<State<ProgramState>>,
    mut timer: Local<Timer>,
) {
    // if **program_state != ProgramState::InGame {
    //     return
    // }

    if timer.duration().is_zero() {
        *timer = Timer::new(Duration::from_secs(1), TimerMode::Once);
    }
    if timer.tick(time.delta()).is_finished() {
        commands.set_state(ProgramState::InGame);
    }

}

fn make_ui(mut commands: Commands<'_, '_>, model: Res<CollisionSceneSelection>) {
    let scene_path = dbg!(model.get_asset_path().to_owned());
    commands.spawn((
        UiMarker,
        Camera2d::default(),
        Camera {
            // on top of 3d
            order: 1,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        Text::new(format!(
            r#"
Scene: {scene_path}
Right/Left: toggle scenes
Enter: fire projectiles (hold time => velocity)
'['/']': rotate geometry

0: toggle physics gizmos
1,2,3: toggle collider vert, edge, face normals
F: flip normals on collider mesh faces (FIX_INTERNAL_EDGES)
Z: recreate collider (default flags -- 0!)
Q: recreate collider (all bits except FIX_INTERNAL_EDGES)
Backspace: recreate collider (FIX_INTERNAL_EDGES)
            "#,
        )),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::WHITE.with_alpha(0.5)),
    ));
}

fn remove_world(
    mut commands: Commands,
    world_q: Query<Entity, With<WorldMarker>>,
) {
    world_q
        .iter()
        .for_each(|ent| commands.entity(ent).try_despawn());
}

fn remove_ui(mut commands: Commands, ui_q: Query<Entity, With<UiMarker>>) {
    ui_q.iter()
        .for_each(|ent| commands.entity(ent).try_despawn());
}

fn restart(mut commands: Commands) {
    commands.set_state(ProgramState::Setup);
}

fn handle_keys(
    keyboard_input: ResMut<ButtonInput<KeyCode>>,
    mut content_q: Query<&mut Transform, With<ContentMarker>>,
    mut gizmos: ResMut<GizmoConfigStore>,
    mut selection: ResMut<CollisionSceneSelection>,
    mut commands: Commands,
    mut include_colliders: ResMut<IncludeColliders>,
    time: Res<Time>,
    mut fire_time: Local<Timer>,
    mut switch_time: Local<Timer>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter) {
        // Fire every N seconds...
        commands.insert_resource(ResetProjectiles);
        let timer = Timer::from_seconds(3.0, TimerMode::Repeating);
        *fire_time = timer;
    }
    // and monitor ongoing press to adjust firing power.
    if keyboard_input.pressed(KeyCode::Enter) {
        if fire_time.tick(time.delta()).just_finished() {
            submit_fire_projectiles(commands.reborrow(), fire_time.elapsed());
        }
    }
    if keyboard_input.just_released(KeyCode::Enter) {
        submit_fire_projectiles(commands.reborrow(), fire_time.elapsed());
    }

    // Turn the world and collider on the Y axis.
    if keyboard_input.just_pressed(KeyCode::BracketLeft) {
        log::info!("Spinning world & collider (no expected change)");
        content_q.iter_mut().for_each(|mut xfrm| {
            xfrm.rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2) * xfrm.rotation
        });
    }
    if keyboard_input.just_pressed(KeyCode::BracketRight) {
        log::info!("Spinning world & collider (no expected change)");
        content_q.iter_mut().for_each(|mut xfrm| {
            xfrm.rotation = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2) * xfrm.rotation
        });
    }

    if keyboard_input.just_pressed(KeyCode::Digit0) {
        log::info!("Toggling physics gizmos");
        gizmos.config_mut::<PhysicsGizmos>().0.enabled ^= true;
    }

    if keyboard_input.just_pressed(KeyCode::Digit1) {
        log::info!("Toggling collider vertex gizmos");
        gizmos.config_mut::<OurColliderGizmos>().1.draw_vert_normal ^= true;
    }
    if keyboard_input.just_pressed(KeyCode::Digit2) {
        log::info!("Toggling collider edge gizmos");
        gizmos.config_mut::<OurColliderGizmos>().1.draw_edge_normal ^= true;
    }
    if keyboard_input.just_pressed(KeyCode::Digit3) {
        log::info!("Toggling collider face gizmos");
        gizmos.config_mut::<OurColliderGizmos>().1.draw_face_normal ^= true;
    }

    // Toggle kind of colliders affected.
    if keyboard_input.just_pressed(KeyCode::KeyA) {
        *include_colliders = include_colliders.next();
        log::info!("Colliders affected: {:?}", *include_colliders);
    }

    // Report collider info.
    if keyboard_input.just_released(KeyCode::KeyI) {
        commands.run_system_cached(report_collider_mesh_info);
    }

    // Recreate mesh with recommended flags.
    if keyboard_input.just_released(KeyCode::Backspace) {
        log::info!("Recreating collider mesh with FIX_INTERNAL_EDGES");
        commands.run_system_cached((|| { (TrimeshFlags::FIX_INTERNAL_EDGES, false)
        }).pipe(recreate_collider_trimesh_faces_with_flags));
    }
    // Flip normals.
    if keyboard_input.just_released(KeyCode::KeyF) {
        log::info!("Recreating collider from flipped face order and using FIX_INTERNAL_EDGES");
        commands.run_system_cached((|| { (TrimeshFlags::FIX_INTERNAL_EDGES, true)
        }).pipe(recreate_collider_trimesh_faces_with_flags));
    }
    // Recreate with all other bits (since the default is zero).
    if keyboard_input.just_released(KeyCode::KeyQ) {
        log::info!("Recreating collider using all bits but not FIX_INTERNAL_EDGES");
        commands.run_system_cached((|| {
            (TrimeshFlags::HALF_EDGE_TOPOLOGY | TrimeshFlags::CONNECTED_COMPONENTS
            | TrimeshFlags::DELETE_BAD_TOPOLOGY_TRIANGLES
            | TrimeshFlags::ORIENTED | TrimeshFlags::MERGE_DUPLICATE_VERTICES
            | TrimeshFlags::DELETE_DEGENERATE_TRIANGLES | TrimeshFlags::DELETE_DUPLICATE_TRIANGLES
            , false)
        }).pipe(recreate_collider_trimesh_faces_with_flags));
    }
    // Recreate mesh with zero flags.
    if keyboard_input.just_released(KeyCode::KeyZ) {
        log::info!("Recreating colliders from their meshes (TrimeshFlags::empty())");
        commands.run_system_cached((|| { (TrimeshFlags::empty(), false)
        }).pipe(recreate_collider_trimesh_faces_with_flags));
    }
    if keyboard_input.just_released(KeyCode::KeyR) {
        log::info!("Recreating colliders from their meshes, faces flipped (TrimeshFlags::empty())");
        commands.run_system_cached((|| { (TrimeshFlags::empty(), true)
        }).pipe(recreate_collider_trimesh_faces_with_flags));
    }

    /// These keys define the bidirectional navigation, from "move back" at position 0
    /// and any others "move forward".
    const NAV_KEYS: [KeyCode; 2] = [KeyCode::ArrowLeft, KeyCode::ArrowRight];

    if keyboard_input.any_just_pressed(NAV_KEYS) {
        let timer = Timer::from_seconds(1.0 / 30.0, TimerMode::Once);
        *switch_time = timer;
    } else if keyboard_input.any_pressed(NAV_KEYS) {
        if switch_time.tick(time.delta()).just_finished() {
            let is_prev = keyboard_input.pressed(NAV_KEYS[0]);
            *selection = if is_prev {
                selection.prev()
            } else {
                selection.next()
            };
            log::info!("Switching scene to {:?}...", *selection);
            commands.set_state(ProgramState::Teardown);
        }
    } else if keyboard_input.any_just_released(NAV_KEYS) {
        switch_time.reset();
    }
}

fn reset_projectiles(
    mut projectiles_q: Query<(&Projectile, &mut Transform, Forces)>,
    mut commands: Commands,
) {
    for (bumper, mut xfrm, mut forces) in projectiles_q.iter_mut() {
        xfrm.translation = bumper.start;
        xfrm.rotation = Quat::IDENTITY;
        *forces.angular_velocity_mut() = default();
        *forces.linear_velocity_mut() = default();
    }
    commands.remove_resource::<ResetProjectiles>();
}

fn queue_fire_projectiles(commands: Commands) {
    submit_fire_projectiles(commands, Duration::from_secs_f32(0.5));
}

fn submit_fire_projectiles(mut commands: Commands, duration: Duration) {
    let power = duration.as_secs_f32().max(1.0 / 15.0) * 2.0;
    commands.insert_resource(FireProjectiles(power));
}

fn fire_projectiles(
    mut projectiles_q: Query<(&Projectile, Forces)>,
    mut commands: Commands,
    fire_projectiles: Res<FireProjectiles>,
) {
    let power = fire_projectiles.0;
    for (bumper, mut forces) in projectiles_q.iter_mut() {
        *forces.angular_velocity_mut() = default();
        *forces.linear_velocity_mut() = bumper.vel * power;
    }
    commands.remove_resource::<FireProjectiles>();
}

fn recreate_collider_trimesh_faces_with_flags(In((flags, flip)): In<(TrimeshFlags, bool)>, include_colliders: Res<IncludeColliders>, mut collider_q: Query<(&Name, &mut Collider)>, mut commands: Commands) {
    for (name, mut collider) in collider_q.iter_mut() {
        let is_mesh = matches!(collider.shape().as_typed_shape(), TypedShape::TriMesh(_));
        let include = match *include_colliders {
            IncludeColliders::OnlyMeshes => is_mesh,
            IncludeColliders::All => true,
            IncludeColliders::NonMeshes => !is_mesh,
        };
        if !include {
            continue
        }

        log::info!("... adjusting {name}");
        let mut trimesh = collider.trimesh_builder().build().expect("no idempotence?");
        if flip {
            for tri in &mut trimesh.indices {
                tri.reverse();
            }
        }
        *collider = Collider::trimesh_with_config(trimesh.vertices, trimesh.indices, flags);

        commands.run_system_cached(report_collider_mesh_info);
    }
}

fn report_collider_mesh_info(collider_q: Query<(Entity, Option<&Name>, &Collider)>) {
    if collider_q.is_empty() {
        log::warn!("No colliders found...");
        return
    }

    for (ent, name_opt, collider) in collider_q.iter() {
        if let TypedShape::TriMesh(mesh) = collider.shape().as_typed_shape() {
            log::info!("{ent}: {:>20}: pseudo_normals_if_oriented={}",
                if let Some(name) = name_opt { name.to_string() } else { String::new() },
                mesh.pseudo_normals_if_oriented().is_some(),
            );

            if log::max_level() >= log::LevelFilter::Debug {
                if let Some(pn) = mesh.pseudo_normals_if_oriented() {
                    #[derive(Debug, Clone)]
                    struct NormalKey(Vec3);

                    impl PartialEq for NormalKey {
                        fn eq(&self, other: &Self) -> bool {
                            self.0.distance(other.0) < 0.00001
                        }
                    }
                    impl Eq for NormalKey {}
                    impl Hash for NormalKey {
                        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                            let val = (self.0.x + self.0.y + self.0.z) * 1e3;
                            (val as u64).hash(state);
                        }
                    }

                    let mut seen: HashSet<NormalKey> = default();
                    for pn in &pn.edges_pseudo_normal {
                        seen.insert(NormalKey(pn[0]));
                        seen.insert(NormalKey(pn[1]));
                        seen.insert(NormalKey(pn[2]));
                    }
                    for val in seen {
                        log::debug!("  unique edge normal: {:.03?}", val.0);
                    }
                }
            }
        }
    }
}


fn draw_collider_mesh_gizmos(
    query: Query<(
        &Collider,
        &GlobalTransform,
    )>,
    mut gizmos: Gizmos<OurColliderGizmos>,
) {
    if !gizmos.config.enabled { return }

    for (collider, gxfrm) in &query {
        let Some(mesh) = collider.shape().as_trimesh() else { continue };

        if gizmos.config_ext.draw_face_normal
        && let Some(face_normal_color) = &gizmos.config_ext.face_normal_color {
            let scale = gizmos.config_ext.scale;
            for tri in mesh.triangles() {
                let center = tri.center();
                let norm = tri.robust_normal();
                gizmos.line(gxfrm.transform_point(center),
                    gxfrm.transform_point(center + norm * scale),
                    *face_normal_color);
            }
        }
        if gizmos.config_ext.draw_vert_normal
        && let Some(vert_normal_color) = gizmos.config_ext.vert_normal_color.clone()
        && let Some(pn) = mesh.pseudo_normals() {
            let scale = gizmos.config_ext.scale * 0.25;

            for (index, norm) in pn.vertices_pseudo_normal.iter().enumerate() {
                let Some(mid) = mesh.vertices().get(index) else { log::error!("wut {index}"); continue };
                gizmos.line(gxfrm.transform_point(*mid), gxfrm.transform_point(mid + norm * scale), vert_normal_color);
            }
        }
        if gizmos.config_ext.draw_edge_normal
        && let Some(edge_normal_color) = gizmos.config_ext.edge_normal_color.clone()
        && let Some(pn) = mesh.pseudo_normals() {
            let scale = gizmos.config_ext.scale * 0.5;

            for (index, tri_edge_normals) in pn.edges_pseudo_normal.iter().enumerate() {
                if index >= mesh.num_triangles() { continue };

                let tri = mesh.triangle(index as u32);
                let mid = tri.a.midpoint(tri.b);
                let norm = tri_edge_normals[0];
                gizmos.line(gxfrm.transform_point(mid), gxfrm.transform_point(mid + norm * scale), edge_normal_color);
                let mid = tri.b.midpoint(tri.c);
                let norm = tri_edge_normals[1];
                gizmos.line(gxfrm.transform_point(mid), gxfrm.transform_point(mid + norm * scale), edge_normal_color);
                let mid = tri.c.midpoint(tri.a);
                let norm = tri_edge_normals[2];
                gizmos.line(gxfrm.transform_point(mid), gxfrm.transform_point(mid + norm * scale), edge_normal_color);
            }
        }
    }
}
