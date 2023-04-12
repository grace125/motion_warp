use std::cmp::Ordering;

use bevy::{prelude::{Vec2, Quat}, reflect::{TypeUuid}, utils::HashMap, math::cubic_splines::CubicCurve};

use crate::{EntityPath, quat_splines::{DeCasteljauQuatCurve, bisect}};

const MAX_ERROR: f32 = 1e-5;

pub mod builder {

    use std::collections::VecDeque;

    use bevy::{prelude::{Quat, Resource, CardinalSpline, CubicGenerator}, reflect::{FromReflect, Reflect}};

    use crate::{AnimationClip, quat_splines::CardinalQuatCurve};

    use super::*;

    #[derive(Reflect, FromReflect, Default, Clone)]
    pub struct MotionWarpCurveFrame {
        pub rotation: Quat,
        pub fix_a: bool,
    }

    #[derive(Reflect, FromReflect, Default, Clone, )]
    pub struct MotionWarpClipFrame {
        pub time: f32,
        pub warp_time: Option<f32>,
        pub map: HashMap<EntityPath, MotionWarpCurveFrame>
    }

    #[derive(Reflect, FromReflect, Resource, Clone)]
    pub struct MotionWarpClipBuilder {
        pub clips: Vec<MotionWarpClipFrame>,
        pub start_time: f32,
        pub end_time: f32,
        pub blend_margin: f32,
        pub tension: f32,
    }

    impl MotionWarpClipBuilder {

        // Efficient? No. Good enough for now? Yes.
        // TODO: debug the shit out of this
        // TODO: add way more asserts
        pub fn build(&mut self, clip: &AnimationClip) -> MotionWarpClip {
            assert!(self.start_time < self.end_time);

            self.clips.sort_by(|a, b| a.time.total_cmp(&b.time));

            let duration = clip.duration();
            
            let g = {
                let duration_splat = Vec2::new(duration, duration);
                let mut times: VecDeque<_> = self
                    .clips
                    .iter()
                    .filter_map(|clip_frame| 
                        clip_frame.warp_time.map(|warp_time| Vec2::new(clip_frame.time, warp_time))
                    ).collect();
                if times.is_empty() {
                    times.push_back(Vec2::new(0.0, 0.0));
                }
                times.push_back(*times.front().unwrap() + duration_splat);
                times.push_front(*times.back().unwrap() - duration_splat);
                
                CardinalSpline::new(self.tension, times).to_curve()
            };

            let (curves, paths) = {
                let mut paths: HashMap<EntityPath, usize> = HashMap::new();
                let mut curves: Vec<MotionWarpCurve> = Vec::new();

                for (i, clip_frame) in self.clips.iter().enumerate() {
                    for path in clip_frame.map.keys() {
                        if !paths.contains_key(path) {

                            paths.insert(path.clone(), curves.len());

                            let (mut a_params, mut b_params): (VecDeque<_>, VecDeque<_>) = self
                                .clips
                                .iter()
                                .skip(i.max(1) - 1)
                                .filter_map(|frame| frame.map
                                    .get(path)
                                    .map(|some_frame| (frame.time, some_frame))
                                )
                                .map(|(t, frame)| {
                                    let theta = clip.get_joint_rotation_at(path, t);
                                    let MotionWarpCurveFrame { rotation: theta_prime, fix_a } = frame;
                                    if *fix_a {
                                        let a = Quat::IDENTITY;
                                        let b = theta - *theta_prime;
                                        ((a, t), (b, t))
                                    }
                                    else {
                                        let b = Quat::IDENTITY;
                                        let a = (*theta_prime - b)*theta.inverse();
                                        ((a, t), (b, t))
                                    }
                                })
                                .unzip();

                            let mut a_start_control = *a_params.back().unwrap();
                            let mut a_end_control = *a_params.front().unwrap();
                            let mut b_start_control = *b_params.back().unwrap();
                            let mut b_end_control = *b_params.front().unwrap();
                            a_start_control.1 -= duration;
                            a_end_control.1 += duration;
                            b_start_control.1 -= duration;
                            b_end_control.1 += duration;
                            a_params.push_front(a_start_control);
                            a_params.push_back(a_end_control);
                            b_params.push_front(b_start_control);
                            b_params.push_back(b_end_control);

                            let curve = MotionWarpCurve {
                                a: CardinalQuatCurve::new(self.tension, a_params).to_curve(),
                                b: CardinalQuatCurve::new(self.tension, b_params).to_curve(),
                            };

                            curves.push(curve);
                        }
                    }
                }

                (curves, paths)
            };

            MotionWarpClip {
                curves,
                paths,
                g,
                start_time: self.start_time,
                end_time: self.end_time,
                blend_margin: self.blend_margin,
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct MotionWarpCurve {
    a: DeCasteljauQuatCurve,
    b: DeCasteljauQuatCurve
}

impl MotionWarpCurve {

    #[inline]
    pub fn theta_prime(&self, t: f32, theta: Quat) -> Quat {
        self.a.position(t) * theta + self.b.position(t)
    }
}

#[derive(Clone, TypeUuid, Debug, Default)]
#[uuid = "7f06b317-fe2e-4bc9-ac6f-a5aa6d7b6a49"]
pub struct MotionWarpClip {
    pub(crate) curves: Vec<MotionWarpCurve>,
    pub(crate) paths: HashMap<EntityPath, usize>,
    g: CubicCurve<Vec2>,
    pub(crate) start_time: f32,
    pub(crate) end_time: f32,
    pub(crate) blend_margin: f32,
}

impl MotionWarpClip {

    /// Maps from "warped time" to "unwarped time"
    #[inline]
    pub fn g(&self, t_prime: f32) -> f32 {
        bisect(self.start_time, self.end_time, |t| {
            let pos = self.g.position(t);
            let error = pos.x - t_prime;

            if error > MAX_ERROR {
                Ordering::Greater
            }
            else if error < -MAX_ERROR {
                Ordering::Less
            }
            else {
                Ordering::Equal
            }
        })
    }

    // TODO: test/document me
    #[inline]
    pub fn theta_blend(&self, curve: &MotionWarpCurve, t: f32, theta: Quat) -> Quat {
        let theta_prime = curve.theta_prime(t, theta);
        let omega = self.omega(t);
        theta.slerp(theta_prime.normalize(), omega)
    }

    // TODO: test me
    #[inline]
    fn p_blend(t: f32) -> f32 {
        let t_squared = t * t;
        0.75 * t_squared / (t_squared - t + 1.0)
    }

    // TODO: test me
    #[inline]
    pub fn omega(&self, t: f32) -> f32 {
        let duration = self.end_time - self.start_time;
        // t is now from 0 to 1
        let t = (t - self.start_time)/duration;
        if t <= self.blend_margin {
            MotionWarpClip::p_blend(2.0*t/self.blend_margin)
        }
        else if t >= 1.0 - self.blend_margin {
            MotionWarpClip::p_blend(2.0*(1.0 - t)/self.blend_margin)
        }
        else {
            1.0
        }
    }
}