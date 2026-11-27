//! Nutrition targets (Mifflin-St Jeor) and the plateau rule.

use crate::model::WeightEntry;
use crate::profile::{Profile, Sex};
use crate::time::add_days;
use crate::units::Grams;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub const KCAL_ROUNDING: u32 = 50;
pub const FAT_SHARE_PCT: u32 = 28;
pub const WATER_ML_PER_KG: f64 = 33.0;
pub const FIBER_G: u32 = 30;
pub const PLATEAU_KCAL_STEP: i32 = -150;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NutritionTargets {
    pub weight: Grams,
    pub bmr: u32,
    pub tdee: u32,
    pub kcal: u32,
    pub protein_g: u32,
    pub fat_g: u32,
    pub carbs_g: u32,
    pub water_ml: u32,
    pub fiber_g: u32,
}

/// Basal metabolic rate, kcal/day.
pub fn mifflin_st_jeor(sex: Sex, weight: Grams, height_cm: u32, age_years: u32) -> f64 {
    let base = 10.0 * weight.kg() + 6.25 * f64::from(height_cm) - 5.0 * f64::from(age_years);
    match sex {
        Sex::Male => base + 5.0,
        Sex::Female => base - 161.0,
    }
}

fn round_to(x: f64, step: u32) -> u32 {
    let s = f64::from(step);
    ((x / s).round() * s).max(0.0) as u32
}

fn ceil_to(x: f64, step: u32) -> u32 {
    let s = f64::from(step);
    ((x / s).ceil() * s).max(0.0) as u32
}

