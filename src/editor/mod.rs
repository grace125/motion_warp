use bevy::{prelude::{Plugin, PluginGroup}, app::PluginGroupBuilder};
use bevy_egui::EguiPlugin;
use bevy::prelude::*;

mod ui;
use ui::*;
mod entity_path_helper;
use entity_path_helper::*;
mod camera;
use camera::*;

use crate::builder::MotionWarpClipBuilder;


pub struct EditorPlugins;

impl PluginGroup for EditorPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(EguiPlugin)
            .add(MainPlugin);
        group
    }
}

pub struct MainPlugin;

impl Plugin for MainPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_state::<Mode>()
            .init_resource::<UiHovered>()   
            .insert_resource(
                MotionWarpClipBuilder {
                    start_time: 0.0, 
                    end_time: 0.001,
                    clips: Vec::new(),
                    blend_margin: 0.1,
                    tension: 0.5,
                }
            )
            .add_event::<RebuildWarpClip>()
            .add_startup_system(setup)
            .add_startup_system(debug_setup)
            .add_system(update_egui_hover.in_base_set(CoreSet::Last))
            .add_systems((
                    top_panel,
                    timeline_panel, 
                    keyframe_panel, 
                    settings_panel.run_if(in_state(Mode::Settings)),
                    property_panel.run_if(in_state(Mode::Keyframe))
                ).chain()
            )
            .add_system(build_warp_clip)
            .add_system(add_entity_paths)
            .edit_schedule(OnExit(Mode::Preview), |schedule| {
                schedule.add_system(pause_on_preview_exit);
            })
            .edit_schedule(OnExit(Mode::Keyframe), |schedule| {
                schedule.add_system(|mut ev: EventWriter<RebuildWarpClip>| ev.send(RebuildWarpClip));
            })
            .edit_schedule(OnEnter(Mode::Keyframe), |schedule| {
                schedule.add_system(set_animation_time_on_keyframe_enter);
            })
            .add_system(play_once_loaded)
            .add_system(orbit_zoom_camera.run_if(not(egui_focused)));
    }
}