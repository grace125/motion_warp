mod bevy_animation;
mod bevy_gltf;
mod motion_warp;

use bevy::{prelude::{PluginGroup, Plugin, CoreSet, App, AddAsset, IntoSystemConfig}, app::PluginGroupBuilder, transform::TransformSystem};

pub use bevy_animation::*;
pub use bevy_gltf::*;
pub use motion_warp::*;

pub mod quat_splines;
pub mod editor;

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

#[derive(Default)]
pub struct AnimationPlugin {}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AnimationClip>()
            .add_asset::<MotionWarpClip>()
            .register_asset_reflect::<AnimationClip>()
            .register_type::<AnimationPlayer>()
            .add_system(
                animation_player
                    .in_base_set(CoreSet::PostUpdate)
                    .before(TransformSystem::TransformPropagate),
            );
    }
}