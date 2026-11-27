//! 12-week simulations through the public facade, exactly as an app would drive it.

use chrono::{Datelike, NaiveDate, NaiveTime, Weekday};
use forja_core::forja::{Alert, Forja};
use forja_core::model::{HabitDay, SessionType, Source, StrengthTest};
use forja_core::plan::{DayKind, Phase, SessionVariant};
use forja_core::profile::LoadMode;
use forja_core::progression::ProgressEvent;
use forja_core::store::Store;
use forja_core::time::{add_days, resolve_local};
use forja_core::units::Grams;

fn at(date: NaiveDate, h: u32, m: u32) -> chrono::DateTime<chrono::Utc> {
    resolve_local(date, NaiveTime::from_hms_opt(h, m, 0).unwrap())
}

fn start() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 30).unwrap() // a Monday
}

/// Does today's full session, every set at the top of the range with the suggested load.
fn train(f: &mut Forja, date: NaiveDate) -> forja_core::SessionOutcome {
    let id = f
        .start_session(at(date, 7, 0), SessionVariant::Full)
        .unwrap();
    f.set_routines_done(id, Some(true), None).unwrap();
    let plan = f.today(at(date, 7, 1)).unwrap().plan.unwrap();
    for e in &plan.exercises {
        for i in 0..e.sets {
            f.log_set(
                id,
                e.exercise_id,
                i,
                e.max,
                e.load.unwrap_or(Grams::ZERO),
                true,
                false,
            )
            .unwrap();
        }
    }
    f.set_routines_done(id, None, Some(true)).unwrap();
    f.close_session(id, at(date, 7, 50), 1, 7, "").unwrap()
}

fn habits(f: &Forja, date: NaiveDate) {
    let goal = f.today(at(date, 6, 30)).unwrap().steps_goal;
    f.update_habits(
        at(date, 21, 0),
        &HabitDay {
            date: None,
            steps: Some(goal),
            water: true,
            vegetables: true,
            no_sugar_alcohol: true,
            mcgill_big3: true,
            sleep_7h: true,
        },
    )
    .unwrap();
    let target = f.nutrition().unwrap().protein_g;
    let mut eaten = 0;
    while eaten < target {
        f.add_protein(at(date, 13, 0), 31, "Pechuga 100 g").unwrap();
        eaten += 31;
    }
}

