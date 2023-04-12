use std::f32::consts::TAU;

use bevy::{window::{Window, PrimaryWindow}, prelude::*, input::mouse::{MouseMotion, MouseWheel}};

const ORBIT_BUTTON: MouseButton = MouseButton::Left;



#[derive(Component)]
pub struct PrimaryCamera {
    pub(crate) focus: Vec3,
    pub(crate) radius: f32,
    pub(crate) upside_down: bool,
}

// TODO: add panning
pub fn orbit_zoom_camera(
    window: Query<&Window, With<PrimaryWindow>>,
    mut motion: EventReader<MouseMotion>,
    mut scroll: EventReader<MouseWheel>,
    input: Res<Input<MouseButton>>,
    mut camera: Query<(&mut Transform, &Projection, &mut PrimaryCamera)>,
) {
    let (mut camera_transform, _camera_projection, mut controller) = camera.single_mut();

    if input.just_pressed(ORBIT_BUTTON) || input.just_released(ORBIT_BUTTON) {
        controller.upside_down = (*camera_transform * Vec3::Y).y < 0.0;
    }

    let scroll_delta: f32 = scroll.iter().map(|ev| ev.y).sum();

    if input.pressed(ORBIT_BUTTON) {
        let mouse_delta: Vec2 = motion.iter().map(|ev| ev.delta).sum();
        if mouse_delta.length_squared() == 0.0 { return; }
        let window = window.single();
        let window_dim = Vec2::new(window.width(), window.height());
        let mut delta = mouse_delta / window_dim * TAU;
        if controller.upside_down { 
            delta.x *= -1.0; 
        }
        let yaw = Quat::from_rotation_y(-delta.x);
        let pitch = Quat::from_rotation_x(-delta.y);
        camera_transform.rotation = yaw * camera_transform.rotation * pitch;
    }
    else if scroll_delta != 0.0 {
        controller.radius -= scroll_delta * controller.radius * 0.25;
        controller.radius = f32::max(controller.radius, 0.01);
    }
    else {
        return;
    }

    let rot_matrix = Mat3::from_quat(camera_transform.rotation);
    camera_transform.translation = controller.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, controller.radius))
}

