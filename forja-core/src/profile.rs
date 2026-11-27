//! The user's profile, home equipment and commitment contract.

use crate::time::week_monday;
use crate::units::Grams;
use chrono::{NaiveDate, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sex {
    Male,
    Female,
}

/// How an exercise is loaded with the home equipment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadMode {
    /// No external load.
    Bodyweight,
    /// Two dumbbells loaded the same: each one gets half of every plate stack.
    Pair,
    /// One dumbbell loaded with everything available.
    Single,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlateStack {
    pub weight: Grams,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Equipment {
    pub handle: Grams,
    pub plates: Vec<PlateStack>,
    /// Optional manual cap per dumbbell when used as a pair.
    pub pair_cap: Option<Grams>,
    /// Optional manual cap for a single loaded dumbbell.
    pub single_cap: Option<Grams>,
    /// How many plates fit on one handle (sleeve length).
    #[serde(default)]
    pub max_plates_per_bell: Option<u32>,
    pub has_flat_bench: bool,
}

impl Equipment {
    /// The user's inventory: iron handles of ~1.5 kg, plates 4 × 2.5 kg, 6 × 2 kg, 6 × 1.25 kg, a flat bench.
    /// A handle holds at most 8 plates (estimated sleeve length; configurable).
    pub fn initial() -> Self {
        let stack = |g: u32, count: u32| PlateStack {
            weight: Grams(g),
            count,
        };
        Equipment {
            handle: Grams(1500),
            plates: vec![stack(2500, 4), stack(2000, 6), stack(1250, 6)],
            max_plates_per_bell: Some(8),
            pair_cap: None,
            single_cap: None,
            has_flat_bench: true,
        }
    }

    /// Every load (per dumbbell) that can actually be built, ascending.
    pub fn achievable(&self, mode: LoadMode) -> Vec<Grams> {
        match mode {
            LoadMode::Bodyweight => vec![Grams::ZERO],
            LoadMode::Pair => self.combos(|c| c / 2, self.pair_cap),
            LoadMode::Single => self.combos(|c| c, self.single_cap),
        }
    }

    fn combos(&self, per_bell: impl Fn(u32) -> u32, cap: Option<Grams>) -> Vec<Grams> {
        // (plate weight, plates used) pairs, so the sleeve limit can be applied.
        let limit = self.max_plates_per_bell.unwrap_or(u32::MAX);
        let mut sums = vec![(0u32, 0u32)];
        for stack in &self.plates {
            let n = per_bell(stack.count);
            let mut next = Vec::with_capacity(sums.len() * (n as usize + 1));
            for &(base, used) in &sums {
                for k in 0..=n {
                    if used + k <= limit {
                        next.push((base + k * stack.weight.0, used + k));
                    }
                }
            }
            // Keep the fewest plates for each weight.
            next.sort_unstable();
            next.dedup_by_key(|x| x.0);
            sums = next;
        }
        let mut out: Vec<Grams> = sums
            .into_iter()
            .map(|(p, _)| Grams(p + self.handle.0))
            .filter(|g| cap.is_none_or(|c| *g <= c))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn max(&self, mode: LoadMode) -> Grams {
        self.achievable(mode).last().copied().unwrap_or(Grams::ZERO)
    }

    /// Nearest buildable load; ties go to the lighter one.
    pub fn snap(&self, mode: LoadMode, target: Grams) -> Grams {
        let loads = self.achievable(mode);
        let mut best = loads[0];
        for &g in &loads {
            let d = g.0.abs_diff(target.0);
            let bd = best.0.abs_diff(target.0);
            if d < bd {
                best = g;
            }
        }
        best
    }

    /// The buildable load heavier than `current` closest to `current + step` (ties → lighter).
    /// `None` when `current` is already the heaviest.
    pub fn next_step(&self, mode: LoadMode, current: Grams, step: Grams) -> Option<Grams> {
        let target = current.0 + step.0;
        self.achievable(mode)
            .into_iter()
            .filter(|g| *g > current)
            .min_by_key(|g| (g.0.abs_diff(target), g.0))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Written by the user during onboarding; empty until then.
    pub display_name: String,
    pub sex: Sex,
    pub age_years: u32,
    pub height_cm: u32,
    pub start_weight: Grams,
    pub timezone: String,
    pub training_days: Vec<Weekday>,
    pub training_time: NaiveTime,
    pub equipment: Equipment,
    /// Monday of program week 1.
    pub program_start: NaiveDate,
    /// First day misses are counted (onboarding date).
    pub tracking_start: NaiveDate,
    /// Activity factor ×1000 (1450 = 1.45).
    pub activity_factor_milli: u32,
    pub deficit_kcal: u32,
    /// Accumulated plateau adjustments (negative lowers the target).
    pub kcal_adjustment: i32,
    /// Base for the max heart rate estimate (220 − age).
    pub max_hr_base: u32,
}

impl Profile {
    /// The preloaded profile from the brief (section 2). Records start empty.
    pub fn initial(onboarding: NaiveDate) -> Self {
        Profile {
            display_name: String::new(),
            sex: Sex::Male,
            age_years: 30,
            height_cm: 173,
            start_weight: Grams(104_000),
            timezone: "America/Santiago".to_string(),
            training_days: vec![Weekday::Mon, Weekday::Tue, Weekday::Thu, Weekday::Fri],
            training_time: NaiveTime::from_hms_opt(7, 0, 0).expect("valid time"),
            equipment: Equipment::initial(),
            program_start: week_monday(onboarding),
            tracking_start: onboarding,
            activity_factor_milli: 1450,
            deficit_kcal: 600,
            kcal_adjustment: 0,
            max_hr_base: 220,
        }
    }

    pub fn max_heart_rate(&self) -> u32 {
        self.max_hr_base.saturating_sub(self.age_years)
    }
}

/// The commitment the user writes in onboarding. Empty until signed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contract {
    pub why: String,
    pub commitment: String,
    pub penalty: String,
    pub partner: Option<String>,
    pub signed_on: Option<NaiveDate>,
}

impl Contract {
    pub fn is_signed(&self) -> bool {
        self.signed_on.is_some() && !self.why.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equipment_matches_the_users_inventory() {
        let e = Equipment::initial();
        // Pair: 1.5 + 2×2.5 + 3×2 + 3×1.25 (8 plates per handle).
        assert_eq!(e.max(LoadMode::Pair), Grams(16250));
        // Single: 1.5 + 4×2.5 + 4×2 (the sleeve fits 8 plates).
        assert_eq!(e.max(LoadMode::Single), Grams(19500));
        assert_eq!(e.achievable(LoadMode::Pair)[0], Grams(1500));
        // Without the sleeve limit everything goes on one handle.
        let mut no_limit = e.clone();
        no_limit.max_plates_per_bell = None;
        assert_eq!(no_limit.max(LoadMode::Single), Grams(31000));
    }

    #[test]
    fn snapping_and_steps() {
        let e = Equipment::initial();
        // 7 kg can't be built as a pair: 6.75 and 7.25 are equally close → lighter.
        assert_eq!(e.snap(LoadMode::Pair, Grams(7000)), Grams(6750));
        assert_eq!(e.snap(LoadMode::Single, Grams(9000)), Grams(9000));
        assert_eq!(
            e.next_step(LoadMode::Pair, Grams(6750), Grams(1250)),
            Some(Grams(8000))
        );
        assert_eq!(e.next_step(LoadMode::Pair, Grams(16250), Grams(1250)), None);
        assert_eq!(
            e.next_step(LoadMode::Single, Grams(9000), Grams(1250)),
            Some(Grams(10250))
        );
    }

    #[test]
    fn caps_are_configurable() {
        let mut e = Equipment::initial();
        e.pair_cap = Some(Grams(8000));
        assert_eq!(e.max(LoadMode::Pair), Grams(8000));
    }

    #[test]
    fn initial_profile() {
        let p = Profile::initial(NaiveDate::from_ymd_opt(2026, 11, 27).unwrap());
        assert_eq!(
            p.program_start,
            NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
        );
        assert_eq!(p.max_heart_rate(), 190);
        assert!(!Contract::default().is_signed());
    }
}