#[test]
fn disciplined_user_levels_up_enters_phase_2_and_deloads_in_week_9() {
    let mut f = Forja::open(
        Store::open_in_memory().unwrap(),
        start(),
        Source::Simulation,
    )
    .unwrap();
    let initial_kcal = f.nutrition().unwrap().kcal;
    let initial_bench = f.today(at(start(), 6, 0)).unwrap().plan.unwrap().exercises[2]
        .load
        .unwrap();
    let mut weight = 104.0;
    let mut saw_deload_week9 = false;
    let mut saw_variants_week10 = false;
    let mut load_ups = 0;
    let mut first_level_up = None;

    for offset in 0..84 {
        let date = add_days(start(), offset);
        let today = f.today(at(date, 6, 45)).unwrap();
        let week = today.phase.program_week;
        if let DayKind::Training(_) = today.day {
            let plan = today.plan.clone().unwrap();
            if week == 9 {
                assert_eq!(today.phase.phase, Phase::Deload);
                assert!(today.alerts.contains(&Alert::Deload));
                let full_sets: u32 = plan
                    .exercises
                    .iter()
                    .map(|e| u32::from(e.exercise().sets))
                    .sum();
                assert!(plan.total_sets() < full_sets, "deload halves the sets");
                saw_deload_week9 = true;
            }
            if week == 10 {
                assert_eq!(today.phase.phase, Phase::Variants);
                assert!(plan
                    .exercises
                    .iter()
                    .any(|e| e.exercise().variant_of.is_some()));
                saw_variants_week10 = true;
            }
            let out = train(&mut f, date);
            assert!(out.counts_for_boss);
            load_ups += out
                .progress
                .iter()
                .filter(|(_, e)| matches!(e, ProgressEvent::LoadUp { .. }))
                .count();
            if first_level_up.is_none() && out.level_after.level > out.level_before.level {
                first_level_up = Some(offset);
            }
        }
        habits(&f, date);
        if date.weekday() == Weekday::Sun {
            weight -= 0.5;
            f.log_weight(at(date, 8, 0), Grams::from_kg(weight))
                .unwrap();
            if week.is_multiple_of(4) {
                f.record_strength_test(
                    at(date, 9, 0),
                    StrengthTest {
                        pushups_max: Some(10 + week),
                        box_squats_60s: Some(20 + week),
                        ..Default::default()
                    },
                )
                .unwrap();
            }
            let review = f.weekly_review(date, date).unwrap();
            assert!(!review.is_empty());
            if week == 8 {
                assert!(review.iter().any(|l| l.contains("descarga")), "{review:?}");
            }
            if week == 9 {
                assert!(review.iter().any(|l| l.contains("Fase 2")), "{review:?}");
            }
        }
    }

    let end = add_days(start(), 84);
    let forge = f.forge(end).unwrap();
    let level = f.level().unwrap();
    println!(
        "level {} ({}), xp {}, heat {}, streak {}",
        level.level, level.title, level.total_xp, forge.heat, forge.current
    );

    assert!(saw_deload_week9 && saw_variants_week10);
    assert_eq!(
        first_level_up,
        Some(0),
        "level 2 with the very first session"
    );
    assert_eq!(forge.current, 48);
    assert_eq!(forge.total_done, 48);
    assert!(forge.misses.is_empty() && forge.breaks.is_empty());
    assert_eq!(
        forge.flame_tier(),
        5,
        "48 sessions pass the 1, 4, 8, 16 and 24 tiers"
    );
    assert!(forge.heat >= 90, "al rojo vivo");
    assert!(
        level.level >= 10,
        "a disciplined quarter reaches Oficial de fragua, got {}",
        level.level
    );
    assert!(load_ups >= 5, "loads went up {load_ups} times");

    // Loads are always buildable with the home equipment.
    let states = f.progression_states(end).unwrap();
    let eq = &f.profile().equipment;
    for (id, s) in &states {
        let ex = forja_core::catalog::exercise(id).unwrap();
        if ex.load != LoadMode::Bodyweight {
            assert!(eq.achievable(ex.load).contains(&s.load), "{id}: {}", s.load);
        }
    }
    assert!(states["db_bench_press"].load > initial_bench);
    assert!(states["db_bench_press"].load <= eq.max(LoadMode::Pair));

    // Lower weight → lower kcal target.
    assert!(f.nutrition().unwrap().kcal < initial_kcal);

    // A full backup restores the same level and streak.
    let backup = f.store().export_backup().unwrap();
    let mut restored = Store::open_in_memory().unwrap();
    restored.import_backup(&backup).unwrap();
    let g = Forja::open(restored, start(), Source::Import).unwrap();
    assert_eq!(g.level().unwrap(), level);
    assert_eq!(g.forge(end).unwrap().current, 48);
}

#[test]
fn never_miss_twice_through_the_facade() {
    let mut f = Forja::open(
        Store::open_in_memory().unwrap(),
        start(),
        Source::Simulation,
    )
    .unwrap();
    let mon = start();
    let tue = add_days(mon, 1);
    let wed = add_days(mon, 2);
    train(&mut f, mon);

    // Tuesday: late alert after 07:00, skipping needs a real reason.
    let t = f.today(at(tue, 7, 30)).unwrap();
    assert!(t
        .alerts
        .iter()
        .any(|a| matches!(a, Alert::LateForTraining { .. })));
    assert!(f.skip_session(at(tue, 8, 0), "flojera").is_err());
    f.skip_session(at(tue, 8, 0), "Turno de noche en la pega, no dormí nada")
        .unwrap();

    // Wednesday: "fallaste ayer" with recovery available on a rest day.
    let w = f.today(at(wed, 9, 0)).unwrap();
    assert_eq!(w.recoverable, Some(tue));
    assert!(w.alerts.iter().any(|a| matches!(
        a,
        Alert::MissedRecently {
            can_recover_today: true,
            ..
        }
    )));
    assert!(w.alerts.iter().any(|a| matches!(a, Alert::ReviveForge(_))));
    assert!(
        f.start_session(at(wed, 9, 0), SessionVariant::Full)
            .is_err(),
        "Wednesday isn't a training day"
    );
    let id = f.start_recovery(at(wed, 9, 0), tue).unwrap();
    let plan = f.today(at(wed, 9, 1)).unwrap().plan.unwrap();
    assert_eq!(plan.kind, forja_core::model::SessionKind::LowerA);
    for e in &plan.exercises {
        for i in 0..e.sets {
            f.log_set(
                id,
                e.exercise_id,
                i,
                e.min,
                e.load.unwrap_or(Grams::ZERO),
                true,
                false,
            )
            .unwrap();
        }
    }
    let out = f
        .close_session(id, at(wed, 9, 45), 2, 6, "recuperada")
        .unwrap();
    assert!(out.revived_forge);
    assert_eq!(out.forge.current, 2);
    assert!(out.forge.heat >= 50);
    assert!(out.xp_gained > 0);
    let rec = f
        .store()
        .sessions()
        .unwrap()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    assert_eq!(rec.session_type, SessionType::Recovery);

    // Thursday and Friday missed → weapon cracked, contract shown.
    let sat = add_days(mon, 5);
    let s = f.today(at(sat, 10, 0)).unwrap();
    assert_eq!(s.forge.current, 0);
    assert!(s.alerts.contains(&Alert::WeaponCracked));
    assert!(!s.forge.breaks.is_empty());
}

