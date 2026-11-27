//! Lower-back safety rules, encoded (not just written). Forja never diagnoses.

use crate::catalog::exercise;
use crate::model::{BackCheck, SessionLog};
use crate::time::add_days;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Pain at or below this (0–10) is "go".
pub const PAIN_OK_MAX: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PainLight {
    /// 0–3 and not worse than the previous report: carry on.
    Green,
    /// 0–3 but worse than last time: substitutions are offered.
    Amber,
    /// Above 3: risky exercises are swapped automatically.
    Red,
}

impl PainLight {
    pub fn label(self) -> &'static str {
        match self {
            PainLight::Green => "Verde: sigue con el plan",
            PainLight::Amber => "Ámbar: el dolor subió; puedes usar las alternativas",
            PainLight::Red => "Rojo: cambiamos los ejercicios de riesgo por alternativas seguras",
        }
    }
}

pub fn pain_light(last: Option<u8>, previous: Option<u8>) -> PainLight {
    match (last, previous) {
        (Some(p), _) if p > PAIN_OK_MAX => PainLight::Red,
        (Some(p), Some(q)) if p > q => PainLight::Amber,
        _ => PainLight::Green,
    }
}

/// Safe swap for an exercise when the back complains (section 3.5).
pub fn substitute(exercise_id: &str) -> Option<&'static str> {
    Some(match exercise_id {
        "romanian_deadlift" | "rdl_tempo4" => "hip_thrust",
        "single_leg_rdl" => "single_leg_glute_bridge",
        "supported_split_squat" | "bulgarian_split_squat" => "wall_sit",
        "goblet_box_squat" | "goblet_1_5" => "high_box_squat_bw",
        "seated_shoulder_press" | "single_arm_shoulder_press" => "paused_floor_press",
        "one_arm_row" => "chest_supported_row",
        _ => return None,
    })
}

pub const RED_FLAG_MESSAGE: &str = "Dolor que baja por la pierna, hormigueo, debilidad o un dolor que empeora \
hace 2 semanas son señales para consultar a un médico o kinesiólogo. Mientras tanto pausamos los ejercicios \
con carga sobre la columna. Forja no diagnostica: esto es solo precaución.";

/// Pain getting worse for two weeks: at least 4 reports in the last 14 days, last week's average
/// at least 1 point above the week before, and above the "go" limit. (Thresholds estimated.)
pub fn worsening_two_weeks(pain: &[(NaiveDate, u8)], today: NaiveDate) -> bool {
    let week1_start = add_days(today, -13);
    let week2_start = add_days(today, -6);
    let in_range = |d: NaiveDate, a: NaiveDate, b: NaiveDate| d >= a && d <= b;
    let w1: Vec<f64> = pain
        .iter()
        .filter(|(d, _)| in_range(*d, week1_start, add_days(week2_start, -1)))
        .map(|(_, p)| f64::from(*p))
        .collect();
    let w2: Vec<f64> = pain
        .iter()
        .filter(|(d, _)| in_range(*d, week2_start, today))
        .map(|(_, p)| f64::from(*p))
        .collect();
    if w1.len() + w2.len() < 4 || w1.is_empty() || w2.is_empty() {
        return false;
    }
    let avg = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let (a1, a2) = (avg(&w1), avg(&w2));
    a2 >= a1 + 1.0 && a2 > f64::from(PAIN_OK_MAX)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SafetyStatus {
    pub light: PainLight,
    pub last_pain: Option<u8>,
    pub red_flags: bool,
    pub red_flag_reasons: Vec<String>,
}

impl SafetyStatus {
    pub fn clear() -> Self {
        SafetyStatus {
            light: PainLight::Green,
            last_pain: None,
            red_flags: false,
            red_flag_reasons: Vec::new(),
        }
    }

    /// Whether an exercise is paused today (axial load under red flags).
    pub fn paused(&self, exercise_id: &str) -> bool {
        self.red_flags && exercise(exercise_id).is_some_and(|e| e.axial_load)
    }
}

/// Safety status for `today` from closed sessions (chronological) and the latest back check.
pub fn safety_status(
    sessions: &[SessionLog],
    back_check: Option<&BackCheck>,
    today: NaiveDate,
) -> SafetyStatus {
    let pains: Vec<(NaiveDate, u8)> = sessions
        .iter()
        .filter(|s| s.is_closed() && s.date <= today)
        .filter_map(|s| s.pain.map(|p| (s.date, p)))
        .collect();
    let last = pains.last().map(|p| p.1);
    let previous = if pains.len() >= 2 {
        Some(pains[pains.len() - 2].1)
    } else {
        None
    };
    let mut reasons = Vec::new();
    if let Some(c) = back_check {
        let recent = c
            .date
            .is_some_and(|d| d <= today && d >= add_days(today, -14));
        if recent {
            if c.radiating_leg_pain {
                reasons.push("Dolor que baja por la pierna".to_string());
            }
            if c.tingling {
                reasons.push("Hormigueo".to_string());
            }
            if c.weakness {
                reasons.push("Debilidad".to_string());
            }
        }
    }
    if worsening_two_weeks(&pains, today) {
        reasons.push("El dolor empeora hace 2 semanas".to_string());
    }
    SafetyStatus {
        light: pain_light(last, previous),
        last_pain: last,
        red_flags: !reasons.is_empty(),
        red_flag_reasons: reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(n: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 12, n).unwrap()
    }

    #[test]
    fn traffic_light() {
        assert_eq!(pain_light(None, None), PainLight::Green);
        assert_eq!(pain_light(Some(3), Some(3)), PainLight::Green);
        assert_eq!(pain_light(Some(2), Some(1)), PainLight::Amber);
        assert_eq!(pain_light(Some(4), Some(0)), PainLight::Red);
    }

    #[test]
    fn substitutions_cover_the_brief() {
        assert_eq!(substitute("romanian_deadlift"), Some("hip_thrust"));
        assert_eq!(substitute("supported_split_squat"), Some("wall_sit"));
        assert_eq!(substitute("bulgarian_split_squat"), Some("wall_sit"));
        assert_eq!(substitute("goblet_box_squat"), Some("high_box_squat_bw"));
        assert_eq!(
            substitute("seated_shoulder_press"),
            Some("paused_floor_press")
        );
        assert_eq!(substitute("one_arm_row"), Some("chest_supported_row"));
        assert_eq!(substitute("dead_bug"), None);
        // Every substitute exists and is never itself axially loaded.
        for e in crate::catalog::EXERCISES {
            if let Some(s) = substitute(e.id) {
                assert!(!exercise(s).unwrap().axial_load, "{} → {}", e.id, s);
            }
        }
    }

    #[test]
    fn worsening_needs_two_weeks_of_rising_pain() {
        let rising = [(d(1), 1), (d(3), 2), (d(9), 4), (d(11), 5)];
        assert!(worsening_two_weeks(&rising, d(14)));
        let stable = [(d(1), 2), (d(3), 2), (d(9), 2), (d(11), 3)];
        assert!(!worsening_two_weeks(&stable, d(14)));
        assert!(!worsening_two_weeks(&rising[2..], d(14)));
    }

    #[test]
    fn red_flags_pause_axial_exercises() {
        let check = BackCheck {
            date: Some(d(10)),
            tingling: true,
            ..Default::default()
        };
        let s = safety_status(&[], Some(&check), d(12));
        assert!(s.red_flags);
        assert!(s.paused("goblet_box_squat"));
        assert!(!s.paused("chest_supported_row"));
        // Old checks expire after 14 days.
        let s = safety_status(&[], Some(&check), d(30));
        assert!(!s.red_flags);
    }
}
