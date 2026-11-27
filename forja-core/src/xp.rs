//! XP, levels, titles and the heat multiplier. XP is an append-only ledger (`xp_event`);
//! the level is always recomputed from it.

use crate::model::{text_enum, SessionType};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

pub const XP_SESSION_FULL: u32 = 100;
pub const XP_SESSION_MINIMUM: u32 = 50;
pub const XP_SESSION_RECOVERED: u32 = 80;
pub const XP_PER_SET: u32 = 5;
pub const XP_WARMUP: u32 = 15;
pub const XP_STRETCHES: u32 = 15;
pub const XP_HABIT: u32 = 5;
pub const XP_PERFECT_HABITS: u32 = 25;
pub const XP_WEIGH_IN: u32 = 10;
pub const XP_STRENGTH_TEST: u32 = 60;
/// Base XP a single session can give before multipliers (no reward for excess volume).
pub const SESSION_XP_CAP: u32 = 250;
/// Sessions shorter than this (or without sets) give reduced XP and don't hurt bosses.
pub const MIN_SESSION_MINUTES: i64 = 8;
pub const REDUCED_XP_PCT: u32 = 50;
pub const WATCH_BONUS_PCT: u32 = 110;

text_enum!(XpReason {
    SessionFull => "session_full",
    SessionMinimum => "session_minimum",
    SessionRecovered => "session_recovered",
    Sets => "sets",
    Warmup => "warmup",
    Stretches => "stretches",
    Habit => "habit",
    PerfectHabits => "perfect_habits",
    WeighIn => "weigh_in",
    StrengthTest => "strength_test",
});

