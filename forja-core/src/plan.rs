//! Weekly template, 18-week cycles (Phase 1 → deload → Phase 2 → deload) and session planning.

use crate::catalog::{
    exercise, Exercise, Measure, RoutineItem, HAPPY_BACK, STRETCH_LOWER, STRETCH_UPPER,
    WARMUP_BASE, WARMUP_LOWER, WARMUP_UPPER,
};
use crate::model::SessionKind;
use crate::profile::{LoadMode, Profile};
use crate::progression::{state_for, ExerciseState, TechniqueMod};
use crate::safety::{substitute, PainLight, SafetyStatus};
use crate::time::{days_between, week_monday};
use crate::units::Grams;
use chrono::{Datelike, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestKind {
    /// Walk 30–45 min + 10 min mobility.
    WalkMobility,
    /// Walk / free activity.
    FreeActivity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayKind {
    Training(SessionKind),
    ActiveRest(RestKind),
    /// Fasted weigh-in, weekly review, meal prep.
    Review,
}

impl DayKind {
    pub fn is_training(self) -> bool {
        matches!(self, DayKind::Training(_))
    }

    pub fn title(self) -> &'static str {
        match self {
            DayKind::Training(k) => k.name(),
            DayKind::ActiveRest(RestKind::WalkMobility) => "Caminar 30–45 min + movilidad 10 min",
            DayKind::ActiveRest(RestKind::FreeActivity) => "Caminar / actividad libre",
            DayKind::Review => "Revisión",
        }
    }

    pub fn focus(self) -> &'static str {
        match self {
            DayKind::Training(k) => k.focus(),
            DayKind::ActiveRest(_) => "Descanso activo",
            DayKind::Review => "Pesaje en ayunas, revisión semanal, preparar comidas",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekTemplate {
    /// Monday first.
    pub days: [DayKind; 7],
}

impl Default for WeekTemplate {
    fn default() -> Self {
        use DayKind::*;
        WeekTemplate {
            days: [
                Training(SessionKind::UpperA),
                Training(SessionKind::LowerA),
                ActiveRest(RestKind::WalkMobility),
                Training(SessionKind::UpperB),
                Training(SessionKind::LowerB),
                ActiveRest(RestKind::FreeActivity),
                Review,
            ],
        }
    }
}

impl WeekTemplate {
    /// Template from the profile's training days: sessions go Upper A, Lower A, Upper B, Lower B in order;
    /// Sunday is review (unless trained), Saturday free activity, other rest days walk + mobility.
    pub fn from_profile(profile: &Profile) -> Self {
        let mut days = profile.training_days.clone();
        days.sort_by_key(|w| w.num_days_from_monday());
        days.dedup();
        if days == [Weekday::Mon, Weekday::Tue, Weekday::Thu, Weekday::Fri] {
            return WeekTemplate::default();
        }
        let order = [
            SessionKind::UpperA,
            SessionKind::LowerA,
            SessionKind::UpperB,
            SessionKind::LowerB,
        ];
        let mut out = [DayKind::ActiveRest(RestKind::WalkMobility); 7];
        out[5] = DayKind::ActiveRest(RestKind::FreeActivity);
        out[6] = DayKind::Review;
        for (i, w) in days.iter().take(4).enumerate() {
            out[w.num_days_from_monday() as usize] = DayKind::Training(order[i]);
        }
        WeekTemplate { days: out }
    }

    pub fn day(&self, date: NaiveDate) -> DayKind {
        self.days[date.weekday().num_days_from_monday() as usize]
    }

    pub fn sessions_per_week(&self) -> u32 {
        self.days.iter().filter(|d| d.is_training()).count() as u32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Phase 1: base (weeks 1–8 of the cycle).
    Base,
    /// Deload: half the sets (weeks 9 and 18).
    Deload,
    /// Phase 2: variants (weeks 10–17).
    Variants,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Base => "Fase 1 · Base",
            Phase::Deload => "Semana de descarga",
            Phase::Variants => "Fase 2 · Variantes",
        }
    }
}

pub const CYCLE_WEEKS: u32 = 18;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseInfo {
    /// 1-based week since the program started.
    pub program_week: u32,
    /// 1-based cycle number.
    pub cycle: u32,
    /// 1..=18.
    pub week_in_cycle: u32,
    pub phase: Phase,
}

impl PhaseInfo {
    pub fn strength_test_week(&self) -> bool {
        self.phase != Phase::Deload && self.week_in_cycle.is_multiple_of(4)
    }

    pub fn label(&self) -> String {
        format!("{} · Semana {}", self.phase.label(), self.week_in_cycle)
    }
}

pub fn phase_info(program_start: NaiveDate, date: NaiveDate) -> PhaseInfo {
    let start = week_monday(program_start);
    let weeks = (days_between(start, week_monday(date)) / 7).max(0) as u32;
    let program_week = weeks + 1;
    let cycle = weeks / CYCLE_WEEKS + 1;
    let week_in_cycle = weeks % CYCLE_WEEKS + 1;
    let phase = match week_in_cycle {
        1..=8 => Phase::Base,
        9 | 18 => Phase::Deload,
        _ => Phase::Variants,
    };
    PhaseInfo {
        program_week,
        cycle,
        week_in_cycle,
        phase,
    }
}

pub fn template_exercises(kind: SessionKind) -> &'static [&'static str] {
    match kind {
        SessionKind::UpperA => &[
            "incline_pushup",
            "one_arm_row",
            "db_bench_press",
            "lateral_raise",
            "biceps_curl",
            "lying_triceps_ext",
            "dead_bug",
        ],
        SessionKind::LowerA => &[
            "goblet_box_squat",
            "romanian_deadlift",
            "hip_thrust",
            "calf_raise",
            "side_plank_knees",
        ],
        SessionKind::UpperB => &[
            "paused_floor_press",
            "chest_supported_row",
            "seated_shoulder_press",
            "bench_pullover",
            "reverse_fly",
            "hammer_curl",
            "close_incline_pushup",
        ],
        SessionKind::LowerB => &[
            "supported_split_squat",
            "single_leg_glute_bridge",
            "single_leg_rdl",
            "wall_sit",
            "bird_dog",
            "mcgill_curl_up",
        ],
    }
}

/// Phase 2 replaces these base exercises with their variants.
pub fn phase2_variant(base_id: &str) -> Option<&'static str> {
    Some(match base_id {
        "goblet_box_squat" => "goblet_1_5",
        "hip_thrust" => "single_leg_hip_thrust",
        "db_bench_press" => "db_bench_1_5_pause",
        "seated_shoulder_press" => "single_arm_shoulder_press",
        "chest_supported_row" => "chest_supported_row_pause",
        "romanian_deadlift" => "rdl_tempo4",
        "supported_split_squat" => "bulgarian_split_squat",
        _ => return None,
    })
}

