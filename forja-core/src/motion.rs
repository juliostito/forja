//! Exercise animations as data: a 2D skeleton with start/end keyframes per exercise (and per warm-up and
//! stretch item). Each keyframe places the pelvis, tilts the torso and sets hand/foot targets; elbows and
//! knees come from two-bone inverse kinematics. Pure maths, no drawing: the apps draw the joints, and the
//! pose detector is tested against the same skeletons.
//!
//! Coordinates: units ≈ cm/1.7, floor at y = 0, **y grows downward** (image convention), the figure faces
//! +x. Angles in degrees: 0 = up, 90 = facing direction, 180 = down.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

impl Pt {
    pub const fn new(x: f32, y: f32) -> Self {
        Pt { x, y }
    }
    pub fn lerp(self, o: Pt, t: f32) -> Pt {
        Pt::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }
    pub fn dist(self, o: Pt) -> f32 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt()
    }
}

pub const TORSO: f32 = 32.0;
pub const HEAD: f32 = 12.0;
pub const UPPER_ARM: f32 = 15.0;
pub const FOREARM: f32 = 14.0;
pub const THIGH: f32 = 22.0;
pub const SHIN: f32 = 22.0;

fn dir(deg: f32) -> Pt {
    let r = deg.to_radians();
    Pt::new(r.sin(), -r.cos())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Key {
    pub hip: Pt,
    pub torso: f32,
    /// Head angle relative to the torso.
    pub head: f32,
    pub hands: [Pt; 2],
    pub feet: [Pt; 2],
}

impl Key {
    fn lerp(&self, o: &Key, t: f32) -> Key {
        Key {
            hip: self.hip.lerp(o.hip, t),
            torso: self.torso + (o.torso - self.torso) * t,
            head: self.head + (o.head - self.head) * t,
            hands: [
                self.hands[0].lerp(o.hands[0], t),
                self.hands[1].lerp(o.hands[1], t),
            ],
            feet: [
                self.feet[0].lerp(o.feet[0], t),
                self.feet[1].lerp(o.feet[1], t),
            ],
        }
    }
    fn head(mut self, h: f32) -> Key {
        self.head = h;
        self
    }
    fn hand0(mut self, x: f32, y: f32) -> Key {
        self.hands[0] = Pt::new(x, y);
        self
    }
    fn hand1(mut self, x: f32, y: f32) -> Key {
        self.hands[1] = Pt::new(x, y);
        self
    }
    fn foot0(mut self, x: f32, y: f32) -> Key {
        self.feet[0] = Pt::new(x, y);
        self
    }
    fn foot1(mut self, x: f32, y: f32) -> Key {
        self.feet[1] = Pt::new(x, y);
        self
    }
}

/// Keyframe: hip position, torso angle, both hands at (hx, hy), both feet at (fx, fy).
fn k(hip: (f32, f32), torso: f32, hand: (f32, f32), foot: (f32, f32)) -> Key {
    Key {
        hip: Pt::new(hip.0, hip.1),
        torso,
        head: 0.0,
        hands: [Pt::new(hand.0, hand.1); 2],
        feet: [Pt::new(foot.0, foot.1); 2],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum Prop {
    /// Bench or box: centre x, width, height.
    Block {
        x: f32,
        w: f32,
        h: f32,
    },
    Wall {
        x: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum View {
    Side,
    /// Facing the camera; limb 1 mirrors limb 0.
    Front,
}

#[derive(Clone, Debug)]
pub struct Motion {
    pub a: Key,
    pub b: Key,
    pub view: View,
    /// Bend side for each elbow / knee (±1).
    pub elbow: [f32; 2],
    pub knee: [f32; 2],
    /// Dumbbell in each hand.
    pub weights: [bool; 2],
    pub props: Vec<Prop>,
    /// Spine shown green (neutral). False only for mobility like cat-camel.
    pub neutral_spine: bool,
    /// Seconds for start → end → start.
    pub period: f32,
}

impl Motion {
    fn new(a: Key, b: Key) -> Self {
        Motion {
            a,
            b,
            view: View::Side,
            elbow: [1.0, 1.0],
            knee: [-1.0, -1.0],
            weights: [false, false],
            props: Vec::new(),
            neutral_spine: true,
            period: 3.2,
        }
    }
    fn front(mut self) -> Self {
        self.view = View::Front;
        self
    }
    fn both(mut self) -> Self {
        self.weights = [true, true];
        self
    }
    fn one(mut self) -> Self {
        self.weights = [true, false];
        self
    }
    fn elbows(mut self, e0: f32, e1: f32) -> Self {
        self.elbow = [e0, e1];
        self
    }
    fn knees(mut self, k0: f32, k1: f32) -> Self {
        self.knee = [k0, k1];
        self
    }
    fn bench(mut self, x: f32, w: f32) -> Self {
        self.props.push(Prop::Block { x, w, h: 18.0 });
        self
    }
    fn block(mut self, x: f32, w: f32, h: f32) -> Self {
        self.props.push(Prop::Block { x, w, h });
        self
    }
    fn wall(mut self, x: f32) -> Self {
        self.props.push(Prop::Wall { x });
        self
    }
    fn mobility(mut self) -> Self {
        self.neutral_spine = false;
        self
    }
    fn period(mut self, p: f32) -> Self {
        self.period = p;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
pub struct Joints {
    pub head: Pt,
    /// Between the shoulders.
    pub neck: Pt,
    /// Pelvis centre.
    pub root: Pt,
    pub shoulder: [Pt; 2],
    pub elbow: [Pt; 2],
    pub wrist: [Pt; 2],
    pub hip: [Pt; 2],
    pub knee: [Pt; 2],
    pub ankle: [Pt; 2],
}

/// Two-bone IK: returns (middle joint, end point). The end stops short when the target is out of reach.
pub fn two_bone(root: Pt, target: Pt, l1: f32, l2: f32, bend: f32) -> (Pt, Pt) {
    let (dx, dy) = (target.x - root.x, target.y - root.y);
    let d0 = (dx * dx + dy * dy).sqrt().max(1e-4);
    let (ux, uy) = (dx / d0, dy / d0);
    let d = d0.clamp((l1 - l2).abs() + 0.01, l1 + l2 - 0.01);
    let a = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let h = (l1 * l1 - a * a).max(0.0).sqrt();
    let mid = Pt::new(root.x + ux * a, root.y + uy * a);
    let joint = Pt::new(mid.x - uy * h * bend, mid.y + ux * h * bend);
    (joint, Pt::new(root.x + ux * d, root.y + uy * d))
}

fn solve(m: &Motion, key: &Key) -> Joints {
    let root = key.hip;
    let d = dir(key.torso);
    let neck = Pt::new(root.x + d.x * TORSO, root.y + d.y * TORSO);
    let hd = dir(key.torso + key.head);
    let head = Pt::new(neck.x + hd.x * HEAD, neck.y + hd.y * HEAD);
    let mut j = Joints {
        head,
        neck,
        root,
        ..Default::default()
    };
    let (mut hands, mut feet, mut elbow_bend, mut knee_bend) =
        (key.hands, key.feet, m.elbow, m.knee);
    match m.view {
        View::Side => {
            j.shoulder = [neck; 2];
            j.hip = [root; 2];
        }
        View::Front => {
            j.shoulder = [Pt::new(neck.x + 8.0, neck.y), Pt::new(neck.x - 8.0, neck.y)];
            j.hip = [Pt::new(root.x + 5.0, root.y), Pt::new(root.x - 5.0, root.y)];
            hands[1] = Pt::new(2.0 * root.x - hands[0].x, hands[0].y);
            feet[1] = Pt::new(2.0 * root.x - feet[0].x, feet[0].y);
            elbow_bend[1] = -elbow_bend[0];
            knee_bend[1] = -knee_bend[0];
        }
    }
    for i in 0..2 {
        let (e, w) = two_bone(j.shoulder[i], hands[i], UPPER_ARM, FOREARM, elbow_bend[i]);
        j.elbow[i] = e;
        j.wrist[i] = w;
        let (kn, an) = two_bone(j.hip[i], feet[i], THIGH, SHIN, knee_bend[i]);
        j.knee[i] = kn;
        j.ankle[i] = an;
    }
    j
}

/// Joints at an eased phase between the start (0) and end (1) keyframes.
pub fn at_phase(m: &Motion, phase: f32) -> Joints {
    let key = m.a.lerp(&m.b, phase.clamp(0.0, 1.0));
    solve(m, &key)
}

/// Phase (0..1, eased) at time `t`: start → end → start every `period` seconds.
pub fn phase_at(m: &Motion, t: f32) -> f32 {
    let c = (t / m.period).rem_euclid(1.0);
    let raw = if c < 0.5 { c * 2.0 } else { 2.0 - c * 2.0 };
    raw * raw * (3.0 - 2.0 * raw)
}

pub fn sample(m: &Motion, t: f32) -> Joints {
    at_phase(m, phase_at(m, t))
}

fn stand() -> Key {
    k((0.0, -43.0), 0.0, (2.0, -47.0), (0.0, 0.0))
}

fn quadruped() -> Key {
    k((-16.0, -24.0), 86.0, (16.0, 0.0), (-38.0, 0.0))
}

fn supine_floor() -> Key {
    k((-16.0, -5.0), 90.0, (-10.0, -2.0), (-36.0, 0.0))
}

fn supine_bench() -> Key {
    k((-22.0, -22.0), 90.0, (12.0, -50.0), (-40.0, 0.0))
}

fn prone_bench() -> Key {
    k((-20.0, -24.0), 90.0, (13.0, -1.0), (-44.0, 0.0))
}

/// The animation for an exercise or warm-up/stretch id. Every catalogue id has its own; unknown ids get
/// a gentle standing breath.
pub fn motion_for(id: &str) -> Motion {
    match id {
        // ---- pushes ----
        "incline_pushup" | "close_incline_pushup" => Motion::new(
            k((-8.0, -24.0), 62.0, (36.0, -18.0), (-48.0, 0.0)),
            k((-6.0, -18.0), 72.0, (36.0, -18.0), (-48.0, 0.0)),
        )
        .bench(36.0, 30.0),
        "wall_pushups" => Motion::new(
            k((-6.0, -42.0), 16.0, (34.0, -70.0), (-12.0, 0.0)),
            k((0.0, -41.0), 26.0, (34.0, -70.0), (-12.0, 0.0)),
        )
        .wall(36.0),
        "db_bench_press" | "db_bench_1_5_pause" => Motion::new(
            supine_bench(),
            supine_bench().hand0(12.0, -30.0).hand1(12.0, -30.0),
        )
        .bench(-10.0, 60.0)
        .both()
        .elbows(-1.0, -1.0)
        .knees(1.0, 1.0),
        "paused_floor_press" => Motion::new(
            k((-22.0, -5.0), 90.0, (12.0, -33.0), (-40.0, 0.0)),
            k((-22.0, -5.0), 90.0, (12.0, -14.0), (-40.0, 0.0)),
        )
        .both()
        .elbows(-1.0, -1.0)
        .knees(1.0, 1.0)
        .period(4.0),
        "seated_shoulder_press" => Motion::new(
            k((-6.0, -24.0), 0.0, (-2.0, -62.0), (16.0, 0.0)),
            k((-6.0, -24.0), 0.0, (-3.0, -84.0), (16.0, 0.0)),
        )
        .bench(-4.0, 30.0)
        .wall(-18.0)
        .both(),
        "single_arm_shoulder_press" => Motion::new(
            k((-6.0, -24.0), 0.0, (-2.0, -62.0), (16.0, 0.0)).hand1(2.0, -36.0),
            k((-6.0, -24.0), 0.0, (-3.0, -84.0), (16.0, 0.0)).hand1(2.0, -36.0),
        )
        .bench(-4.0, 30.0)
        .wall(-18.0)
        .one(),
        "lying_triceps_ext" => Motion::new(
            supine_bench(),
            supine_bench().hand0(24.0, -34.0).hand1(24.0, -34.0),
        )
        .bench(-10.0, 60.0)
        .both()
        .elbows(-1.0, -1.0)
        .knees(1.0, 1.0),
        // ---- pulls ----
        "one_arm_row" => {
            let base = k((-18.0, -36.0), 80.0, (14.0, -14.0), (-6.0, 0.0))
                .hand1(16.0, -18.0)
                .foot1(-38.0, -18.0);
            Motion::new(base, base.hand0(4.0, -40.0))
                .bench(-8.0, 52.0)
                .one()
                .knees(-1.0, -1.0)
        }
        "chest_supported_row" | "chest_supported_row_pause" => Motion::new(
            prone_bench(),
            prone_bench().hand0(0.0, -30.0).hand1(0.0, -30.0),
        )
        .bench(-12.0, 56.0)
        .both(),
        "reverse_fly" => Motion::new(
            prone_bench(),
            prone_bench().hand0(14.0, -22.0).hand1(14.0, -22.0),
        )
        .bench(-12.0, 56.0)
        .both()
        .elbows(-1.0, -1.0),
        "bench_pullover" => Motion::new(
            supine_bench(),
            supine_bench().hand0(40.0, -22.0).hand1(40.0, -22.0),
        )
        .bench(-10.0, 60.0)
        .both()
        .knees(1.0, 1.0)
        .period(4.0),
        // ---- arms ----
        "lateral_raise" => Motion::new(stand().hand0(10.0, -48.0), stand().hand0(36.0, -73.0))
            .front()
            .both(),
        "biceps_curl" | "hammer_curl" => {
            Motion::new(stand().hand0(3.0, -47.0).hand1(3.0, -47.0), {
                let s = stand();
                s.hand0(10.0, -70.0).hand1(10.0, -70.0)
            })
            .both()
        }
        "arm_circles" => Motion::new(stand().hand0(12.0, -48.0), stand().hand0(10.0, -104.0))
            .front()
            .period(2.4),
        "triceps_stretch" => {
            Motion::new(stand().hand0(-4.0, -80.0), stand().hand0(-7.0, -74.0)).period(5.0)
        }
        "pec_wall" => Motion::new(stand().hand1(-24.0, -74.0), {
            let mut s = stand().hand1(-24.0, -74.0);
            s.hip = Pt::new(3.0, -43.0);
            s
        })
        .wall(-26.0)
        .period(5.0),
        // ---- squats ----
        "goblet_box_squat" | "goblet_1_5" => Motion::new(
            k((0.0, -43.0), 0.0, (8.0, -66.0), (4.0, 0.0)),
            k((-18.0, -24.0), 25.0, (6.0, -48.0), (4.0, 0.0)),
        )
        .bench(-24.0, 26.0)
        .one(),
        "box_squat_bw" => Motion::new(
            k((0.0, -43.0), 0.0, (26.0, -72.0), (4.0, 0.0)),
            k((-18.0, -24.0), 25.0, (24.0, -52.0), (4.0, 0.0)),
        )
        .bench(-24.0, 26.0),
        "high_box_squat_bw" => Motion::new(
            k((0.0, -43.0), 0.0, (26.0, -72.0), (4.0, 0.0)),
            k((-14.0, -31.0), 20.0, (24.0, -58.0), (4.0, 0.0)),
        )
        .block(-22.0, 26.0, 26.0),
        "supported_split_squat" => Motion::new(
            k((0.0, -42.0), 0.0, (26.0, -62.0), (14.0, 0.0))
                .hand1(2.0, -46.0)
                .foot1(-18.0, 0.0),
            k((-2.0, -25.0), 0.0, (26.0, -48.0), (14.0, 0.0))
                .hand1(0.0, -28.0)
                .foot1(-22.0, 0.0),
        )
        .wall(30.0)
        .both(),
        "bulgarian_split_squat" => Motion::new(
            k((0.0, -42.0), 0.0, (2.0, -46.0), (14.0, 0.0)).foot1(-36.0, -18.0),
            k((-2.0, -26.0), 0.0, (0.0, -29.0), (14.0, 0.0)).foot1(-36.0, -18.0),
        )
        .bench(-40.0, 24.0)
        .both(),
        "wall_sit" => Motion::new(
            k((-12.0, -24.0), 0.0, (0.0, -62.0), (12.0, 0.0)),
            k((-12.0, -23.0), 0.0, (0.0, -61.0), (12.0, 0.0)),
        )
        .wall(-18.0)
        .elbows(-1.0, -1.0)
        .period(5.0),
        "hip_flexor" | "hb_hip_flexor" => {
            let a = k((-6.0, -24.0), 0.0, (6.0, -30.0), (18.0, 0.0)).foot1(-30.0, 0.0);
            let mut b = a.hand0(8.0, -28.0).hand1(8.0, -28.0);
            b.hip = Pt::new(0.0, -22.0);
            b.torso = -5.0;
            Motion::new(a, b).knees(-1.0, -1.0).period(5.0)
        }
        // ---- hinges ----
        "romanian_deadlift" | "rdl_tempo4" => Motion::new(
            k((0.0, -43.0), 0.0, (4.0, -48.0), (2.0, 0.0)),
            k((-14.0, -41.0), 72.0, (14.0, -8.0), (2.0, 0.0)),
        )
        .both(),
        "hinge_drill" => Motion::new(
            k((0.0, -43.0), 0.0, (4.0, -48.0), (2.0, 0.0)),
            k((-14.0, -41.0), 70.0, (16.0, -10.0), (2.0, 0.0)),
        ),
        "single_leg_rdl" => Motion::new(
            k((0.0, -43.0), 0.0, (4.0, -48.0), (4.0, 0.0)).foot1(-18.0, -1.0),
            k((-12.0, -40.0), 70.0, (16.0, -8.0), (4.0, 0.0)).foot1(-28.0, -1.0),
        )
        .both(),
        "hip_thrust" => Motion::new(
            k((-8.0, -10.0), 293.0, (-6.0, -14.0), (24.0, 0.0)),
            k((-4.0, -28.0), 256.0, (-2.0, -30.0), (24.0, 0.0)),
        )
        .bench(-44.0, 22.0)
        .one()
        .knees(-1.0, -1.0),
        "single_leg_hip_thrust" => Motion::new(
            k((-8.0, -10.0), 293.0, (-6.0, -14.0), (24.0, 0.0)).foot1(18.0, -26.0),
            k((-4.0, -28.0), 256.0, (-2.0, -30.0), (24.0, 0.0)).foot1(36.0, -34.0),
        )
        .bench(-44.0, 22.0)
        .one()
        .knees(-1.0, -1.0),
        "glute_bridge" | "warm_glute_bridge" => Motion::new(
            k((-6.0, -5.0), 268.0, (-30.0, 0.0), (20.0, 0.0)),
            k((-4.0, -20.0), 246.0, (-30.0, 0.0), (20.0, 0.0)),
        ),
        "single_leg_glute_bridge" => Motion::new(
            k((-6.0, -5.0), 268.0, (-30.0, 0.0), (20.0, 0.0)).foot1(30.0, -16.0),
            k((-4.0, -20.0), 246.0, (-30.0, 0.0), (20.0, 0.0)).foot1(36.0, -28.0),
        ),
        // ---- core (McGill) ----
        "dead_bug" => Motion::new(
            k((-16.0, -6.0), 90.0, (16.0, -34.0), (-30.0, -26.0)),
            k((-16.0, -6.0), 90.0, (16.0, -34.0), (-30.0, -26.0))
                .hand0(44.0, -10.0)
                .foot1(-58.0, -6.0),
        )
        .knees(1.0, 1.0),
        "bird_dog" | "warm_bird_dog" | "hb_bird_dog" => Motion::new(
            quadruped(),
            quadruped().hand0(46.0, -26.0).foot1(-60.0, -26.0),
        )
        .knees(-1.0, -1.0)
        .period(4.0),
        "cat_camel" | "hb_cat_camel" => Motion::new(
            {
                let mut q = quadruped().head(-25.0);
                q.torso = 80.0;
                q
            },
            {
                let mut q = quadruped().head(30.0);
                q.torso = 96.0;
                q
            },
        )
        .knees(-1.0, -1.0)
        .mobility(),
        "mcgill_curl_up" | "warm_curl_up" | "hb_curl_up" => {
            Motion::new(supine_floor().foot1(-58.0, -4.0), {
                let mut s = supine_floor().foot1(-58.0, -4.0);
                s.torso = 84.0;
                s
            })
            .knees(1.0, 1.0)
            .period(4.0)
        }
        "side_plank_knees" | "warm_side_plank" | "hb_side_plank" => Motion::new(
            k((-16.0, -5.0), 72.0, (24.0, 0.0), (-56.0, -1.0)).hand1(-8.0, -18.0),
            k((-15.0, -13.0), 78.0, (24.0, 0.0), (-56.0, -1.0)).hand1(-6.0, -24.0),
        )
        .elbows(-1.0, 1.0)
        .period(4.0),
        // ---- lower leg / locomotion ----
        "calf_raise" => Motion::new(
            k((0.0, -43.0), 0.0, (29.0, -70.0), (2.0, 0.0)),
            k((0.0, -49.0), 0.0, (29.0, -75.0), (2.0, -6.0)),
        )
        .wall(31.0)
        .period(2.6),
        "march" | "hb_march" => Motion::new(
            stand().hand0(-8.0, -50.0).hand1(10.0, -50.0),
            stand()
                .hand0(10.0, -50.0)
                .hand1(-8.0, -50.0)
                .foot0(10.0, -18.0),
        )
        .period(1.4),
        "leg_swings" => Motion::new(
            k((0.0, -43.0), 0.0, (22.0, -70.0), (22.0, -18.0)).foot1(0.0, 0.0),
            k((0.0, -43.0), 0.0, (22.0, -70.0), (-20.0, -16.0)).foot1(0.0, 0.0),
        )
        .wall(24.0)
        .period(2.0),
        "calf_wall" => Motion::new(
            k((-4.0, -42.0), 12.0, (30.0, -70.0), (12.0, 0.0)).foot1(-26.0, 0.0),
            k((2.0, -40.0), 16.0, (30.0, -70.0), (12.0, 0.0)).foot1(-26.0, 0.0),
        )
        .wall(32.0)
        .period(5.0),
        // ---- floor stretches ----
        "lat_bench" => Motion::new(
            k((-14.0, -22.0), 70.0, (34.0, -24.0), (-36.0, 0.0)),
            k((-22.0, -20.0), 84.0, (34.0, -24.0), (-40.0, 0.0)),
        )
        .bench(34.0, 24.0)
        .knees(-1.0, -1.0)
        .period(5.0),
        "hamstring_lying" => Motion::new(
            supine_floor()
                .foot0(-12.0, -46.0)
                .foot1(-58.0, -4.0)
                .hand0(-10.0, -36.0)
                .hand1(-10.0, -36.0),
            supine_floor()
                .foot0(-5.0, -46.0)
                .foot1(-58.0, -4.0)
                .hand0(-6.0, -36.0)
                .hand1(-6.0, -36.0),
        )
        .knees(1.0, 1.0)
        .period(5.0),
        "figure_four" | "hb_figure_four" => Motion::new(
            supine_floor()
                .foot0(-36.0, 0.0)
                .foot1(-26.0, -20.0)
                .hand0(-22.0, -22.0),
            supine_floor()
                .foot0(-30.0, -8.0)
                .foot1(-22.0, -26.0)
                .hand0(-20.0, -26.0),
        )
        .knees(1.0, 1.0)
        .period(5.0),
        "breath_46" | "breath_60" | "hb_breath" => Motion::new(supine_floor(), {
            let mut s = supine_floor();
            s.torso = 88.0;
            s
        })
        .knees(1.0, 1.0)
        .period(10.0),
        _ => Motion::new(stand(), stand()).period(6.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{
        EXERCISES, HAPPY_BACK, STRETCH_LOWER, STRETCH_UPPER, WARMUP_BASE, WARMUP_LOWER,
        WARMUP_UPPER,
    };

    #[test]
    fn ik_keeps_bone_lengths() {
        let (j, e) = two_bone(Pt::new(0.0, 0.0), Pt::new(10.0, 20.0), 15.0, 14.0, 1.0);
        assert!((Pt::new(0.0, 0.0).dist(j) - 15.0).abs() < 0.01);
        assert!((j.dist(e) - 14.0).abs() < 0.01);
        // Unreachable target: straight limb pointing at it.
        let (_, e) = two_bone(Pt::new(0.0, 0.0), Pt::new(100.0, 0.0), 15.0, 14.0, 1.0);
        assert!((e.x - 29.0).abs() < 0.05);
    }

    #[test]
    fn every_exercise_and_routine_item_has_its_own_animation() {
        let default = motion_for("__none__");
        let ids = EXERCISES.iter().map(|e| e.id).chain(
            [
                WARMUP_BASE,
                WARMUP_UPPER,
                WARMUP_LOWER,
                STRETCH_UPPER,
                STRETCH_LOWER,
                HAPPY_BACK,
            ]
            .into_iter()
            .flat_map(|l| l.iter().map(|i| i.id)),
        );
        for id in ids {
            let m = motion_for(id);
            assert!(
                m.a != default.a || m.b != default.b,
                "{id} uses the fallback animation"
            );
            assert!(m.a != m.b, "{id} doesn't move");
            for t in [0.0, 0.8, 1.6, 2.4] {
                let j = sample(&m, t);
                for p in [j.head, j.wrist[0], j.ankle[0], j.knee[1]] {
                    assert!(p.x.is_finite() && p.y.is_finite(), "{id}");
                    assert!(p.y < 12.0, "{id}: a joint sinks below the floor");
                }
            }
        }
    }
}