impl XpReason {
    pub fn label(self) -> &'static str {
        match self {
            XpReason::SessionFull => "Sesión completa",
            XpReason::SessionMinimum => "Versión mínima",
            XpReason::SessionRecovered => "Sesión recuperada",
            XpReason::Sets => "Series",
            XpReason::Warmup => "Calentamiento",
            XpReason::Stretches => "Estiramientos",
            XpReason::Habit => "Hábito",
            XpReason::PerfectHabits => "Día perfecto de hábitos",
            XpReason::WeighIn => "Pesaje",
            XpReason::StrengthTest => "Prueba de fuerza",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct XpEvent {
    pub id: i64,
    pub at: DateTime<Utc>,
    pub date: NaiveDate,
    pub reason: XpReason,
    pub base: u32,
    /// Combined multiplier in percent (heat × watch × anti-cheat).
    pub multiplier_pct: u32,
    pub amount: u32,
    /// Related record (session id, …).
    pub ref_id: Option<i64>,
    /// Discriminator for idempotent awards (e.g. habit id).
    pub note: String,
}

impl XpEvent {
    pub fn new(
        at: DateTime<Utc>,
        date: NaiveDate,
        reason: XpReason,
        base: u32,
        multiplier_pct: u32,
    ) -> Self {
        XpEvent {
            id: 0,
            at,
            date,
            reason,
            base,
            multiplier_pct,
            amount: apply_pct(base, multiplier_pct),
            ref_id: None,
            note: String::new(),
        }
    }
}

pub fn apply_pct(base: u32, pct: u32) -> u32 {
    (base * pct + 50) / 100
}

/// XP to go from `level` to `level + 1`: round(120 · n^1.35).
pub fn xp_to_next(level: u32) -> u32 {
    (120.0 * f64::from(level.max(1)).powf(1.35)).round() as u32
}

pub fn title(level: u32) -> &'static str {
    match level {
        0..=4 => "Aprendiz",
        5..=9 => "Ayudante",
        10..=19 => "Oficial de fragua",
        20..=34 => "Herrero",
        35..=49 => "Maestro herrero",
        _ => "Forjador de leyenda",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LevelInfo {
    pub level: u32,
    pub total_xp: u64,
    pub into_level: u32,
    pub needed: u32,
    pub title: &'static str,
}

pub fn level_for(total_xp: u64) -> LevelInfo {
    let mut level = 1;
    let mut rest = total_xp;
    loop {
        let need = u64::from(xp_to_next(level));
        if rest < need {
            return LevelInfo {
                level,
                total_xp,
                into_level: rest as u32,
                needed: need as u32,
                title: title(level),
            };
        }
        rest -= need;
        level += 1;
    }
}

/// Heat multiplier in percent: <30 ×0.8 · 30–69 ×1 · 70–89 ×1.25 · ≥90 ×1.5.
pub fn heat_multiplier_pct(heat: u8) -> u32 {
    match heat {
        0..=29 => 80,
        30..=69 => 100,
        70..=89 => 125,
        _ => 150,
    }
}

pub fn heat_label(heat: u8) -> &'static str {
    match heat {
        0..=29 => "Forja fría",
        30..=69 => "Forja encendida",
        70..=89 => "Forja caliente",
        _ => "Al rojo vivo",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionXpInput {
    pub session_type: SessionType,
    pub ok_sets: u32,
    pub planned_sets: u32,
    pub warmup_done: bool,
    pub stretches_done: bool,
    pub duration_minutes: i64,
    pub heat_at_start: u8,
    pub watch_verified: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionXp {
    /// (reason, base XP) before multipliers, already capped.
    pub parts: [(XpReason, u32); 4],
    pub multiplier_pct: u32,
    /// False when the session was too short or had no sets.
    pub counts_for_boss: bool,
}

impl SessionXp {
    pub fn total(&self) -> u32 {
        self.parts
            .iter()
            .map(|(_, b)| apply_pct(*b, self.multiplier_pct))
            .sum()
    }
}

pub fn session_xp(input: SessionXpInput) -> SessionXp {
    let session = match input.session_type {
        SessionType::Full => (XpReason::SessionFull, XP_SESSION_FULL),
        SessionType::Minimum => (XpReason::SessionMinimum, XP_SESSION_MINIMUM),
        SessionType::Recovery => (XpReason::SessionRecovered, XP_SESSION_RECOVERED),
        SessionType::Skipped => (XpReason::SessionFull, 0),
    };
    // Only planned sets earn XP.
    let mut sets = input.ok_sets.min(input.planned_sets) * XP_PER_SET;
    let warm = if input.warmup_done { XP_WARMUP } else { 0 };
    let stretch = if input.stretches_done {
        XP_STRETCHES
    } else {
        0
    };
    let over = (session.1 + sets + warm + stretch).saturating_sub(SESSION_XP_CAP);
    sets = sets.saturating_sub(over);
    let legit = input.ok_sets > 0 && input.duration_minutes >= MIN_SESSION_MINUTES;
    let mut pct = heat_multiplier_pct(input.heat_at_start);
    if input.watch_verified {
        pct = pct * WATCH_BONUS_PCT / 100;
    }
    if !legit {
        pct = pct * REDUCED_XP_PCT / 100;
    }
    SessionXp {
        parts: [
            session,
            (XpReason::Sets, sets),
            (XpReason::Warmup, warm),
            (XpReason::Stretches, stretch),
        ],
        multiplier_pct: pct,
        counts_for_boss: legit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_curve() {
        assert_eq!(xp_to_next(1), 120);
        assert_eq!(xp_to_next(2), 306);
        assert_eq!(level_for(0).level, 1);
        assert_eq!(level_for(119).level, 1);
        assert_eq!(level_for(120).level, 2);
        assert_eq!(level_for(426).level, 3);
        assert_eq!(title(1), "Aprendiz");
        assert_eq!(title(5), "Ayudante");
        assert_eq!(title(10), "Oficial de fragua");
        assert_eq!(title(50), "Forjador de leyenda");
    }

    fn input() -> SessionXpInput {
        SessionXpInput {
            session_type: SessionType::Full,
            ok_sets: 17,
            planned_sets: 17,
            warmup_done: true,
            stretches_done: true,
            duration_minutes: 45,
            heat_at_start: 50,
            watch_verified: false,
        }
    }

    #[test]
    fn first_full_session_reaches_level_2() {
        let x = session_xp(input());
        assert!(u64::from(x.total()) >= u64::from(xp_to_next(1)));
        // Even a minimum version at a cold forge gets close.
        let m = session_xp(SessionXpInput {
            session_type: SessionType::Minimum,
            ok_sets: 8,
            planned_sets: 8,
            heat_at_start: 0,
            ..input()
        });
        assert_eq!(
            m.total(),
            apply_pct(50, 80) + apply_pct(40, 80) + apply_pct(15, 80) * 2
        );
    }

    #[test]
    fn no_xp_for_excess_volume() {
        let normal = session_xp(input()).total();
        let excessive = session_xp(SessionXpInput {
            ok_sets: 60,
            ..input()
        })
        .total();
        assert_eq!(normal, excessive);
        let huge_plan = session_xp(SessionXpInput {
            ok_sets: 60,
            planned_sets: 60,
            ..input()
        });
        let base: u32 = huge_plan.parts.iter().map(|p| p.1).sum();
        assert_eq!(base, SESSION_XP_CAP);
    }

    #[test]
    fn multipliers() {
        assert_eq!(heat_multiplier_pct(29), 80);
        assert_eq!(heat_multiplier_pct(30), 100);
        assert_eq!(heat_multiplier_pct(70), 125);
        assert_eq!(heat_multiplier_pct(90), 150);
        let short = session_xp(SessionXpInput {
            duration_minutes: 5,
            ..input()
        });
        assert!(!short.counts_for_boss);
        assert_eq!(short.multiplier_pct, 50);
        let watch = session_xp(SessionXpInput {
            watch_verified: true,
            ..input()
        });
        assert_eq!(watch.multiplier_pct, 110);
    }
}
