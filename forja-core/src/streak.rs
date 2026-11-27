//! Streaks, "never miss twice", session recovery and the forge heat.
//!
//! Everything is a pure fold over the days from `tracking_start` to `today`:
//! * A planned training day with no full/minimum session by the end of the day is a **miss**.
//! * A miss opens the quest "Reavivar la forja": any session (or a recovery on a rest day) in the next
//!   2 days revives it — the streak survives and the heat goes back to at least 50.
//! * A second miss in a row, or letting the revival window expire, **breaks** the streak: the weapon
//!   cracks and the contract is shown.
//! * A recovery done on a rest day within 2 days counts as the missed session.

use crate::model::{SessionLog, SessionType};
use crate::plan::{DayKind, WeekTemplate};
use crate::time::add_days;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const REVIVAL_WINDOW_DAYS: i64 = 2;
pub const INITIAL_HEAT: u8 = 50;
pub const HEAT_PER_SESSION: u8 = 25;
pub const HEAT_MISS_PENALTY: u8 = 35;
pub const HEAT_IDLE_PENALTY: u8 = 5;
pub const HEAT_REVIVED: u8 = 50;
pub const FLAME_TIERS: [u32; 6] = [1, 4, 8, 16, 24, 52];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revival {
    pub missed: NaiveDate,
    /// Last day (inclusive) to revive the forge.
    pub deadline: NaiveDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForgeReport {
    pub today: NaiveDate,
    /// Consecutive sessions without a broken "never miss twice".
    pub current: u32,
    pub best: u32,
    pub total_done: u32,
    pub misses: Vec<NaiveDate>,
    pub recovered: Vec<NaiveDate>,
    pub breaks: Vec<NaiveDate>,
    /// Forges revived in time.
    pub revivals: u32,
    pub pending: Option<Revival>,
    pub cracked: bool,
    /// Forge heat 0–100 as of now.
    pub heat: u8,
    /// Heat at the end of each closed day.
    pub heat_history: Vec<(NaiveDate, u8)>,
}

impl ForgeReport {
    /// 0..=6: how many flame tiers (1, 4, 8, 16, 24, 52 sessions) the streak reached.
    pub fn flame_tier(&self) -> u8 {
        FLAME_TIERS.iter().filter(|t| self.current >= **t).count() as u8
    }

    pub fn is_unrecovered_miss(&self, date: NaiveDate) -> bool {
        self.misses.contains(&date) && !self.recovered.contains(&date)
    }
}

/// Can the session missed on `missed` be recovered on `on`? Only on a rest/review day within 2 days.
pub fn can_recover(
    template: &WeekTemplate,
    report: &ForgeReport,
    missed: NaiveDate,
    on: NaiveDate,
) -> bool {
    let gap = (on - missed).num_days();
    report.is_unrecovered_miss(missed)
        && (1..=REVIVAL_WINDOW_DAYS).contains(&gap)
        && !template.day(on).is_training()
}

/// Fold the history. `activity_days` are days with any logged habit, protein or weight.
pub fn evaluate(
    template: &WeekTemplate,
    tracking_start: NaiveDate,
    sessions: &[SessionLog],
    activity_days: &BTreeSet<NaiveDate>,
    today: NaiveDate,
) -> ForgeReport {
    let mut done: Vec<&SessionLog> = sessions
        .iter()
        .filter(|s| s.is_done() && s.date >= tracking_start)
        .collect();
    done.sort_by_key(|s| (s.date, s.started_at));

    let mut r = ForgeReport {
        today,
        current: 0,
        best: 0,
        total_done: 0,
        misses: Vec::new(),
        recovered: Vec::new(),
        breaks: Vec::new(),
        revivals: 0,
        pending: None,
        cracked: false,
        heat: INITIAL_HEAT,
        heat_history: Vec::new(),
    };

    let mut idx = 0;
    let mut day = tracking_start;
    while day <= today {
        // 1. Sessions done today.
        let mut any_session = false;
        while idx < done.len() && done[idx].date == day {
            let s = done[idx];
            idx += 1;
            if s.session_type == SessionType::Recovery {
                match s.recovered_from {
                    Some(m)
                        if r.is_unrecovered_miss(m)
                            && (day - m).num_days() <= REVIVAL_WINDOW_DAYS =>
                    {
                        r.recovered.push(m);
                    }
                    // A recovery that doesn't match a miss still counts as a session.
                    _ => {}
                }
            }
            any_session = true;
            r.total_done += 1;
            r.current += 1;
            r.best = r.best.max(r.current);
            r.cracked = false;
            r.heat = r.heat.saturating_add(HEAT_PER_SESSION).min(100);
            if let Some(p) = r.pending {
                if day <= p.deadline {
                    r.revivals += 1;
                    r.pending = None;
                    r.heat = r.heat.max(HEAT_REVIVED);
                }
            }
        }

        // 2. Close the day (only past days).
        if day < today {
            if let Some(p) = r.pending {
                if p.deadline == day {
                    r.current = 0;
                    r.cracked = true;
                    r.breaks.push(day);
                    r.pending = None;
                }
            }
            let trained = sessions.iter().any(|s| {
                s.is_done()
                    && s.date == day
                    && matches!(s.session_type, SessionType::Full | SessionType::Minimum)
            });
            if matches!(template.day(day), DayKind::Training(_)) && !trained {
                r.misses.push(day);
                r.heat = r.heat.saturating_sub(HEAT_MISS_PENALTY);
                if r.pending.is_some() {
                    r.current = 0;
                    r.cracked = true;
                    if r.breaks.last() != Some(&day) {
                        r.breaks.push(day);
                    }
                }
                r.pending = Some(Revival {
                    missed: day,
                    deadline: add_days(day, REVIVAL_WINDOW_DAYS),
                });
            } else if !any_session && !activity_days.contains(&day) {
                r.heat = r.heat.saturating_sub(HEAT_IDLE_PENALTY);
            }
            r.heat_history.push((day, r.heat));
        }
        day = add_days(day, 1);
    }
    r
}

/// Heat right before a session that starts on `date` (all earlier sessions counted).
pub fn heat_before(
    template: &WeekTemplate,
    tracking_start: NaiveDate,
    sessions: &[SessionLog],
    activity_days: &BTreeSet<NaiveDate>,
    date: NaiveDate,
    exclude_session: i64,
) -> u8 {
    let others: Vec<SessionLog> = sessions
        .iter()
        .filter(|s| s.id != exclude_session)
        .cloned()
        .collect();
    evaluate(template, tracking_start, &others, activity_days, date).heat
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionKind, Source};
    use crate::time::resolve_local;
    use chrono::NaiveTime;

    fn d(m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, m, day).unwrap()
    }

    pub(crate) fn session(
        id: i64,
        date: NaiveDate,
        t: SessionType,
        from: Option<NaiveDate>,
    ) -> SessionLog {
        let start = resolve_local(date, NaiveTime::from_hms_opt(7, 0, 0).unwrap());
        SessionLog {
            id,
            date,
            kind: SessionKind::UpperA,
            session_type: t,
            recovered_from: from,
            started_at: start,
            ended_at: Some(start + chrono::Duration::minutes(45)),
            pain: Some(1),
            effort: Some(6),
            note: String::new(),
            source: Source::Simulation,
            warmup_done: true,
            stretches_done: true,
        }
    }

    // Week of 2026-11-23 (Mon) .. 2026-11-29 (Sun); training Mon, Tue, Thu, Fri.
    const MON: u32 = 23;

    #[test]
    fn perfect_week() {
        let t = WeekTemplate::default();
        let s: Vec<_> = [23, 24, 26, 27]
            .iter()
            .enumerate()
            .map(|(i, &n)| session(i as i64, d(11, n), SessionType::Full, None))
            .collect();
        let r = evaluate(&t, d(11, MON), &s, &BTreeSet::new(), d(11, 30));
        assert_eq!(r.current, 4);
        assert!(r.misses.is_empty());
        assert!(r.breaks.is_empty());
        assert_eq!(r.flame_tier(), 2);
        // Wed is idle (−5, then Thu refills to 100); Sat and Sun are idle.
        assert_eq!(r.heat, 100 - 2 * HEAT_IDLE_PENALTY);
    }

    #[test]
    fn one_miss_revived_by_next_session_keeps_streak() {
        let t = WeekTemplate::default();
        // Mon done, Tue missed, Thu done (within 2 days of Tue).
        let s = vec![
            session(1, d(11, 23), SessionType::Full, None),
            session(2, d(11, 26), SessionType::Minimum, None),
        ];
        let r = evaluate(&t, d(11, MON), &s, &BTreeSet::new(), d(11, 27));
        assert_eq!(r.misses, vec![d(11, 24)]);
        assert!(r.breaks.is_empty());
        assert_eq!(r.current, 2);
        assert_eq!(r.revivals, 1);
        assert!(r.heat >= HEAT_REVIVED);
    }

    #[test]
    fn recovery_on_rest_day_counts_as_done() {
        let t = WeekTemplate::default();
        let s = vec![
            session(1, d(11, 23), SessionType::Full, None),
            session(2, d(11, 25), SessionType::Recovery, Some(d(11, 24))),
        ];
        let r = evaluate(&t, d(11, MON), &s, &BTreeSet::new(), d(11, 26));
        assert_eq!(r.recovered, vec![d(11, 24)]);
        assert_eq!(r.current, 2);
        assert!(can_recover(
            &t,
            &evaluate(&t, d(11, MON), &s[..1], &BTreeSet::new(), d(11, 25)),
            d(11, 24),
            d(11, 25)
        ));
        // Can't recover on a training day.
        let r1 = evaluate(&t, d(11, MON), &s[..1], &BTreeSet::new(), d(11, 26));
        assert!(!can_recover(&t, &r1, d(11, 24), d(11, 26)));
    }

    #[test]
    fn two_misses_in_a_row_break_and_crack() {
        let t = WeekTemplate::default();
        let s = vec![session(1, d(11, 23), SessionType::Full, None)];
        let r = evaluate(&t, d(11, MON), &s, &BTreeSet::new(), d(11, 26));
        // Tue missed; Wed no recovery; Thu is today (not closed). Deadline for Tue was Thu → not yet.
        assert_eq!(r.breaks, Vec::<NaiveDate>::new());
        assert!(r.pending.is_some());
        let r = evaluate(&t, d(11, MON), &s, &BTreeSet::new(), d(11, 28));
        // Thu missed with Tue pending → break.
        assert_eq!(r.current, 0);
        assert!(r.cracked);
        assert!(!r.breaks.is_empty());
    }

    #[test]
    fn friday_miss_expires_over_the_weekend() {
        let t = WeekTemplate::default();
        let s: Vec<_> = [23, 24, 26]
            .iter()
            .map(|&n| session(n as i64, d(11, n), SessionType::Full, None))
            .collect();
        let r = evaluate(&t, d(11, MON), &s, &BTreeSet::new(), d(11, 30));
        assert_eq!(r.breaks, vec![d(11, 29)]);
        assert_eq!(r.current, 0);
    }

    #[test]
    fn idle_days_cool_the_forge_and_activity_prevents_it() {
        let t = WeekTemplate::default();
        // Start on a Wednesday so only rest days pass.
        let r = evaluate(&t, d(11, 25), &[], &BTreeSet::new(), d(11, 26));
        assert_eq!(r.heat, INITIAL_HEAT - HEAT_IDLE_PENALTY);
        let mut act = BTreeSet::new();
        act.insert(d(11, 25));
        let r = evaluate(&t, d(11, 25), &[], &act, d(11, 26));
        assert_eq!(r.heat, INITIAL_HEAT);
    }

    #[test]
    fn skipped_sessions_are_misses() {
        let t = WeekTemplate::default();
        let s = vec![session(1, d(11, 23), SessionType::Skipped, None)];
        let r = evaluate(&t, d(11, MON), &s, &BTreeSet::new(), d(11, 24));
        assert_eq!(r.misses, vec![d(11, 23)]);
        assert_eq!(r.heat, INITIAL_HEAT - HEAT_MISS_PENALTY);
    }
}
