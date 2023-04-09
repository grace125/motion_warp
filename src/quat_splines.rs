use bevy::prelude::{Quat, Vec3};
use itertools::Itertools;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct CardinalQuatCurve {
    tension: f32,
    controls: Vec<(Quat, f32)>
}

impl CardinalQuatCurve {

    pub fn new(
        tension: f32, 
        control_points: impl IntoIterator<Item = (Quat, f32)>
    ) -> CardinalQuatCurve 
    {
        let mut controls = control_points.into_iter().collect::<Vec<_>>();

        assert!(-1.0 <= tension && tension <= 1.0);
        assert!(controls.len() >= 2);

        for window in controls.windows(2) {
            let &[(_, t0), (_, t1)] = window else { continue };
            assert!(t0 < t1);
        }

        // Canonicalize points
        let mut a = controls[0].0.clone();
        for (b, _) in controls.iter_mut().skip(1) {
            if a.dot(*b) < 0.0 {
                *b = -*b;
                a = *b;
            }
        }

        CardinalQuatCurve { controls, tension }
    }

    pub fn to_curve(&self) -> DeCasteljauQuatCurve {
        let len = self.controls.len();

        let control_points = self
            .controls
            .windows(3)
            .map(|window| {
                let &[(q0, t0), (q1, t1), (q2, t2)] = window else { panic!("window iterator is incorrect") };
                let h1 = t1 - t0;
                let h2 = t2 - t1;
                
                let q_in = q1 * q0.inverse();
                let q_out = q2 * q1.inverse();
                let rho_in = log_map(q_in) / h1;
                let rho_out = log_map(q_out) / h2;

                let omega = 
                    (rho_in * h2 + rho_out * h1) 
                    / (h1 + h2)
                    * (1.0-self.tension);

                [exp_map(-omega * h1 / 3.0) * q1, exp_map(omega * h2 / 3.0) * q1]
            }).flatten();

        let first_control = [end_quaternion(self.controls[0].0, self.controls[1].0)].into_iter();
        let last_control = [end_quaternion(self.controls[len-1].0, self.controls[len-2].0)].into_iter();

        let control_points = first_control.chain(control_points).chain(last_control).tuples::<(Quat, Quat)>();
        
        let (segments, mut times): (Vec<_>, Vec<_>) = self
            .controls
            .windows(2)
            .zip(control_points)
            .map(|(window, (a, b))| {
                let &[(q0, t0), (q1, _)] = window else { panic!("window iterator is incorrect") };
                (
                    DeCasteljauQuatSegment { coeff: [q0, a, b, q1] }, 
                    t0
                )
            })
            .unzip();

        times.push(self.controls.last().unwrap().1);

        DeCasteljauQuatCurve {
            segments,
            times
        }
    }
}

fn log_map(q: Quat) -> Vec3 {
    if q.w >= 1.0 {
        (0.0, 0.0, 0.0).into()
    }
    else {
        let (angle, axis) = q.to_axis_angle();
        axis * (angle / 2.0)
    }
}

fn exp_map(e: Vec3) -> Quat {
    let normal = e.length();
    if normal <= 1e-6 {
        Quat::IDENTITY
    }
    else {
        let s = normal.sin() / normal;
        Quat::from_xyzw(e.x * s, e.y * s, e.z * s, normal.cos())
    }
}

fn quat_pow_f(q: Quat, f: f32) -> Quat {
    let (axis, angle) = q.to_axis_angle();
    Quat::from_axis_angle(axis, angle*f)
}

fn end_quaternion(first: Quat, third: Quat) -> Quat {
    quat_pow_f(third * first.inverse(), 1.0/3.0) * first
}

/// An implementation of bisection.
/// 
/// # Arguments
/// 
/// * `left` - some value such that `cmp(left) != Ordering::Greater`
/// * `right` - some value such that `cmp(right) != Ordering::Less`
/// * `cmp` - some continuous function
pub fn bisect(left: f32, right: f32, cmp: impl Fn(f32) -> Ordering) -> f32 {
    let middle = (right - left)*0.5;
    match cmp(middle) {
        Ordering::Greater => bisect(middle, right, cmp),
        Ordering::Less => bisect(left, middle, cmp),
        Ordering::Equal => middle
    }
}

#[derive(Clone, Debug)]
pub struct DeCasteljauQuatSegment {
    coeff: [Quat; 4]
}

impl DeCasteljauQuatSegment {

    #[inline]
    pub fn position(&self, t: f32) -> Quat {
        let [a, b, c, d] = self.coeff;
        let ab = a.slerp(b, t);
        let bc = b.slerp(c, t);
        let cd = c.slerp(d, t);
        let abc = ab.slerp(bc, t);
        let bcd = bc.slerp(cd, t);
        abc.slerp(bcd, t)
    }
}

