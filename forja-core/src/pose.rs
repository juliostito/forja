//! Exercise detection from body keypoints.
//!
//! Input: a stream of [`Joints`] in image coordinates (y grows downward), as produced on the iPhone by
//! Apple Vision's body-pose request (head, neck, shoulders, elbows, wrists, root, hips, knees, ankles) or,
//! for tests and the desktop demo, by the animated skeletons in [`crate::motion`]. Everything runs on the
//! device; no image is stored.
//!
//! What it does: classifies the movement (squat, hinge, bridge, push, press, pull, curl, core), counts
//! reps with a low-pass filter and hysteresis, checks squat depth (hip vs knee) and gives an orientative
//! posture warning when the head/upper back drops out of line with the torso. When confidence is low the
//! app only suggests and the user confirms.

use crate::catalog::exercise;
use crate::motion::{Joints, Pt};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Movement {
    Squat,
    Hinge,
    Bridge,
    Push,
    Press,
    Pull,
    Curl,
    Core,
    Unknown,
}

impl Movement {
    pub fn label(self) -> &'static str {
        match self {
            Movement::Squat => "Sentadilla",
            Movement::Hinge => "Bisagra de cadera (peso muerto)",
            Movement::Bridge => "Puente / hip thrust",
            Movement::Push => "Flexión",
            Movement::Press => "Press",
            Movement::Pull => "Remo",
            Movement::Curl => "Curl",
            Movement::Core => "Core (McGill)",
            Movement::Unknown => "Sin movimiento claro",
        }
    }

    /// Words in catalogue ids that point at this movement, most specific first.
    fn keywords(self) -> &'static [&'static str] {
        match self {
            Movement::Squat => &["squat", "wall_sit"],
            Movement::Hinge => &["rdl", "deadlift"],
            Movement::Bridge => &["thrust", "bridge"],
            Movement::Push => &["pushup"],
            Movement::Press => &["press", "pullover", "triceps"],
            Movement::Pull => &["row", "fly"],
            Movement::Curl => &["curl", "raise"],
            Movement::Core => &["bird_dog", "dead_bug", "plank", "curl_up"],
            Movement::Unknown => &[],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Frame {
    knee: f32,
    hip: f32,
    elbow: [f32; 2],
    tilt: f32,
    root_y: f32,
    shoulder_y: f32,
    wrist_rel: f32,
    wrist_y: f32,
    head_y: f32,
    spread: f32,
    neck_flex: f32,
    hip_minus_knee: f32,
}

/// Angle at `b` (degrees, 0..180).
fn angle(a: Pt, b: Pt, c: Pt) -> f32 {
    let (v1, v2) = ((a.x - b.x, a.y - b.y), (c.x - b.x, c.y - b.y));
    let n = ((v1.0 * v1.0 + v1.1 * v1.1) * (v2.0 * v2.0 + v2.1 * v2.1))
        .sqrt()
        .max(1e-6);
    ((v1.0 * v2.0 + v1.1 * v2.1) / n)
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees()
}

fn features(j: &Joints) -> Frame {
    let s = j.neck.dist(j.root).max(1.0);
    let avg = |f: &dyn Fn(usize) -> f32| (f(0) + f(1)) / 2.0;
    let torso = (j.neck.x - j.root.x, j.neck.y - j.root.y);
    let tilt = (-torso.1 / s).clamp(-1.0, 1.0).acos().to_degrees();
    let head_dir = (j.head.x - j.neck.x, j.head.y - j.neck.y);
    let hn = (head_dir.0 * head_dir.0 + head_dir.1 * head_dir.1)
        .sqrt()
        .max(1e-6);
    let neck_flex = ((torso.0 * head_dir.0 + torso.1 * head_dir.1) / (s * hn))
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();
    let shoulder_y = (j.shoulder[0].y + j.shoulder[1].y) / 2.0;
    Frame {
        knee: avg(&|i| angle(j.hip[i], j.knee[i], j.ankle[i])),
        hip: avg(&|i| angle(j.neck, j.hip[i], j.knee[i])),
        elbow: [0, 1].map(|i| angle(j.shoulder[i], j.elbow[i], j.wrist[i])),
        tilt,
        root_y: j.root.y / s,
        shoulder_y: shoulder_y / s,
        wrist_rel: (j.wrist[0].y.min(j.wrist[1].y) - shoulder_y) / s,
        wrist_y: j.wrist[0].y.min(j.wrist[1].y) / s,
        head_y: j.head.y / s,
        spread: j.wrist[0].dist(j.ankle[1]).max(j.wrist[1].dist(j.ankle[0])) / s,
        neck_flex,
        hip_minus_knee: (j.root.y - (j.knee[0].y + j.knee[1].y) / 2.0) / s,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Detection {
    pub movement: Movement,
    /// 0..1. Below [`CONFIDENT`] the app only suggests and the user confirms.
    pub confidence: f32,
    pub reps: u32,
    /// Squats: hip reached (about) knee height at the bottom.
    pub depth_ok: Option<bool>,
    /// Orientative: head/upper back dropped out of line with the torso.
    pub posture_warning: bool,
    /// Best matching exercise among the candidates (today's plan).
    pub exercise_id: Option<&'static str>,
}

pub const CONFIDENT: f32 = 0.6;

#[derive(Clone, Debug, Default)]
pub struct Detector {
    frames: Vec<Frame>,
    max_frames: usize,
}

fn range(v: impl Iterator<Item = f32>) -> f32 {
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for x in v {
        lo = lo.min(x);
        hi = hi.max(x);
    }
    if lo > hi {
        0.0
    } else {
        hi - lo
    }
}

/// Reps by hysteresis on a low-pass filtered, normalised signal: one rep = leave the starting side and
/// come back.
pub fn count_reps(signal: &[f32]) -> u32 {
    if signal.len() < 3 {
        return 0;
    }
    let mut smooth = Vec::with_capacity(signal.len());
    let mut acc = signal[0];
    for &x in signal {
        acc += 0.35 * (x - acc);
        smooth.push(acc);
    }
    let lo = smooth.iter().copied().fold(f32::MAX, f32::min);
    let hi = smooth.iter().copied().fold(f32::MIN, f32::max);
    if hi - lo < 1e-6 {
        return 0;
    }
    let norm: Vec<f32> = smooth.iter().map(|x| (x - lo) / (hi - lo)).collect();
    let start_high = norm[0] > 0.5;
    let mut away = false;
    let mut reps = 0;
    for &x in &norm {
        let high = x > 0.7;
        let low = x < 0.3;
        if !away && ((start_high && low) || (!start_high && high)) {
            away = true;
        } else if away && ((start_high && high) || (!start_high && low)) {
            away = false;
            reps += 1;
        }
    }
    reps
}

impl Detector {
    /// Keeps the last `max_frames` frames (e.g. 30 fps × 20 s = 600).
    pub fn new(max_frames: usize) -> Self {
        Detector {
            frames: Vec::new(),
            max_frames: max_frames.max(10),
        }
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }

    pub fn push(&mut self, j: &Joints) {
        self.frames.push(features(j));
        if self.frames.len() > self.max_frames {
            let extra = self.frames.len() - self.max_frames;
            self.frames.drain(..extra);
        }
    }

    pub fn detect(&self, candidates: &[&'static str]) -> Detection {
        let f = &self.frames;
        let unknown = Detection {
            movement: Movement::Unknown,
            confidence: 0.0,
            reps: 0,
            depth_ok: None,
            posture_warning: false,
            exercise_id: None,
        };
        if f.len() < 10 {
            return unknown;
        }
        let r_knee = range(f.iter().map(|x| x.knee));
        let r_hip = range(f.iter().map(|x| x.hip));
        let elbow_ranges = [0, 1].map(|i| range(f.iter().map(|x| x.elbow[i])));
        let arm = if elbow_ranges[0] >= elbow_ranges[1] {
            0
        } else {
            1
        };
        let r_elbow = elbow_ranges[arm];
        let r_root = range(f.iter().map(|x| x.root_y));
        let r_shoulder = range(f.iter().map(|x| x.shoulder_y));
        let r_spread = range(f.iter().map(|x| x.spread));
        let mean_tilt = f.iter().map(|x| x.tilt).sum::<f32>() / f.len() as f32;
        let wrists_above = f.iter().filter(|x| x.wrist_rel < -0.2).count() as f32 / f.len() as f32;
        let overhead = f.iter().any(|x| x.wrist_y < x.head_y);

        // (movement, signal, amplitude relative to a clear rep)
        let (movement, signal, amplitude): (Movement, Vec<f32>, f32) = if mean_tilt > 55.0 {
            if wrists_above > 0.6 && r_elbow > 40.0 {
                (
                    Movement::Press,
                    f.iter().map(|x| x.wrist_y).collect(),
                    r_elbow / 80.0,
                )
            } else if r_shoulder > 0.15 && r_elbow > 30.0 {
                (
                    Movement::Push,
                    f.iter().map(|x| x.shoulder_y).collect(),
                    r_shoulder / 0.3,
                )
            } else if r_spread > 0.6 {
                (
                    Movement::Core,
                    f.iter().map(|x| x.spread).collect(),
                    r_spread / 1.0,
                )
            } else if r_root > 0.25 && r_hip > 20.0 {
                (
                    Movement::Bridge,
                    f.iter().map(|x| x.root_y).collect(),
                    r_root / 0.4,
                )
            } else if r_elbow > 40.0 {
                (
                    Movement::Pull,
                    f.iter().map(|x| x.elbow[arm]).collect(),
                    r_elbow / 70.0,
                )
            } else {
                (Movement::Unknown, Vec::new(), 0.0)
            }
        } else if r_knee > 35.0 && r_root > 0.25 {
            (
                Movement::Squat,
                f.iter().map(|x| x.root_y).collect(),
                r_root / 0.45,
            )
        } else if r_hip > 35.0 {
            (
                Movement::Hinge,
                f.iter().map(|x| x.hip).collect(),
                r_hip / 60.0,
            )
        } else if r_elbow > 50.0 {
            if overhead {
                (
                    Movement::Press,
                    f.iter().map(|x| x.wrist_y).collect(),
                    r_elbow / 80.0,
                )
            } else {
                (
                    Movement::Curl,
                    f.iter().map(|x| x.elbow[arm]).collect(),
                    r_elbow / 90.0,
                )
            }
        } else {
            (Movement::Unknown, Vec::new(), 0.0)
        };
        if movement == Movement::Unknown {
            return unknown;
        }
        let reps = count_reps(&signal);
        let confidence = (amplitude.min(1.0) * 0.8 + if reps > 0 { 0.2 } else { 0.0 }).min(1.0);
        let depth_ok = (movement == Movement::Squat).then(|| {
            let bottom = f
                .iter()
                .max_by(|a, b| a.root_y.total_cmp(&b.root_y))
                .expect("frames");
            bottom.hip_minus_knee >= -0.15
        });
        let posture_warning = matches!(movement, Movement::Hinge | Movement::Squat)
            && f.iter().any(|x| x.neck_flex > 35.0);
        let exercise_id = movement
            .keywords()
            .iter()
            .find_map(|kw| candidates.iter().copied().find(|id| id.contains(kw)))
            .filter(|id| exercise(id).is_some());
        Detection {
            movement,
            confidence,
            reps,
            depth_ok,
            posture_warning,
            exercise_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motion::{motion_for, sample};

    fn run(id: &str, cycles: f32) -> Detection {
        let m = motion_for(id);
        let mut d = Detector::new(2000);
        let n = (m.period * cycles * 30.0) as usize;
        for i in 0..=n {
            d.push(&sample(&m, i as f32 / 30.0));
        }
        let today = [
            "incline_pushup",
            "one_arm_row",
            "db_bench_press",
            "biceps_curl",
            "goblet_box_squat",
            "romanian_deadlift",
            "hip_thrust",
            "seated_shoulder_press",
            "chest_supported_row",
            "bird_dog",
        ];
        d.detect(&today)
    }

    #[test]
    fn recognises_movements_and_counts_reps_on_the_animations() {
        let cases = [
            ("goblet_box_squat", Movement::Squat, "goblet_box_squat"),
            ("romanian_deadlift", Movement::Hinge, "romanian_deadlift"),
            ("hip_thrust", Movement::Bridge, "hip_thrust"),
            ("incline_pushup", Movement::Push, "incline_pushup"),
            ("db_bench_press", Movement::Press, "db_bench_press"),
            ("seated_shoulder_press", Movement::Press, "db_bench_press"),
            ("chest_supported_row", Movement::Pull, "one_arm_row"),
            ("biceps_curl", Movement::Curl, "biceps_curl"),
            ("bird_dog", Movement::Core, "bird_dog"),
        ];
        for (id, mv, ex) in cases {
            let d = run(id, 3.0);
            assert_eq!(d.movement, mv, "{id}: {d:?}");
            assert_eq!(d.reps, 3, "{id}: {d:?}");
            assert!(d.confidence >= CONFIDENT, "{id}: {d:?}");
            assert_eq!(d.exercise_id, Some(ex), "{id}");
        }
    }

    #[test]
    fn squat_depth_is_checked() {
        let d = run("goblet_box_squat", 2.0);
        assert_eq!(d.depth_ok, Some(true));
        let shallow = run("high_box_squat_bw", 2.0);
        assert_eq!(shallow.movement, Movement::Squat);
        assert_eq!(shallow.depth_ok, Some(false), "{shallow:?}");
    }

    #[test]
    fn standing_still_is_unknown() {
        let m = motion_for("__none__");
        let mut d = Detector::new(300);
        for i in 0..200 {
            d.push(&sample(&m, i as f32 / 30.0));
        }
        assert_eq!(d.detect(&[]).movement, Movement::Unknown);
    }

    #[test]
    fn hysteresis_ignores_jitter() {
        // 5.5 cycles of 2 s starting at the bottom, with jitter.
        let s: Vec<f32> = (0..330)
            .map(|i| {
                -(i as f32 / 30.0 * std::f32::consts::PI).cos() + ((i * 7919) % 13) as f32 * 0.01
            })
            .collect();
        assert_eq!(count_reps(&s), 5);
    }
}
