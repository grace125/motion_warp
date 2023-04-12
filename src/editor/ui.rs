use std::f32::consts::{TAU, PI};

use bevy::{prelude::*, pbr::CascadeShadowConfigBuilder};
use bevy_egui::{EguiContexts, egui};
use crate::{*, builder::*};

use super::{entity_path_helper::TrackedEntityPath, camera::PrimaryCamera};

const TOP_PANEL_ID: i32 = 0;
const PROPERTY_PANEL_ID: i32 = 1;
const KEYFRAME_PANEL_ID: i32 = 2;
const TIMELINE_PANEL_ID: i32 = 3;
const SETTINGS_PANEL_ID: i32 = 4;

#[derive(Resource, Default)]
pub struct UiHovered(bool);

#[derive(Resource)]
pub struct CurrentAnimation(Handle<AnimationClip>);

#[derive(Resource)]
pub struct CurrentMotionWarp(Handle<MotionWarpClip>);

pub struct RebuildWarpClip;

#[derive(Resource)]
pub struct CurrentKeyframe(usize);

pub fn update_egui_hover(mut selected: ResMut<UiHovered>, mut contexts: EguiContexts) {
    selected.0 = contexts.ctx_mut().is_pointer_over_area();
}

pub fn egui_focused(egui_hover: Res<UiHovered>) -> bool {
    egui_hover.0
}

#[derive(States, Hash, Eq, PartialEq, Clone, Default, Debug)]
pub enum Mode {
    Settings,
    #[default]
    Preview,
    Keyframe,
}

pub fn pause_on_preview_exit(mut player: Query<&mut AnimationPlayer>,) {
    if let Ok(mut player) = player.get_single_mut() {
        player.pause();
    }
}

pub fn set_animation_time_on_keyframe_enter(
    mut player: Query<&mut AnimationPlayer>,
    keyframe: Res<CurrentKeyframe>,
    clip_builder: Res<MotionWarpClipBuilder>,
) {
    let Ok(mut player) = player.get_single_mut() else { return; };
    let time = clip_builder.clips[keyframe.0].time;
    player.set_elapsed(time);
}

pub fn build_warp_clip(
    current_animation: Res<CurrentAnimation>,
    mut current_keyframe: Option<ResMut<CurrentKeyframe>>,
    mut clip_builder: ResMut<MotionWarpClipBuilder>,
    mut commands: Commands,
    mut motion_warps: ResMut<Assets<MotionWarpClip>>,
    mut player: Query<&mut AnimationPlayer>,
    mut rebuild_ev: EventReader<RebuildWarpClip>,
    animations: Res<Assets<AnimationClip>>
) {
    if rebuild_ev.iter().next().is_some() {
        let Some(animation) = animations.get(&current_animation.0) else {
            warn!("Can't find animation clip.");
            return;
        };
        let Ok(mut player) = player.get_single_mut() else {
            warn!("No single animation player.");
            return;
        };
        if let Some(mut current_keyframe) = current_keyframe {
            let Some(current_keyframe_time) = clip_builder.clips.get(current_keyframe.0).map(|c| c.time) else { 
                warn!("There are no keyframes specified; can't build warp clip.");
                return; 
            };
            clip_builder.clips.sort_by(|a, b| a.time.total_cmp(&b.time));
            current_keyframe.0 = clip_builder.clips.binary_search_by(|clip| clip.time.partial_cmp(&current_keyframe_time).unwrap()).unwrap();
        }

        let handle = motion_warps.add(clip_builder.build(animation));
        player.play_warp(handle.clone());
        commands.insert_resource(CurrentMotionWarp(handle));
    }
}   

pub fn top_panel(
    mut contexts: EguiContexts,
    mode: Res<State<Mode>>,
    mut next_mode: ResMut<NextState<Mode>>,
) {
    egui::TopBottomPanel::top(egui::Id::new(TOP_PANEL_ID)).show(contexts.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            if ui.add_enabled(mode.0 != Mode::Settings, egui::Button::new("Settings")).clicked() {
                next_mode.0 = Some(Mode::Settings);
            }
            if ui.add_enabled(mode.0 != Mode::Preview, egui::Button::new("Preview")).clicked() {
                next_mode.0 = Some(Mode::Preview);
            }
            if mode.0 == Mode::Keyframe {
                let _ = ui.add_enabled(false, egui::Button::new("Keyframe"));
            }
            
        });
    });
}