#[derive(Clone, Debug)]
pub struct DeCasteljauQuatCurve {
    segments: Vec<DeCasteljauQuatSegment>,
    times: Vec<f32>,
}

impl DeCasteljauQuatCurve {

    #[inline]
    pub fn position(&self, t: f32) -> Quat {
        let (segment, t) = self.segment(t);
        return segment.position(t);
    }

    #[inline]
    pub fn segment(&self, t: f32) -> (&DeCasteljauQuatSegment, f32) {
        let index = self.times.partition_point(|probe| *probe <= t).clamp(0, self.segments.len());
        let segment = &self.segments[index-1];
        let t0 = self.times[index-1];
        let t1 = self.times[index];
        let t = (t - t0) / (t1 - t0);
        (segment, t)
    }

    #[inline]
    pub fn iter_positions(&self, subdivisions: usize) -> impl Iterator<Item = Quat> + '_ 
    {
        let segments = self.segments.len() as f32;
        (0..subdivisions).map(move |i| {
            let t = segments * i as f32 / subdivisions as f32;
            self.position(t)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const ERROR_BOUND: f32 = 1e-6;
    const SOFT_ERROR_BOUND: f32 = 1e-3;

    /// Checks that control points evaluate properly
    #[test]
    fn test_control_points() {
        let quats = [
            (Quat::IDENTITY, 0.0f32), 
            (Quat::from_axis_angle(Vec3::X, PI/2.0), 1.0), 
            (Quat::from_axis_angle(Vec3::Y, PI/2.0), 2.0), 
            (Quat::from_axis_angle(Vec3::Z, PI/2.0), PI)
            ];

        let curve = CardinalQuatCurve::new(0.0, quats.clone()).to_curve();
            
        for (q, t) in quats {
            let diff = curve.position(t) * q.inverse();
            assert!(diff.x.abs() < ERROR_BOUND);
            assert!(diff.y.abs() < ERROR_BOUND);
            assert!(diff.z.abs() < ERROR_BOUND);
            assert!((diff.w - 1.0).abs() < ERROR_BOUND);
        }
    }

    // Checks that the rotation interpolating quaternions in a straight line doesn't
    // deviate from that line between controls
    #[test]
    fn test_between_controls() {
        let curve = CardinalQuatCurve::new( 
            0.0,
            [
                (Quat::IDENTITY, 0.0), 
                (Quat::from_axis_angle(Vec3::Y, PI*0.1), 1.0), 
                (Quat::from_axis_angle(Vec3::Y, PI*0.2), 2.0),
                (Quat::from_axis_angle(Vec3::Y, PI*0.3), 4.0),
                (Quat::from_axis_angle(Vec3::Y, PI*0.4), 4.5),
                (Quat::from_axis_angle(Vec3::Y, PI*0.5), 5.2),
                (Quat::from_axis_angle(Vec3::Y, PI*0.6), 20.3)
            ])
            .to_curve();

        for q in curve.iter_positions(500) {
            assert!(q.x.abs() < ERROR_BOUND);
            assert!(q.z.abs() < ERROR_BOUND);
        }
    }

    // Checks that, when tension is 1.0, the spline moves in a straight line
    #[test]
    fn test_max_tension() {
        const N: u32 = 500;

        let quats = [
            Quat::IDENTITY, 
            Quat::from_axis_angle(Vec3::X, PI/2.0), 
            Quat::from_axis_angle(Vec3::Y, PI/2.0),
            Quat::from_axis_angle(Vec3::Z, PI/2.0)
        ];

        let curve = CardinalQuatCurve::new(1.0, quats.iter().enumerate().map(|(i, q)| (*q, i as f32))).to_curve();


        for (i, window) in quats.windows(2).enumerate() {

            let &[q1, q2] = window else { continue };
            let (axis1, _) = (q2 * q1.inverse()).to_axis_angle();

            for j in (N/10)..=(9*N/10) {
                let t = i as f32 + j as f32 / N as f32;
                let s = curve.position(t);
                let diff = s * q1.inverse(); // The difference between q and s
                let (axis2, _) = diff.to_axis_angle();

                // println!("{:?}; {:?}", t, axis1.dot(axis2).abs());
                let error = axis1.dot(axis2) - 1.0;

                assert!(error.abs() < SOFT_ERROR_BOUND);
            }
        }
    }

    #[test]
    fn test_quat_pow_f() {
        const NUM_ITERATIONS: u32 = 30;

        let q = quat_pow_f(Quat::from_axis_angle(Vec3::Y, PI/2.0), 4.0/NUM_ITERATIONS as f32);
        let mut r = Vec3::X;

        for _ in 0..(NUM_ITERATIONS) {
            r = q*r;
        }

        let error = r.dot(Vec3::X) - 1.0;

        assert!(error.abs() < ERROR_BOUND)
    }
}