//! Records the user creates. Everything else (level, heat, streak, loads) is derived from these.

use crate::units::Grams;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

macro_rules! text_enum {
    ($name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub enum $name { $($variant),+ }
        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($text => Some($name::$variant),)+ _ => None }
            }
        }
    };
}
pub(crate) use text_enum;

text_enum!(SessionKind {
    UpperA => "upper_a",
    LowerA => "lower_a",
    UpperB => "upper_b",
    LowerB => "lower_b",
});

impl SessionKind {
    pub fn name(self) -> &'static str {
        match self {
            SessionKind::UpperA => "Upper A",
            SessionKind::LowerA => "Lower A",
            SessionKind::UpperB => "Upper B",
            SessionKind::LowerB => "Lower B",
        }
    }

    pub fn focus(self) -> &'static str {
        match self {
            SessionKind::UpperA => "Empuje + tirón horizontal",
            SessionKind::LowerA => "Sentadilla a banca + bisagra de cadera",
            SessionKind::UpperB => "Hombros + espalda (todo apoyado)",
            SessionKind::LowerB => "Unilateral + core McGill",
        }
    }

    pub fn is_upper(self) -> bool {
        matches!(self, SessionKind::UpperA | SessionKind::UpperB)
    }
}

text_enum!(SessionType {
    Full => "full",
    Minimum => "minimum",
    Recovery => "recovery",
    Skipped => "skipped",
});

impl SessionType {
    /// Counts as a session done for streaks and the week.
    pub fn counts_as_done(self) -> bool {
        !matches!(self, SessionType::Skipped)
    }
}

text_enum!(Source {
    Phone => "phone",
    Watch => "watch",
    Desktop => "desktop",
    Import => "import",
    Simulation => "simulation",
});

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionLog {
    pub id: i64,
    /// Local (Santiago) date the session counts for.
    pub date: NaiveDate,
    pub kind: SessionKind,
    pub session_type: SessionType,
    /// For recoveries: the missed day this session makes up.
    pub recovered_from: Option<NaiveDate>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub pain: Option<u8>,
    pub effort: Option<u8>,
    pub note: String,
    pub source: Source,
    pub warmup_done: bool,
    pub stretches_done: bool,
}

impl SessionLog {
    pub fn is_closed(&self) -> bool {
        self.ended_at.is_some()
    }

    /// Closed and counting as done (full, minimum or recovery).
    pub fn is_done(&self) -> bool {
        self.is_closed() && self.session_type.counts_as_done()
    }

    pub fn duration_minutes(&self) -> Option<i64> {
        self.ended_at.map(|e| (e - self.started_at).num_minutes())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetLog {
    pub session_id: i64,
    pub exercise_id: String,
    pub index: u8,
    /// Reps, or seconds for isometrics.
    pub reps: u16,
    pub load: Grams,
    pub ok: bool,
    pub watch_verified: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HabitDay {
    pub date: Option<NaiveDate>,
    pub steps: Option<u32>,
    pub water: bool,
    pub vegetables: bool,
    pub no_sugar_alcohol: bool,
    pub mcgill_big3: bool,
    pub sleep_7h: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProteinEntry {
    pub date: NaiveDate,
    pub at: DateTime<Utc>,
    pub grams: u32,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightEntry {
    pub date: NaiveDate,
    pub weight: Grams,
    pub at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrengthTest {
    pub date: Option<NaiveDate>,
    pub pushups_max: Option<u32>,
    pub pushup_level: Option<u8>,
    pub box_squats_60s: Option<u32>,
    pub side_plank_worst_s: Option<u32>,
    pub waist_mm: Option<u32>,
    pub weight: Option<Grams>,
}

/// Self-reported red-flag symptoms for the back (never a diagnosis).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackCheck {
    pub date: Option<NaiveDate>,
    pub radiating_leg_pain: bool,
    pub tingling: bool,
    pub weakness: bool,
}

impl BackCheck {
    pub fn any(&self) -> bool {
        self.radiating_leg_pain || self.tingling || self.weakness
    }
}

/// Minimum characters to justify skipping a session.
pub const SKIP_REASON_MIN_CHARS: usize = 25;

pub fn validate_skip_reason(reason: &str) -> Result<(), String> {
    let n = reason.trim().chars().count();
    if n >= SKIP_REASON_MIN_CHARS {
        Ok(())
    } else {
        Err(format!(
            "Escribe el motivo con al menos {SKIP_REASON_MIN_CHARS} caracteres (llevas {n})."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_roundtrip() {
        for k in SessionKind::ALL {
            assert_eq!(SessionKind::parse(k.as_str()), Some(*k));
        }
        for t in SessionType::ALL {
            assert_eq!(SessionType::parse(t.as_str()), Some(*t));
        }
        assert!(!SessionType::Skipped.counts_as_done());
    }

    #[test]
    fn skip_reason_needs_25_chars() {
        assert!(validate_skip_reason("no tengo ganas").is_err());
        assert!(validate_skip_reason("   corto   ").is_err());
        assert!(validate_skip_reason("Tengo turno de noche y no dormí nada").is_ok());
        // Accents count as one character each.
        assert!(validate_skip_reason("ñññññññññññññññññññññññññ").is_ok());
    }
}
