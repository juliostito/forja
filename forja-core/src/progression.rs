//! Double progression with loads the home equipment can actually build.
//!
//! Rule: when every set of an exercise reaches the top of its rep range with good technique, the load
//! goes up by ~1.25 kg (the buildable load closest to +1.25 kg). Once the equipment tops out, the
//! exercise climbs a technique ladder instead: 4 s tempo → 2 s pauses → 1½ reps → unilateral.

use crate::catalog::{exercise, Exercise, Measure, Pattern, Progression, PUSHUP_START_LEVEL};
use crate::profile::{Equipment, LoadMode};
use crate::units::Grams;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LOAD_STEP: Grams = Grams(1250);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TechniqueMod {
    Tempo4,
    Pause2,
    OneAndHalf,
    Unilateral,
}

impl TechniqueMod {
    pub fn label(self) -> &'static str {
        match self {
            TechniqueMod::Tempo4 => "Tempo de 4 s al bajar",
            TechniqueMod::Pause2 => "Pausa de 2 s en el punto difícil",
            TechniqueMod::OneAndHalf => "Reps 1½",
            TechniqueMod::Unilateral => "Versión a un brazo / una pierna",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExerciseState {
    /// Load per dumbbell (zero for bodyweight).
    pub load: Grams,
    /// Bodyweight level (index into the exercise's level list).
    pub level: u8,
    /// 0 = no technique modifier; n = n-th rung of the technique ladder.
    pub technique_step: u8,
}

impl ExerciseState {
    pub fn modifier(&self, ex: &Exercise) -> Option<TechniqueMod> {
        if self.technique_step == 0 {
            return None;
        }
        technique_ladder(ex)
            .get(self.technique_step as usize - 1)
            .copied()
    }

    pub fn level_name(&self, ex: &Exercise) -> Option<&'static str> {
        match ex.progression {
            Progression::Levels(levels) => levels.get(self.level as usize).copied(),
            _ => None,
        }
    }
}

pub fn technique_ladder(ex: &Exercise) -> Vec<TechniqueMod> {
    use TechniqueMod::*;
    match ex.progression {
        Progression::Load => {
            let mut v = vec![Tempo4, Pause2, OneAndHalf];
            if ex.unilateral_option {
                v.push(Unilateral);
            }
            v
        }
        Progression::Levels(_) => vec![Tempo4, Pause2],
        Progression::Hold => {
            if ex.measure == Measure::Reps && ex.pattern != Pattern::Core {
                vec![Pause2, Tempo4]
            } else {
                Vec::new()
            }
        }
    }
}

pub fn initial_state(ex: &Exercise, equipment: &Equipment) -> ExerciseState {
    ExerciseState {
        load: match ex.load {
            LoadMode::Bodyweight => Grams::ZERO,
            mode => equipment.snap(mode, Grams(ex.initial_g)),
        },
        level: match ex.progression {
            Progression::Levels(_) => PUSHUP_START_LEVEL,
            _ => 0,
        },
        technique_step: 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetResult {
    pub reps: u16,
    pub load: Grams,
    pub ok: bool,
}

/// Every logged set is good, at the top of the range, and there are at least `required_sets`.
pub fn all_sets_at_top(ex: &Exercise, required_sets: u8, sets: &[SetResult]) -> bool {
    required_sets > 0
        && sets.len() >= required_sets as usize
        && sets.iter().all(|s| s.ok && s.reps >= ex.max)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressEvent {
    LoadUp {
        from: Grams,
        to: Grams,
    },
    LevelUp {
        from: u8,
        to: u8,
    },
    Technique(TechniqueMod),
    /// Already at the top of everything: keep it clean.
    Ceiling,
}

/// One session's worth of results for an exercise → the next prescription.
pub fn advance(
    ex: &Exercise,
    state: ExerciseState,
    equipment: &Equipment,
    sets: &[SetResult],
    required_sets: u8,
) -> (ExerciseState, Option<ProgressEvent>) {
    let mut s = state;
    if ex.load != LoadMode::Bodyweight {
        // Adopt the load actually used (the lightest good set), snapped to what can be built.
        if let Some(used) = sets
            .iter()
            .filter(|x| x.ok && x.load > Grams::ZERO)
            .map(|x| x.load)
            .min()
        {
            let snapped = equipment.snap(ex.load, used);
            if snapped != s.load {
                s.load = snapped;
                if snapped < equipment.max(ex.load) {
                    s.technique_step = 0;
                }
            }
        }
    }
    if !all_sets_at_top(ex, required_sets, sets) {
        return (s, None);
    }
    let ladder = technique_ladder(ex);
    let technique_up = |mut s: ExerciseState| {
        if (s.technique_step as usize) < ladder.len() {
            s.technique_step += 1;
            let m = ladder[s.technique_step as usize - 1];
            (s, Some(ProgressEvent::Technique(m)))
        } else {
            (s, Some(ProgressEvent::Ceiling))
        }
    };
    match ex.progression {
        Progression::Load => match equipment.next_step(ex.load, s.load, LOAD_STEP) {
            Some(next) => {
                let from = s.load;
                s.load = next;
                s.technique_step = 0;
                (s, Some(ProgressEvent::LoadUp { from, to: next }))
            }
            None => technique_up(s),
        },
        Progression::Levels(levels) => {
            if (s.level as usize) + 1 < levels.len() {
                let from = s.level;
                s.level += 1;
                s.technique_step = 0;
                (s, Some(ProgressEvent::LevelUp { from, to: s.level }))
            } else {
                technique_up(s)
            }
        }
        Progression::Hold => technique_up(s),
    }
}

/// Current state of an exercise, inheriting from its base exercise for Phase 2 variants.
pub fn state_for(
    ex: &Exercise,
    states: &BTreeMap<String, ExerciseState>,
    equipment: &Equipment,
) -> ExerciseState {
    if let Some(s) = states.get(ex.id) {
        return *s;
    }
    if let Some(base) = ex.variant_of.and_then(|b| states.get(b)) {
        return ExerciseState {
            load: match ex.load {
                LoadMode::Bodyweight => Grams::ZERO,
                mode => equipment.snap(mode, base.load),
            },
            level: base.level,
            technique_step: 0,
        };
    }
    initial_state(ex, equipment)
}

/// One past session's sets, in order.
#[derive(Clone, Debug, Default)]
pub struct SessionSets {
    /// Full sessions outside deload weeks drive progression; minimum versions and deloads don't.
    pub drives_progression: bool,
    pub sets: Vec<(String, SetResult)>,
}

#[derive(Clone, Debug, Default)]
pub struct Replay {
    pub states: BTreeMap<String, ExerciseState>,
    pub events: Vec<(String, ProgressEvent)>,
}

/// Replays history (chronological) into current prescriptions. Deterministic.
pub fn replay<'a>(
    equipment: &Equipment,
    sessions: impl IntoIterator<Item = &'a SessionSets>,
) -> Replay {
    let mut out = Replay::default();
    for session in sessions {
        if !session.drives_progression {
            continue;
        }
        let mut grouped: BTreeMap<&str, Vec<SetResult>> = BTreeMap::new();
        let mut order: Vec<&str> = Vec::new();
        for (id, r) in &session.sets {
            if !grouped.contains_key(id.as_str()) {
                order.push(id.as_str());
            }
            grouped.entry(id.as_str()).or_default().push(*r);
        }
        for id in order {
            let Some(ex) = exercise(id) else { continue };
            let state = state_for(ex, &out.states, equipment);
            let (next, event) = advance(ex, state, equipment, &grouped[id], ex.sets);
            out.states.insert(id.to_string(), next);
            if let Some(e) = event {
                out.events.push((id.to_string(), e));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn top(ex: &Exercise, load: Grams, n: usize) -> Vec<SetResult> {
        vec![
            SetResult {
                reps: ex.max,
                load,
                ok: true
            };
            n
        ]
    }

    #[test]
    fn load_goes_up_only_when_all_sets_hit_the_top() {
        let eq = Equipment::initial();
        let ex = exercise("db_bench_press").unwrap();
        let s0 = initial_state(ex, &eq);
        assert_eq!(s0.load, Grams(6750));
        let mut sets = top(ex, s0.load, 3);
        sets[2].reps = ex.max - 1;
        let (s1, e1) = advance(ex, s0, &eq, &sets, 3);
        assert_eq!(e1, None);
        assert_eq!(s1.load, s0.load);
        let (s2, e2) = advance(ex, s1, &eq, &top(ex, s1.load, 3), 3);
        assert_eq!(
            e2,
            Some(ProgressEvent::LoadUp {
                from: Grams(6750),
                to: Grams(8000)
            })
        );
        assert_eq!(s2.load, Grams(8000));
        // Two sets aren't enough when three are planned.
        let (_, e3) = advance(ex, s2, &eq, &top(ex, s2.load, 2), 3);
        assert_eq!(e3, None);
        // A bad-technique set blocks progression.
        let mut bad = top(ex, s2.load, 3);
        bad[0].ok = false;
        assert_eq!(advance(ex, s2, &eq, &bad, 3).1, None);
    }

    #[test]
    fn at_equipment_ceiling_switch_to_technique_ladder() {
        let eq = Equipment::initial();
        let ex = exercise("db_bench_press").unwrap();
        let mut s = ExerciseState {
            load: eq.max(LoadMode::Pair),
            level: 0,
            technique_step: 0,
        };
        let mut seen = Vec::new();
        for _ in 0..6 {
            let (n, e) = advance(ex, s, &eq, &top(ex, s.load, 3), 3);
            s = n;
            seen.push(e.unwrap());
        }
        assert_eq!(
            seen,
            vec![
                ProgressEvent::Technique(TechniqueMod::Tempo4),
                ProgressEvent::Technique(TechniqueMod::Pause2),
                ProgressEvent::Technique(TechniqueMod::OneAndHalf),
                ProgressEvent::Technique(TechniqueMod::Unilateral),
                ProgressEvent::Ceiling,
                ProgressEvent::Ceiling,
            ]
        );
        assert_eq!(s.load, eq.max(LoadMode::Pair));
    }

    #[test]
    fn pushups_climb_levels() {
        let eq = Equipment::initial();
        let ex = exercise("incline_pushup").unwrap();
        let s0 = initial_state(ex, &eq);
        assert_eq!(s0.level_name(ex), Some("Flexiones inclinadas en banca"));
        let (s1, e) = advance(ex, s0, &eq, &top(ex, Grams::ZERO, 3), 3);
        assert_eq!(e, Some(ProgressEvent::LevelUp { from: 1, to: 2 }));
        assert_eq!(s1.level_name(ex), Some("Flexiones inclinadas en escalón"));
    }

    #[test]
    fn variants_inherit_base_load() {
        let eq = Equipment::initial();
        let mut states = BTreeMap::new();
        states.insert(
            "seated_shoulder_press".to_string(),
            ExerciseState {
                load: Grams(6750),
                level: 0,
                technique_step: 2,
            },
        );
        let v = exercise("single_arm_shoulder_press").unwrap();
        let s = state_for(v, &states, &eq);
        assert_eq!(s.load, eq.snap(LoadMode::Single, Grams(6750)));
        assert_eq!(s.technique_step, 0);
    }

    #[test]
    fn replay_skips_sessions_that_dont_drive_progression() {
        let eq = Equipment::initial();
        let ex = exercise("goblet_box_squat").unwrap();
        let sets: Vec<(String, SetResult)> = top(ex, Grams(9000), 3)
            .into_iter()
            .map(|r| (ex.id.to_string(), r))
            .collect();
        let deload = SessionSets {
            drives_progression: false,
            sets: sets.clone(),
        };
        let full = SessionSets {
            drives_progression: true,
            sets,
        };
        let r = replay(&eq, [&deload]);
        assert!(r.states.is_empty());
        let r = replay(&eq, [&full]);
        assert_eq!(r.states["goblet_box_squat"].load, Grams(10250));
    }
}