#[test]
fn back_pain_above_3_swaps_exercises_next_session() {
    let mut f = Forja::open(
        Store::open_in_memory().unwrap(),
        start(),
        Source::Simulation,
    )
    .unwrap();
    let mon = start();
    let id = f
        .start_session(at(mon, 7, 0), SessionVariant::Full)
        .unwrap();
    f.log_set(id, "incline_pushup", 0, 10, Grams::ZERO, true, false)
        .unwrap();
    f.close_session(id, at(mon, 7, 40), 5, 6, "me molestó la espalda")
        .unwrap();
    let tue = f.today(at(add_days(mon, 1), 6, 45)).unwrap();
    let plan = tue.plan.unwrap();
    assert!(tue
        .alerts
        .contains(&Alert::Pain(forja_core::safety::PainLight::Red)));
    assert!(!plan
        .exercises
        .iter()
        .any(|e| e.exercise_id == "romanian_deadlift"));
    assert!(plan
        .exercises
        .iter()
        .any(|e| e.substituted_from == Some("goblet_box_squat")));
    // Red flags pause axial loading and show the message.
    f.log_back_check(
        add_days(mon, 1),
        forja_core::model::BackCheck {
            radiating_leg_pain: true,
            ..Default::default()
        },
    )
    .unwrap();
    let thu = f.today(at(add_days(mon, 3), 6, 45)).unwrap();
    assert!(thu.alerts.iter().any(|a| matches!(a, Alert::RedFlags(_))));
    assert!(!thu
        .plan
        .unwrap()
        .exercises
        .iter()
        .any(|e| e.exercise().axial_load));
}

#[test]
fn data_survives_reopening_the_file() {
    let path = std::env::temp_dir().join(format!("forja-test-{}.db", std::process::id()));
    let _ = std::fs::remove_file(&path);
    {
        let mut f = Forja::open(Store::open(&path).unwrap(), start(), Source::Simulation).unwrap();
        train(&mut f, start());
        f.log_weight(at(start(), 8, 30), Grams::from_kg(103.8))
            .unwrap();
    }
    let f = Forja::open(Store::open(&path).unwrap(), start(), Source::Simulation).unwrap();
    assert_eq!(f.store().sessions().unwrap().len(), 1);
    assert_eq!(f.latest_weight().unwrap(), Some(Grams(103_800)));
    assert!(f.level().unwrap().level >= 2);
    drop(f);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn app_starts_with_profile_and_empty_records() {
    let f = Forja::open(Store::open_in_memory().unwrap(), start(), Source::Phone).unwrap();
    let t = f.today(at(start(), 6, 0)).unwrap();
    assert_eq!(f.profile().start_weight, Grams(104_000));
    assert!(f.store().sessions().unwrap().is_empty());
    assert_eq!(t.level.level, 1);
    assert_eq!(t.level.total_xp, 0);
    assert!(t.alerts.contains(&Alert::ContractMissing));
    assert_eq!(t.nutrition.kcal, 2250);
    assert_eq!(t.steps_goal, 6000);
}

#[test]
fn photo_meal_is_logged_and_counts_protein() {
    let f = Forja::open(
        Store::open_in_memory().unwrap(),
        start(),
        Source::Simulation,
    )
    .unwrap();
    let answer = r#"{"plato": "Pollo con arroz y ensalada chilena", "items": [], "kcal": 620, "proteina_g": 45, "carbohidratos_g": 60, "grasas_g": 18, "confianza": "alta", "nota": "Buen plato."}"#;
    let e = forja_core::meal::parse_estimate(answer).unwrap();
    let (meal, _) = f
        .log_meal(at(start(), 13, 30), &e, Some("almuerzo.jpg".into()))
        .unwrap();
    assert!(meal.id > 0);
    let t = f.today(at(start(), 14, 0)).unwrap();
    assert_eq!(t.kcal_today, 620);
    assert_eq!(t.meals.len(), 1);
    assert_eq!(t.protein_today, 45);
    // The meal survives a backup.
    let b = f.store().export_backup().unwrap();
    assert_eq!(b.meals.len(), 1);
}