pub fn timeline_panel(
    mut contexts: EguiContexts, 
    mut player: Query<&mut AnimationPlayer>,
    clips: Res<Assets<AnimationClip>>,
    current_motion_warp: Option<Res<CurrentMotionWarp>>,
    mode: Res<State<Mode>>,
    mut next_mode: ResMut<NextState<Mode>>,
) {
    egui::TopBottomPanel::bottom(egui::Id::new(TIMELINE_PANEL_ID)).show(contexts.ctx_mut(), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            let Ok(mut player) = player.get_single_mut() else { return; };

            if player.is_paused() {
                if ui.button("Play").clicked() {
                    player.resume();
                    if mode.0 == Mode::Keyframe {
                        next_mode.0 = Some(Mode::Preview);
                    }
                }
            }
            else if ui.button("Pause").clicked() {
                player.pause();
            }
            if let Some(current_motion_warp) = current_motion_warp {
                if player.is_warped() {
                    if ui.button("Unwarp").clicked() {
                        player.stop_warp();
                        
                    }
                }
                else if ui.button("Warp").clicked() {
                    player.play_warp(current_motion_warp.0.clone());
                }
            }
            

            let Some(clip) = clips.get(player.animation_clip()) else { return; };
            let duration = clip.duration();

            let mut elapsed = player.elapsed() % duration;

            ui.style_mut().spacing.slider_width = ui.available_width();
            if ui.add(egui::Slider::new(&mut elapsed, 0.0..=duration).show_value(true)).dragged() {
                if mode.0 == Mode::Keyframe {
                    next_mode.0 = Some(Mode::Preview);
                }
            };

            player.set_elapsed(elapsed);
        });
    });
}

pub fn keyframe_panel(
    mut contexts: EguiContexts, 
    mut clip_builder: ResMut<MotionWarpClipBuilder>,
    mut next_mode: ResMut<NextState<Mode>>,
    mut commands: Commands,
    player: Query<&AnimationPlayer>,
    current_keyframe: Option<Res<CurrentKeyframe>>,
    mode: Res<State<Mode>>,
) {
    egui::TopBottomPanel::bottom(egui::Id::new(KEYFRAME_PANEL_ID)).show(contexts.ctx_mut(), |ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.label("Keyframes:");

            if let Some(current_keyframe) = current_keyframe {
                for i in 0..clip_builder.clips.len() {
                    let enabled = i != current_keyframe.0 || mode.0 != Mode::Keyframe;
                    let button = ui.add_enabled(enabled, egui::Button::new(format!("{}", i)));
                    if button.clicked() {
                        next_mode.0 = Some(Mode::Keyframe);
                        commands.insert_resource(CurrentKeyframe(i));
                    }
                }
            }

            let Ok(player) = player.get_single() else { return; };
            let elapsed = player.elapsed();
            let time_non_overlapping = clip_builder.clips.iter().all(|clip| (clip.time - elapsed).abs() >= 1e-4);

            if time_non_overlapping && ui.button("+").clicked() {
                clip_builder.clips.push(MotionWarpClipFrame {
                    time: elapsed,
                    ..default()
                });
                next_mode.0 = Some(Mode::Keyframe);
                commands.insert_resource(CurrentKeyframe(clip_builder.clips.len() - 1));
            }
        });
    });
}


