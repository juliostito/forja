//! Property tests for streaks, heat and progression.

use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};
use forja_core::catalog::exercise;
use forja_core::model::{SessionKind, SessionLog, SessionType, Source};
use forja_core::plan::WeekTemplate;
use forja_core::profile::{Equipment, LoadMode};
use forja_core::progression::{advance, all_sets_at_top, initial_state, SetResult};
use forja_core::streak::{can_recover, evaluate};
use forja_core::time::{add_days, resolve_local};
use forja_core::units::Grams;
use proptest::prelude::*;
use std::collections::BTreeSet;

fn start() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 30).unwrap()
}

fn session(id: i64, date: NaiveDate, ty: SessionType, from: Option<NaiveDate>) -> SessionLog {
    let s = resolve_local(date, NaiveTime::from_hms_opt(7, 0, 0).unwrap());
    SessionLog {
        id,
        date,
        kind: SessionKind::UpperA,
        session_type: ty,
        recovered_from: from,
        started_at: s,
        ended_at: Some(s + chrono::Duration::minutes(40)),
        pain: Some(1),
        effort: Some(6),
        note: String::new(),
        source: Source::Simulation,
        warmup_done: false,
        stretches_done: false,
    }
}

/// Per day: training days → done?; rest days → try to recover?
fn build(actions: &[bool]) -> (Vec<SessionLog>, usize) {
    let t = WeekTemplate::default();
    let mut sessions = Vec::new();
    let mut misses = 0;
    for (i, &act) in actions.iter().enumerate() {
        let date = add_days(start(), i as i64);
        if t.day(date).is_training() {
            if act {
                sessions.push(session(i as i64, date, SessionType::Full, None));
            } else {
                misses += 1;
            }
        } else if act {
            let r = evaluate(&t, start(), &sessions, &BTreeSet::new(), date);
            if let Some(m) = r
                .misses
                .iter()
                .rev()
                .copied()
                .find(|m| can_recover(&t, &r, *m, date))
            {
                sessions.push(session(i as i64, date, SessionType::Recovery, Some(m)));
            }
        }
    }
    (sessions, misses)
}

proptest! {
    #[test]
    fn streak_invariants(actions in proptest::collection::vec(any::<bool>(), 7..120)) {
        let t = WeekTemplate::default();
        let (sessions, misses) = build(&actions);
        let today = add_days(start(), actions.len() as i64);
        let r = evaluate(&t, start(), &sessions, &BTreeSet::new(), today);
        prop_assert!(r.heat <= 100);
        prop_assert!(r.current <= r.total_done);
        prop_assert!(r.best >= r.current);
        prop_assert_eq!(r.total_done as usize, sessions.len());
        prop_assert_eq!(r.misses.len(), misses);
        prop_assert!(r.recovered.len() <= r.misses.len());
        prop_assert!(r.revivals as usize <= r.misses.len());
        if misses == 0 {
            prop_assert!(r.breaks.is_empty());
            prop_assert_eq!(r.current, r.total_done);
        }
        for (_, h) in &r.heat_history {
            prop_assert!(*h <= 100);
        }
    }

    /// Never two training days missed in a row, and Friday always done (its window has no training day):
    /// the streak can never break.
    #[test]
    fn single_misses_revived_never_break(actions in proptest::collection::vec(any::<bool>(), 7..120)) {
        let t = WeekTemplate::default();
        let mut fixed = actions.clone();
        let mut last_training_missed = false;
        for (i, a) in fixed.iter_mut().enumerate() {
            let date = add_days(start(), i as i64);
            if t.day(date).is_training() {
                if date.weekday() == Weekday::Fri || last_training_missed {
                    *a = true;
                }
                last_training_missed = !*a;
            }
        }
        // The first training day after the actions is done, so the last miss is always followed by a session.
        let (mut sessions, _) = build(&fixed);
        let end = add_days(start(), fixed.len() as i64);
        let next_training = (0..7).map(|k| add_days(end, k)).find(|d| t.day(*d).is_training()).unwrap();
        sessions.push(session(9999, next_training, SessionType::Full, None));
        let r = evaluate(&t, start(), &sessions, &BTreeSet::new(), add_days(next_training, 1));
        prop_assert!(r.breaks.is_empty(), "breaks {:?} misses {:?}", r.breaks, r.misses);
        prop_assert_eq!(r.current, r.total_done);
    }

    #[test]
    fn progression_stays_buildable_and_never_drops(
        sessions in proptest::collection::vec(
            proptest::collection::vec((0u16..25, any::<bool>()), 0..5), 1..60)
    ) {
        let eq = Equipment::initial();
        for id in ["db_bench_press", "goblet_box_squat", "one_arm_row", "incline_pushup"] {
            let ex = exercise(id).unwrap();
            let mut s = initial_state(ex, &eq);
            for sets in &sessions {
                let results: Vec<SetResult> = sets.iter().map(|(r, ok)| SetResult { reps: *r, load: s.load, ok: *ok }).collect();
                let top = all_sets_at_top(ex, ex.sets, &results);
                let (next, event) = advance(ex, s, &eq, &results, ex.sets);
                prop_assert!(next.load >= s.load, "{} dropped", id);
                if ex.load != LoadMode::Bodyweight {
                    prop_assert!(eq.achievable(ex.load).contains(&next.load));
                    prop_assert!(next.load <= eq.max(ex.load));
                }
                prop_assert_eq!(event.is_some(), top, "progress only when every set hits the top");
                s = next;
            }
        }
    }

    #[test]
    fn snap_returns_the_nearest_buildable_load(target in 0u32..20_000) {
        let eq = Equipment::initial();
        for mode in [LoadMode::Pair, LoadMode::Single] {
            let all = eq.achievable(mode);
            let got = eq.snap(mode, Grams(target));
            prop_assert!(all.contains(&got));
            let best = all.iter().map(|g| g.0.abs_diff(target)).min().unwrap();
            prop_assert_eq!(got.0.abs_diff(target), best);
        }
    }
}
