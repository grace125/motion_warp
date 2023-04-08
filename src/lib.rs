mod bevy_animation;
mod bevy_gltf;

use bevy::{prelude::PluginGroup, app::PluginGroupBuilder};

pub use bevy_animation::*;
pub use bevy_gltf::*;

pub struct MotionWarpPlugins;

impl PluginGroup for MotionWarpPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(AnimationPlugin {})
            .add(GltfPlugin);
        group
    }
}