pub fn property_panel(
    mut contexts: EguiContexts, 
    mut clip_builder: ResMut<MotionWarpClipBuilder>,
    player: Query<&AnimationPlayer>,
    current_keyframe: Res<CurrentKeyframe>,
    animation_clips: Res<Assets<AnimationClip>>,
    mut joint_paths: Query<(&TrackedEntityPath, &mut Transform)>,
    mut rebuild_ev: EventWriter<RebuildWarpClip>
) {
    egui::SidePanel::left(egui::Id::new(PROPERTY_PANEL_ID)).show(contexts.ctx_mut(), |ui| {
        let Ok(player) = player.get_single() else { return; };

        let MotionWarpClipBuilder { 
            clips: motion_clips, 
            start_time, 
            end_time, 
            blend_margin, 
            tension 
        } = &mut *clip_builder;
        let Some(clip_frame) = motion_clips.get_mut(current_keyframe.0) else { return; };

        let mut rebuild: bool = false;


        let Some(clip) = animation_clips.get(player.animation_clip()) else { return; };
        let duration = clip.duration();

        ui.label("Start time:");
        rebuild |= ui.add(egui::DragValue::new(start_time).speed(0.001).clamp_range(0.0..=(duration-1e-4))).dragged();
        
        ui.label("End time:");
        rebuild |= ui.add(egui::DragValue::new(end_time).speed(0.001).clamp_range(1e-4..=duration)).dragged();

        ui.label("Blend margin:");
        rebuild |= ui.add(egui::DragValue::new(blend_margin).speed(0.001).clamp_range(0.0..=0.5)).dragged();

        ui.label("Tension:");
        rebuild |= ui.add(egui::DragValue::new(tension).speed(0.001).clamp_range(0.0..=1.0)).dragged();

        ui.label(format!("Frame time: {:?}", clip_frame.time));

        ui.label("Warp time: ");
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            match &mut clip_frame.warp_time {
                Some(t) => {
                    ui.add(egui::DragValue::new(t).speed(0.1));

                    *t = t.clamp(0.0, duration);

                    if ui.button("-").clicked() {
                        clip_frame.warp_time = None;
                        rebuild = true;
                    }                    
                },
                None => {
                    if ui.button("+").clicked() {
                        clip_frame.warp_time = Some(clip_frame.time);
                        rebuild = true;
                    }
                }
            }
        });

        ui.label("Add Joint");
        egui::ComboBox::from_label("").show_ui(ui, |ui| {
            for (path, transform) in joint_paths.iter() {
                if !clip_frame.map.contains_key(&path.0) {
                    if ui.button(format!("{:?}", path)).clicked() {
                        rebuild = true;
                        clip_frame.map.insert(
                            path.0.clone(), 
                            MotionWarpCurveFrame {
                                rotation: transform.rotation,
                                fix_a: true,
                            }
                        );
                    }
                }
            }
        });

        for (path, mut transform) in joint_paths.iter_mut() {
            let Some(MotionWarpCurveFrame { rotation: quat, mut fix_a }) = clip_frame.map.get(&path.0).cloned() else { continue; };
            let (mut x, mut y, mut z) = quat.to_euler(EulerRot::XYZ);
            let mut changed = false;

            egui::CollapsingHeader::new(format!("{:?}", path)).show(ui, |ui| {
                
                ui.label("x rotation:");
                let x_drag = ui.add(egui::DragValue::new(&mut x).speed(TAU/50.0));
                ui.label("y rotation:");
                let y_drag = ui.add(egui::DragValue::new(&mut y).speed(TAU/50.0));
                ui.label("z rotation:");
                let z_drag = ui.add(egui::DragValue::new(&mut z).speed(TAU/50.0));

                changed = x_drag.dragged() || y_drag.dragged() || z_drag.dragged();

                if ui.checkbox(&mut fix_a, "fix a").clicked() {
                    changed = true;
                }

                rebuild |= changed;

                ui.label("delete");
                if ui.button("-").clicked() {
                    clip_frame.map.remove_entry(&path.0);
                    rebuild = true;
                }
            });

            if changed {
                let new_quat = Quat::from_euler(EulerRot::XYZ, x, y, z);
                clip_frame.map.insert(
                    path.0.clone(), 
                    MotionWarpCurveFrame {
                        rotation: new_quat,
                        fix_a
                    }
                );
                transform.rotation = new_quat;
            }
        }

        if rebuild {
            rebuild_ev.send(RebuildWarpClip);
        }
    });
}

// TODO: add settings
pub fn settings_panel(
    mut contexts: EguiContexts, 
) {
    egui::SidePanel::left(egui::Id::new(SETTINGS_PANEL_ID)).show(contexts.ctx_mut(), |ui| {
        if ui.button("aaaaa").clicked() {
            println!("AAAAA");
        }
    });
}

#[derive(Component)]
pub struct SceneRoot;

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(200.0, 200.0, 200.0)
            .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }).insert(PrimaryCamera {
        focus: Vec3::ZERO,
        radius: Vec3::new(200.0, 200.0, 200.0).length(),
        upside_down: false
    });

    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 200.0,
            maximum_distance: 400.0,
            ..default()
        }
        .into(),
        ..default()
    });
}

// TODO: allow for models/animations to be loaded dynamically.
pub fn debug_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(CurrentAnimation(asset_server.load("Fox.glb#Animation2")));

    commands.spawn(SceneBundle {
        scene: asset_server.load("Fox.glb#Scene0"),
        ..default()
    }).insert(SceneRoot);
}

pub fn play_once_loaded(
    animation: Res<CurrentAnimation>,
    mut player: Query<&mut AnimationPlayer>,
    mut done: Local<bool>,
) {
    if !*done {
        if let Ok(mut player) = player.get_single_mut() {
            player.play(animation.0.clone_weak()).repeat();
            *done = true;
        }
    }
}
