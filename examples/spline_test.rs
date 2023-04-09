use bevy::{prelude::*, time::Stopwatch};
use motion_warp::{quat_splines::{DeCasteljauQuatCurve, CardinalQuatCurve}};
use std::f32::consts::PI;

// An example which shows off quaternion splines.

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_systems((restart_rotation, rotation_control).chain())
        .run();
}


fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let material = materials.add(StandardMaterial::default());
    let mesh = meshes.add(shape::Box::new(5.0, 0.1, 0.1).into());
    let transform = Transform::from_xyz(0.0, 6., 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y);

    commands.spawn(Camera3dBundle { transform, ..default() });

    commands.spawn((
        PbrBundle { mesh, material, ..default() },
        RotationControl {
            stopwatch: Stopwatch::new(),
            spline: CardinalQuatCurve::new( 
                0.0,
                [
                    (Quat::IDENTITY, 0.0), 
                    (Quat::from_axis_angle(Vec3::X, PI/2.0), 1.0), 
                    (Quat::from_axis_angle(Vec3::Y, PI/2.0), 4.0),
                    (Quat::from_axis_angle(Vec3::Z, PI/3.0), 6.0),
                    (Quat::from_axis_angle(Vec3::Y, PI/4.0), 7.0),
                ],
            ).to_curve()
        }
    ));
}

#[derive(Component)]
struct RotationControl {
    stopwatch: Stopwatch,
    spline: DeCasteljauQuatCurve
}

fn rotation_control(mut query: Query<(&mut Transform, &mut RotationControl)>, time: Res<Time>) {
    for (mut transform, mut rotation_control) in query.iter_mut() {
        let t = rotation_control.stopwatch.tick(time.delta()).elapsed_secs().min(7.0);
        transform.rotation = rotation_control.spline.position(t);
    }
}

fn restart_rotation(
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut RotationControl>,
) {
    if input.just_pressed(KeyCode::Space) {
        for mut control in query.iter_mut() {
            control.stopwatch.reset();
        }
    }
}