/// Targets from the most recent weight (or the starting weight).
pub fn targets(profile: &Profile, latest_weight: Option<Grams>) -> NutritionTargets {
    let weight = latest_weight.unwrap_or(profile.start_weight);
    let bmr = mifflin_st_jeor(profile.sex, weight, profile.height_cm, profile.age_years);
    let tdee = bmr * f64::from(profile.activity_factor_milli) / 1000.0;
    let raw = tdee - f64::from(profile.deficit_kcal) + f64::from(profile.kcal_adjustment);
    // Never below BMR (rounded up): a safety floor (estimated rule).
    let kcal = round_to(raw, KCAL_ROUNDING).max(ceil_to(bmr, KCAL_ROUNDING));
    let protein_raw = 2.0 * f64::from(profile.height_cm.saturating_sub(100));
    let protein_g = round_to(protein_raw, 5);
    let fat_g = (f64::from(kcal) * f64::from(FAT_SHARE_PCT) / 100.0 / 9.0).round() as u32;
    let carbs_kcal = i64::from(kcal) - i64::from(protein_g) * 4 - i64::from(fat_g) * 9;
    let carbs_g = (carbs_kcal.max(0) / 4) as u32;
    NutritionTargets {
        weight,
        bmr: bmr.round() as u32,
        tdee: tdee.round() as u32,
        kcal,
        protein_g,
        fat_g,
        carbs_g,
        water_ml: (WATER_ML_PER_KG * weight.kg()).round() as u32,
        fiber_g: FIBER_G,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PlateauAdvice {
    NotEnoughData,
    Dropping {
        delta_kg: f64,
    },
    /// The 2-week average didn't go down: suggest lowering kcal.
    Stalled {
        delta_kg: f64,
        suggest_kcal: i32,
    },
}

fn avg_between(weights: &[WeightEntry], from: NaiveDate, to: NaiveDate) -> Option<(f64, usize)> {
    let v: Vec<f64> = weights
        .iter()
        .filter(|w| w.date >= from && w.date <= to)
        .map(|w| w.weight.kg())
        .collect();
    if v.is_empty() {
        None
    } else {
        Some((v.iter().sum::<f64>() / v.len() as f64, v.len()))
    }
}

/// Compares the average of the last 14 days with the 14 before (at least 2 weigh-ins each).
pub fn plateau_check(weights: &[WeightEntry], today: NaiveDate) -> PlateauAdvice {
    let recent = avg_between(weights, add_days(today, -13), today);
    let before = avg_between(weights, add_days(today, -27), add_days(today, -14));
    match (recent, before) {
        (Some((r, rn)), Some((b, bn))) if rn >= 2 && bn >= 2 => {
            let delta = r - b;
            if delta < 0.0 {
                PlateauAdvice::Dropping { delta_kg: delta }
            } else {
                PlateauAdvice::Stalled {
                    delta_kg: delta,
                    suggest_kcal: PLATEAU_KCAL_STEP,
                }
            }
        }
        _ => PlateauAdvice::NotEnoughData,
    }
}

/// 7-day moving average per weigh-in date (trend line for the chart).
pub fn moving_average_7d(weights: &[WeightEntry]) -> Vec<(NaiveDate, f64)> {
    weights
        .iter()
        .map(|w| {
            let avg = avg_between(weights, add_days(w.date, -6), w.date)
                .map(|a| a.0)
                .unwrap_or(w.weight.kg());
            (w.date, avg)
        })
        .collect()
}

/// Quick protein buttons: (id, label, grams).
pub const PROTEIN_QUICK: &[(&str, &str, u32)] = &[
    ("egg", "Huevo", 6),
    ("chicken_100", "Pechuga 100 g", 31),
    ("lean_beef_100", "Carne magra 100 g", 26),
    ("tuna_can", "Lata de atún", 25),
    ("jurel_half", "½ lata de jurel", 22),
    ("protein_yogurt", "Yogur proteico", 15),
    ("scoop", "Scoop de proteína", 24),
    ("quesillo_100", "Quesillo 100 g", 13),
    ("milk_glass", "Vaso de leche", 8),
    ("legumes_cup", "Taza de legumbres", 15),
];

pub const HAND_PLATE: &str =
    "Plato de la mano por comida: 1–2 palmas de proteína, 1–2 puños de verduras, \
1 mano ahuecada de carbohidratos (marraqueta integral, arroz, legumbres) y 1 pulgar de grasa.";

pub const EXAMPLE_DAY: &[(&str, &str)] = &[
    (
        "Desayuno",
        "2–3 huevos revueltos, ½ marraqueta integral, tomate",
    ),
    (
        "Almuerzo",
        "Pechuga de pollo, porotos o lentejas, ensalada chilena",
    ),
    ("Once", "Yogur proteico o quesillo con pan integral"),
    ("Cena", "½ lata de jurel o atún, ensalada grande, palta"),
];

pub const RULE_80_20: &str =
    "80/20: el 80 % de lo que comes sigue el plan; el 20 % es flexible, sin culpa.";

pub const SUPPLEMENTS: &str =
    "Creatina monohidratada 3–5 g al día es el único suplemento recomendado, \
además de la proteína en polvo si te cuesta llegar a la meta.";

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn profile() -> Profile {
        Profile::initial(NaiveDate::from_ymd_opt(2026, 11, 23).unwrap())
    }

    #[test]
    fn initial_targets_follow_the_formula() {
        let t = targets(&profile(), None);
        // 10·104 + 6.25·173 − 5·30 + 5 = 1976.25
        assert_eq!(t.bmr, 1976);
        // ×1.45 = 2865.6 ; −600 = 2265.6 → 2250 (the brief said ~2150; see DECISIONS.md D6)
        assert_eq!(t.kcal, 2250);
        assert_eq!(t.protein_g, 145);
        assert_eq!(t.fat_g, 70);
        assert_eq!(t.water_ml, 3432);
        assert_eq!(t.fiber_g, 30);
        assert_eq!(t.carbs_g, (2250 - 145 * 4 - 70 * 9) / 4);
    }

    #[test]
    fn adjustment_and_floor() {
        let mut p = profile();
        p.kcal_adjustment = -150;
        assert_eq!(targets(&p, None).kcal, 2100);
        p.kcal_adjustment = -2000;
        assert_eq!(targets(&p, None).kcal, 2000, "never below BMR");
        p.activity_factor_milli = 1400;
        p.kcal_adjustment = 0;
        assert_eq!(targets(&p, None).kcal, 2150);
    }

    fn w(day: u32, kg: f64) -> WeightEntry {
        let date = NaiveDate::from_ymd_opt(2026, 12, day).unwrap();
        WeightEntry {
            date,
            weight: Grams::from_kg(kg),
            at: Utc.from_utc_datetime(&date.and_hms_opt(11, 0, 0).unwrap()),
        }
    }

    #[test]
    fn plateau_rule() {
        let today = NaiveDate::from_ymd_opt(2026, 12, 28).unwrap();
        let dropping = [w(1, 104.0), w(8, 103.6), w(15, 103.1), w(22, 102.8)];
        assert!(matches!(
            plateau_check(&dropping, today),
            PlateauAdvice::Dropping { .. }
        ));
        let stalled = [w(1, 103.0), w(8, 103.0), w(15, 103.1), w(22, 103.0)];
        assert!(matches!(
            plateau_check(&stalled, today),
            PlateauAdvice::Stalled {
                suggest_kcal: -150,
                ..
            }
        ));
        assert_eq!(
            plateau_check(&dropping[..2], today),
            PlateauAdvice::NotEnoughData
        );
        let ma = moving_average_7d(&[w(1, 104.0), w(4, 103.0)]);
        assert!((ma[1].1 - 103.5).abs() < 1e-9);
    }
}
