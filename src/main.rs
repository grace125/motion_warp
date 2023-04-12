use motion_warp::{MotionWarpPlugins, editor::EditorPlugins};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(MotionWarpPlugins)
        .add_plugins(EditorPlugins)
        .run()
}
