//! Daily habits and the progressive steps goal.

use crate::model::HabitDay;
use serde::{Deserialize, Serialize};

pub const STEPS_START: u32 = 6000;
pub const STEPS_STEP: u32 = 1000;
pub const STEPS_EVERY_WEEKS: u32 = 2;
pub const STEPS_CAP: u32 = 10_000;

/// 6,000 + 1,000 every 2 weeks, capped at 10,000.
pub fn steps_goal(program_week: u32) -> u32 {
    let bumps = program_week.saturating_sub(1) / STEPS_EVERY_WEEKS;
    (STEPS_START + bumps * STEPS_STEP).min(STEPS_CAP)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Habit {
    Steps,
    Protein,
    Water,
    Vegetables,
    NoSugarAlcohol,
    McGillBig3,
    Sleep7h,
}

impl Habit {
    pub const ALL: [Habit; 7] = [
        Habit::Steps,
        Habit::Protein,
        Habit::Water,
        Habit::Vegetables,
        Habit::NoSugarAlcohol,
        Habit::McGillBig3,
        Habit::Sleep7h,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Habit::Steps => "steps",
            Habit::Protein => "protein",
            Habit::Water => "water",
            Habit::Vegetables => "vegetables",
            Habit::NoSugarAlcohol => "no_sugar_alcohol",
            Habit::McGillBig3 => "mcgill_big3",
            Habit::Sleep7h => "sleep_7h",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Habit::Steps => "Pasos",
            Habit::Protein => "Proteína",
            Habit::Water => "Agua",
            Habit::Vegetables => "Verduras",
            Habit::NoSugarAlcohol => "Cero bebidas azucaradas / alcohol",
            Habit::McGillBig3 => "Big 3 de McGill",
            Habit::Sleep7h => "Sueño 7+ h",
        }
    }
}

/// Which habits are met. Steps and protein are measured against today's goals.
pub fn habits_met(
    day: &HabitDay,
    steps_goal: u32,
    protein_g: u32,
    protein_target: u32,
) -> Vec<Habit> {
    Habit::ALL
        .into_iter()
        .filter(|h| match h {
            Habit::Steps => day.steps.is_some_and(|s| s >= steps_goal),
            Habit::Protein => protein_target > 0 && protein_g >= protein_target,
            Habit::Water => day.water,
            Habit::Vegetables => day.vegetables,
            Habit::NoSugarAlcohol => day.no_sugar_alcohol,
            Habit::McGillBig3 => day.mcgill_big3,
            Habit::Sleep7h => day.sleep_7h,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_goal_progression() {
        assert_eq!(steps_goal(1), 6000);
        assert_eq!(steps_goal(2), 6000);
        assert_eq!(steps_goal(3), 7000);
        assert_eq!(steps_goal(8), 9000);
        assert_eq!(steps_goal(9), 10_000);
        assert_eq!(steps_goal(40), 10_000);
    }

    #[test]
    fn perfect_day() {
        let day = HabitDay {
            date: None,
            steps: Some(6500),
            water: true,
            vegetables: true,
            no_sugar_alcohol: true,
            mcgill_big3: true,
            sleep_7h: true,
        };
        assert_eq!(habits_met(&day, 6000, 150, 145).len(), 7);
        assert_eq!(habits_met(&day, 7000, 100, 145).len(), 5);
    }
}