/// 15-minute minimum version (counts for the streak): (exercise, sets, reps).
pub const MINIMUM_VERSION: &[(&str, u8, u16)] = &[
    ("incline_pushup", 2, 10),
    ("goblet_box_squat", 2, 10),
    ("chest_supported_row", 2, 10),
    ("glute_bridge", 2, 15),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionVariant {
    Full,
    Minimum,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PlannedExercise {
    pub exercise_id: &'static str,
    pub name: String,
    pub sets: u8,
    pub min: u16,
    pub max: u16,
    pub measure: Measure,
    pub per_side: bool,
    /// Suggested load per dumbbell (None for bodyweight).
    pub load: Option<Grams>,
    pub load_mode: LoadMode,
    pub level_name: Option<&'static str>,
    pub modifier: Option<TechniqueMod>,
    pub tempo: Option<&'static str>,
    pub technique: &'static str,
    pub back_note: &'static str,
    pub rest_s: u16,
    /// Set when this replaced another exercise because of back pain.
    pub substituted_from: Option<&'static str>,
    /// A safer swap the user can choose.
    pub alternative: Option<&'static str>,
}

impl PlannedExercise {
    pub fn dose_label(&self) -> String {
        exercise(self.exercise_id)
            .map(|e| e.dose_label(self.sets, self.min, self.max))
            .unwrap_or_default()
    }

    pub fn exercise(&self) -> &'static Exercise {
        exercise(self.exercise_id).expect("planned exercises come from the catalogue")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SessionPlan {
    pub date: NaiveDate,
    pub kind: SessionKind,
    pub variant: SessionVariant,
    pub phase: PhaseInfo,
    pub warmup: Vec<RoutineItem>,
    pub exercises: Vec<PlannedExercise>,
    pub stretches: Vec<RoutineItem>,
    pub safety_messages: Vec<String>,
}

impl SessionPlan {
    pub fn total_sets(&self) -> u32 {
        self.exercises.iter().map(|e| u32::from(e.sets)).sum()
    }

    pub fn planned_sets(&self, exercise_id: &str) -> u8 {
        self.exercises
            .iter()
            .find(|e| e.exercise_id == exercise_id)
            .map(|e| e.sets)
            .unwrap_or(0)
    }
}

pub fn warmup_for(kind: SessionKind) -> Vec<RoutineItem> {
    let extra = if kind.is_upper() {
        WARMUP_UPPER
    } else {
        WARMUP_LOWER
    };
    WARMUP_BASE.iter().chain(extra.iter()).copied().collect()
}

pub fn stretches_for(kind: SessionKind) -> Vec<RoutineItem> {
    if kind.is_upper() {
        STRETCH_UPPER
    } else {
        STRETCH_LOWER
    }
    .to_vec()
}

pub fn rest_day_routine() -> Vec<RoutineItem> {
    HAPPY_BACK.to_vec()
}

fn plan_exercise(
    ex: &'static Exercise,
    sets: u8,
    min: u16,
    max: u16,
    state: ExerciseState,
    substituted_from: Option<&'static str>,
) -> PlannedExercise {
    PlannedExercise {
        exercise_id: ex.id,
        name: ex.name.to_string(),
        sets,
        min,
        max,
        measure: ex.measure,
        per_side: ex.per_side,
        load: (ex.load != LoadMode::Bodyweight).then_some(state.load),
        load_mode: ex.load,
        level_name: state.level_name(ex),
        modifier: state.modifier(ex),
        tempo: ex.tempo,
        technique: ex.technique,
        back_note: ex.back_note,
        rest_s: ex.rest_seconds(),
        substituted_from,
        alternative: substitute(ex.id),
    }
}

/// Plan one session. Pure: everything comes in as arguments.
pub fn plan_session(
    profile: &Profile,
    states: &BTreeMap<String, ExerciseState>,
    safety: &SafetyStatus,
    date: NaiveDate,
    kind: SessionKind,
    variant: SessionVariant,
) -> SessionPlan {
    let phase = phase_info(profile.program_start, date);
    let eq = &profile.equipment;
    let mut messages = Vec::new();
    let mut out = Vec::new();

    let entries: Vec<(&'static Exercise, u8, u16, u16)> = match variant {
        SessionVariant::Minimum => MINIMUM_VERSION
            .iter()
            .filter_map(|(id, sets, reps)| exercise(id).map(|e| (e, *sets, *reps, *reps)))
            .collect(),
        SessionVariant::Full => template_exercises(kind)
            .iter()
            .filter_map(|id| {
                let id = if phase.phase == Phase::Variants {
                    phase2_variant(id).unwrap_or(id)
                } else {
                    id
                };
                exercise(id)
            })
            .map(|e| {
                let sets = if phase.phase == Phase::Deload {
                    e.sets.div_ceil(2)
                } else {
                    e.sets
                };
                (e, sets, e.min, e.max)
            })
            .collect(),
    };

    for (ex, sets, min, max) in entries {
        if safety.paused(ex.id) {
            messages.push(format!("En pausa por precaución: {}", ex.name));
            continue;
        }
        if safety.light == PainLight::Red {
            if let Some(sub) = substitute(ex.id).and_then(exercise) {
                messages.push(format!(
                    "Cambio por dolor lumbar: {} → {}",
                    ex.name, sub.name
                ));
                let s = state_for(sub, states, eq);
                out.push(plan_exercise(sub, sets, sub.min, sub.max, s, Some(ex.id)));
                continue;
            }
        }
        let state = state_for(ex, states, eq);
        out.push(plan_exercise(ex, sets, min, max, state, None));
    }
    if safety.light == PainLight::Amber {
        messages.push(
            "El dolor subió respecto de la última sesión: tienes alternativas más seguras a mano."
                .into(),
        );
    }
    if phase.phase == Phase::Deload && variant == SessionVariant::Full {
        messages
            .push("Semana de descarga: la mitad de las series, misma técnica impecable.".into());
    }

    SessionPlan {
        date,
        kind,
        variant,
        phase,
        warmup: warmup_for(kind),
        exercises: out,
        stretches: stretches_for(kind),
        safety_messages: messages,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn default_week_matches_brief() {
        let t = WeekTemplate::default();
        assert_eq!(
            t.day(d(2026, 11, 23)),
            DayKind::Training(SessionKind::UpperA)
        );
        assert_eq!(
            t.day(d(2026, 11, 24)),
            DayKind::Training(SessionKind::LowerA)
        );
        assert_eq!(
            t.day(d(2026, 11, 25)),
            DayKind::ActiveRest(RestKind::WalkMobility)
        );
        assert_eq!(
            t.day(d(2026, 11, 26)),
            DayKind::Training(SessionKind::UpperB)
        );
        assert_eq!(
            t.day(d(2026, 11, 27)),
            DayKind::Training(SessionKind::LowerB)
        );
        assert_eq!(
            t.day(d(2026, 11, 28)),
            DayKind::ActiveRest(RestKind::FreeActivity)
        );
        assert_eq!(t.day(d(2026, 11, 29)), DayKind::Review);
        assert_eq!(t.sessions_per_week(), 4);
        let p = Profile::initial(d(2026, 11, 27));
        assert_eq!(WeekTemplate::from_profile(&p), t);
    }

    #[test]
    fn cycle_phases() {
        let start = d(2026, 11, 23);
        let week = |n: i64| phase_info(start, start + chrono::Duration::days(7 * (n - 1)));
        assert_eq!(week(1).phase, Phase::Base);
        assert_eq!(week(8).phase, Phase::Base);
        assert_eq!(week(9).phase, Phase::Deload);
        assert_eq!(week(10).phase, Phase::Variants);
        assert_eq!(week(17).phase, Phase::Variants);
        assert_eq!(week(18).phase, Phase::Deload);
        assert_eq!(week(19).phase, Phase::Base);
        assert_eq!(week(19).cycle, 2);
        assert!(week(4).strength_test_week());
        assert!(week(12).strength_test_week());
        assert!(!week(9).strength_test_week());
        assert!(!week(5).strength_test_week());
    }

    #[test]
    fn deload_halves_sets_and_phase2_uses_variants() {
        let p = Profile::initial(d(2026, 11, 23));
        let states = BTreeMap::new();
        let safe = SafetyStatus::clear();
        let w1 = plan_session(
            &p,
            &states,
            &safe,
            d(2026, 11, 24),
            SessionKind::LowerA,
            SessionVariant::Full,
        );
        let w9 = plan_session(
            &p,
            &states,
            &safe,
            d(2027, 1, 19),
            SessionKind::LowerA,
            SessionVariant::Full,
        );
        assert_eq!(w9.phase.phase, Phase::Deload);
        assert_eq!(w1.planned_sets("goblet_box_squat"), 3);
        assert_eq!(w9.planned_sets("goblet_box_squat"), 2);
        let w10 = plan_session(
            &p,
            &states,
            &safe,
            d(2027, 1, 26),
            SessionKind::LowerA,
            SessionVariant::Full,
        );
        assert_eq!(w10.phase.phase, Phase::Variants);
        assert!(w10.exercises.iter().any(|e| e.exercise_id == "goblet_1_5"));
        assert!(w10.exercises.iter().any(|e| e.exercise_id == "rdl_tempo4"));
    }

    #[test]
    fn red_pain_swaps_risky_exercises() {
        let p = Profile::initial(d(2026, 11, 23));
        let mut safety = SafetyStatus::clear();
        safety.light = PainLight::Red;
        let plan = plan_session(
            &p,
            &BTreeMap::new(),
            &safety,
            d(2026, 11, 24),
            SessionKind::LowerA,
            SessionVariant::Full,
        );
        let ids: Vec<_> = plan.exercises.iter().map(|e| e.exercise_id).collect();
        assert!(!ids.contains(&"romanian_deadlift"));
        assert!(!ids.contains(&"goblet_box_squat"));
        assert!(ids.contains(&"high_box_squat_bw"));
        let swapped = plan
            .exercises
            .iter()
            .find(|e| e.exercise_id == "hip_thrust" && e.substituted_from.is_some());
        assert!(swapped.is_some());
    }

    #[test]
    fn minimum_version() {
        let p = Profile::initial(d(2026, 11, 23));
        let plan = plan_session(
            &p,
            &BTreeMap::new(),
            &SafetyStatus::clear(),
            d(2026, 11, 23),
            SessionKind::UpperA,
            SessionVariant::Minimum,
        );
        assert_eq!(plan.exercises.len(), 4);
        assert_eq!(plan.total_sets(), 8);
        assert_eq!(plan.exercises[3].min, 15);
    }
}
