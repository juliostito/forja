//! `Forja`: the facade every app (desktop, iOS through FFI, web) talks to.
//! Every method takes `now` explicitly — the time simulator just passes a different clock.

use crate::catalog::{exercise, RoutineItem};
use crate::habits::{habits_met, steps_goal, Habit};
use crate::meal::{Meal, MealEstimate};
use crate::model::*;
use crate::nutrition::{plateau_check, targets, NutritionTargets, PlateauAdvice};
use crate::plan::{
    phase_info, plan_session, rest_day_routine, DayKind, PhaseInfo, SessionPlan, SessionVariant,
    WeekTemplate,
};
use crate::profile::{Contract, Profile};
use crate::progression::{replay, ExerciseState, ProgressEvent, SessionSets, SetResult};
use crate::review::{weekly_review, WeekSummary};
use crate::safety::{safety_status, PainLight, SafetyStatus};
use crate::store::{Error, Result, Store};
use crate::streak::{can_recover, evaluate, heat_before, ForgeReport, Revival};
use crate::time::{add_days, local_date, local_time, week_monday};
use crate::units::Grams;
use crate::xp::*;
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub struct Forja {
    store: Store,
    profile: Profile,
    template: WeekTemplate,
    source: Source,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub enum Alert {
    /// Training time passed and nothing is logged yet (pulsing red).
    LateForTraining {
        since: NaiveTime,
    },
    /// "Fallaste ayer: nunca dos veces seguidas".
    MissedRecently {
        missed: NaiveDate,
        can_recover_today: bool,
    },
    /// The "Reavivar la forja" quest is open.
    ReviveForge(Revival),
    /// Missed twice in a row: show the contract full screen.
    WeaponCracked,
    RedFlags(Vec<String>),
    Pain(PainLight),
    Deload,
    StrengthTestWeek,
    ContractMissing,
}

#[derive(Clone, Debug, Serialize)]
pub struct TodayView {
    pub date: NaiveDate,
    pub now_local: NaiveTime,
    pub day: DayKind,
    pub phase: PhaseInfo,
    /// Today's full session (training days, or a recovery being done today).
    pub plan: Option<SessionPlan>,
    pub minimum_plan: Option<SessionPlan>,
    /// A missed session that can be recovered today.
    pub recoverable: Option<NaiveDate>,
    pub rest_routine: Vec<RoutineItem>,
    pub session: Option<SessionLog>,
    pub sets: Vec<SetLog>,
    pub alerts: Vec<Alert>,
    pub forge: ForgeReport,
    pub level: LevelInfo,
    pub steps_goal: u32,
    pub nutrition: NutritionTargets,
    pub protein_today: u32,
    /// Meals logged today (photo or manual) and their total kcal.
    pub meals: Vec<Meal>,
    pub kcal_today: u32,
    pub habits: HabitDay,
    pub habits_met: Vec<Habit>,
    pub safety: SafetyStatus,
    pub contract_signed: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct SessionOutcome {
    pub xp_gained: u32,
    pub level_before: LevelInfo,
    pub level_after: LevelInfo,
    pub progress: Vec<(String, ProgressEvent)>,
    pub counts_for_boss: bool,
    pub revived_forge: bool,
    pub forge: ForgeReport,
}

impl Forja {
    /// Opens the store; creates the preloaded profile (section 2) on first run. Records start empty.
    pub fn open(store: Store, onboarding: NaiveDate, source: Source) -> Result<Self> {
        let profile = match store.load_profile()? {
            Some(p) => p,
            None => {
                let p = Profile::initial(onboarding);
                store.save_profile(&p)?;
                p
            }
        };
        let template = WeekTemplate::from_profile(&profile);
        Ok(Forja {
            store,
            profile,
            template,
            source,
        })
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }

    pub fn template(&self) -> &WeekTemplate {
        &self.template
    }

    pub fn update_profile(&mut self, p: Profile) -> Result<()> {
        self.store.save_profile(&p)?;
        self.template = WeekTemplate::from_profile(&p);
        self.profile = p;
        Ok(())
    }

    pub fn sign_contract(&self, c: &Contract) -> Result<()> {
        if c.why.trim().is_empty() {
            return Err(Error::Invalid("Escribe tu porqué antes de firmar.".into()));
        }
        self.store.save_contract(c)
    }

    fn activity_days(&self) -> Result<BTreeSet<NaiveDate>> {
        let mut set = BTreeSet::new();
        for h in self.store.habit_days()? {
            let any = h.steps.is_some_and(|s| s > 0)
                || h.water
                || h.vegetables
                || h.no_sugar_alcohol
                || h.mcgill_big3
                || h.sleep_7h;
            if let (true, Some(d)) = (any, h.date) {
                set.insert(d);
            }
        }
        set.extend(self.store.protein_entries()?.into_iter().map(|p| p.date));
        set.extend(self.store.weights()?.into_iter().map(|w| w.date));
        set.extend(self.store.meals()?.into_iter().map(|m| m.date));
        Ok(set)
    }

    pub fn forge(&self, today: NaiveDate) -> Result<ForgeReport> {
        Ok(evaluate(
            &self.template,
            self.profile.tracking_start,
            &self.store.sessions()?,
            &self.activity_days()?,
            today,
        ))
    }

    pub fn level(&self) -> Result<LevelInfo> {
        Ok(level_for(self.store.xp_total()?))
    }

    fn drives_progression(&self, s: &SessionLog) -> bool {
        s.is_closed()
            && matches!(s.session_type, SessionType::Full | SessionType::Recovery)
            && phase_info(self.profile.program_start, s.date).phase != crate::plan::Phase::Deload
    }

    fn history(&self, include: impl Fn(&SessionLog) -> bool) -> Result<Vec<SessionSets>> {
        let sets = self.store.all_sets()?;
        let mut by_session: BTreeMap<i64, Vec<(String, SetResult)>> = BTreeMap::new();
        for s in sets {
            by_session.entry(s.session_id).or_default().push((
                s.exercise_id,
                SetResult {
                    reps: s.reps,
                    load: s.load,
                    ok: s.ok,
                },
            ));
        }
        Ok(self
            .store
            .sessions()?
            .into_iter()
            .filter(|s| include(s))
            .map(|s| SessionSets {
                drives_progression: self.drives_progression(&s),
                sets: by_session.remove(&s.id).unwrap_or_default(),
            })
            .collect())
    }

    /// Current prescriptions from all closed sessions before `date`.
    pub fn progression_states(&self, date: NaiveDate) -> Result<BTreeMap<String, ExerciseState>> {
        let h = self.history(|s| s.date < date)?;
        Ok(replay(&self.profile.equipment, &h).states)
    }

    pub fn safety(&self, today: NaiveDate) -> Result<SafetyStatus> {
        let earlier: Vec<SessionLog> = self
            .store
            .sessions()?
            .into_iter()
            .filter(|s| s.date < today)
            .collect();
        let checks = self.store.back_checks()?;
        let last = checks
            .iter()
            .rev()
            .find(|c| c.date.is_some_and(|d| d <= today));
        Ok(safety_status(&earlier, last, today))
    }

    pub fn plan(
        &self,
        date: NaiveDate,
        kind: SessionKind,
        variant: SessionVariant,
    ) -> Result<SessionPlan> {
        Ok(plan_session(
            &self.profile,
            &self.progression_states(date)?,
            &self.safety(date)?,
            date,
            kind,
            variant,
        ))
    }

    pub fn session_on(&self, date: NaiveDate) -> Result<Option<SessionLog>> {
        Ok(self
            .store
            .sessions()?
            .into_iter()
            .rfind(|s| s.date == date && s.session_type != SessionType::Skipped))
    }

    pub fn open_session(&self) -> Result<Option<SessionLog>> {
        Ok(self.store.sessions()?.into_iter().rfind(|s| !s.is_closed()))
    }

    fn new_session(
        &self,
        now: DateTime<Utc>,
        kind: SessionKind,
        ty: SessionType,
        from: Option<NaiveDate>,
    ) -> SessionLog {
        SessionLog {
            id: 0,
            date: local_date(now),
            kind,
            session_type: ty,
            recovered_from: from,
            started_at: now,
            ended_at: None,
            pain: None,
            effort: None,
            note: String::new(),
            source: self.source,
            warmup_done: false,
            stretches_done: false,
        }
    }

    pub fn start_session(&mut self, now: DateTime<Utc>, variant: SessionVariant) -> Result<i64> {
        let date = local_date(now);
        let DayKind::Training(kind) = self.template.day(date) else {
            return Err(Error::Invalid(
                "Hoy no toca entrenar. Si fallaste una sesión, puedes recuperarla.".into(),
            ));
        };
        if self.open_session()?.is_some() {
            return Err(Error::Invalid("Ya tienes una sesión abierta.".into()));
        }
        if self.session_on(date)?.is_some_and(|s| s.is_done()) {
            return Err(Error::Invalid("La sesión de hoy ya está hecha.".into()));
        }
        let ty = match variant {
            SessionVariant::Full => SessionType::Full,
            SessionVariant::Minimum => SessionType::Minimum,
        };
        self.store
            .insert_session(&self.new_session(now, kind, ty, None))
    }

    pub fn start_recovery(&mut self, now: DateTime<Utc>, missed: NaiveDate) -> Result<i64> {
        let date = local_date(now);
        let report = self.forge(date)?;
        if !can_recover(&self.template, &report, missed, date) {
            return Err(Error::Invalid(
                "Solo puedes recuperar una sesión fallada en un día de descanso dentro de los 2 días siguientes.".into(),
            ));
        }
        let DayKind::Training(kind) = self.template.day(missed) else {
            return Err(Error::Invalid("Ese día no era de entrenamiento.".into()));
        };
        if self.open_session()?.is_some() {
            return Err(Error::Invalid("Ya tienes una sesión abierta.".into()));
        }
        self.store
            .insert_session(&self.new_session(now, kind, SessionType::Recovery, Some(missed)))
    }

    fn plan_for_session(&self, s: &SessionLog) -> Result<SessionPlan> {
        let variant = if s.session_type == SessionType::Minimum {
            SessionVariant::Minimum
        } else {
            SessionVariant::Full
        };
        self.plan(s.date, s.kind, variant)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn log_set(
        &self,
        session_id: i64,
        exercise_id: &str,
        index: u8,
        reps: u16,
        load: Grams,
        ok: bool,
        watch_verified: bool,
    ) -> Result<()> {
        let s = self
            .store
            .session(session_id)?
            .ok_or_else(|| Error::Invalid("Sesión inexistente.".into()))?;
        if s.is_closed() {
            return Err(Error::Invalid("La sesión ya está cerrada.".into()));
        }
        if exercise(exercise_id).is_none() {
            return Err(Error::Invalid(format!(
                "Ejercicio desconocido: {exercise_id}"
            )));
        }
        self.store.upsert_set(&SetLog {
            session_id,
            exercise_id: exercise_id.to_string(),
            index,
            reps,
            load,
            ok,
            watch_verified,
        })
    }

    pub fn unlog_set(&self, session_id: i64, exercise_id: &str, index: u8) -> Result<()> {
        self.store.delete_set(session_id, exercise_id, index)
    }

    pub fn set_routines_done(
        &self,
        session_id: i64,
        warmup: Option<bool>,
        stretches: Option<bool>,
    ) -> Result<()> {
        let mut s = self
            .store
            .session(session_id)?
            .ok_or_else(|| Error::Invalid("Sesión inexistente.".into()))?;
        if let Some(w) = warmup {
            s.warmup_done = w;
        }
        if let Some(st) = stretches {
            s.stretches_done = st;
        }
        self.store.update_session(&s)
    }

    pub fn close_session(
        &mut self,
        session_id: i64,
        now: DateTime<Utc>,
        pain: u8,
        effort: u8,
        note: &str,
    ) -> Result<SessionOutcome> {
        let mut s = self
            .store
            .session(session_id)?
            .ok_or_else(|| Error::Invalid("Sesión inexistente.".into()))?;
        if s.is_closed() {
            return Err(Error::Invalid("La sesión ya está cerrada.".into()));
        }
        if pain > 10 || effort > 10 {
            return Err(Error::Invalid("Dolor y esfuerzo van de 0 a 10.".into()));
        }
        let plan = self.plan_for_session(&s)?;
        let sets = self.store.sets_for_session(session_id)?;
        let mut ok_sets = 0u32;
        for e in &plan.exercises {
            let n = sets
                .iter()
                .filter(|x| x.exercise_id == e.exercise_id && x.ok)
                .count() as u32;
            ok_sets += n.min(u32::from(e.sets));
        }
        let level_before = self.level()?;
        let forge_before = self.forge(s.date)?;
        let history_before = self.history(|x| x.is_closed())?;
        let events_before = replay(&self.profile.equipment, &history_before)
            .events
            .len();
        let heat = heat_before(
            &self.template,
            self.profile.tracking_start,
            &self.store.sessions()?,
            &self.activity_days()?,
            s.date,
            s.id,
        );
        let xp = session_xp(SessionXpInput {
            session_type: s.session_type,
            ok_sets,
            planned_sets: plan.total_sets(),
            warmup_done: s.warmup_done,
            stretches_done: s.stretches_done,
            duration_minutes: (now - s.started_at).num_minutes(),
            heat_at_start: heat,
            watch_verified: sets.iter().any(|x| x.watch_verified),
        });

        s.ended_at = Some(now);
        s.pain = Some(pain);
        s.effort = Some(effort);
        s.note = note.trim().to_string();
        self.store.update_session(&s)?;

        let mut gained = 0;
        for (reason, base) in xp.parts {
            if base == 0 {
                continue;
            }
            let mut e = XpEvent::new(now, s.date, reason, base, xp.multiplier_pct);
            e.ref_id = Some(s.id);
            if !xp.counts_for_boss {
                e.note = "reduced".into();
            }
            gained += e.amount;
            self.store.add_xp_event(&e)?;
        }

        let after = replay(&self.profile.equipment, &self.history(|x| x.is_closed())?);
        let forge = self.forge(s.date)?;
        Ok(SessionOutcome {
            xp_gained: gained,
            level_before,
            level_after: self.level()?,
            progress: after.events.into_iter().skip(events_before).collect(),
            counts_for_boss: xp.counts_for_boss,
            revived_forge: forge.revivals > forge_before.revivals,
            forge,
        })
    }

    /// Skipping needs a written reason (≥ 25 characters). The UI shows the contract's "why" and offers the
    /// minimum version first.
    pub fn skip_session(&mut self, now: DateTime<Utc>, reason: &str) -> Result<()> {
        validate_skip_reason(reason).map_err(Error::Invalid)?;
        let date = local_date(now);
        let DayKind::Training(kind) = self.template.day(date) else {
            return Err(Error::Invalid("Hoy no toca entrenar.".into()));
        };
        if self.session_on(date)?.is_some() {
            return Err(Error::Invalid("Ya registraste una sesión hoy.".into()));
        }
        let mut s = self.new_session(now, kind, SessionType::Skipped, None);
        s.ended_at = Some(now);
        s.note = reason.trim().to_string();
        self.store.insert_session(&s)?;
        Ok(())
    }

    fn award(
        &self,
        now: DateTime<Utc>,
        date: NaiveDate,
        reason: XpReason,
        base: u32,
        note: &str,
    ) -> Result<XpEvent> {
        let heat = self.forge(date)?.heat;
        let mut e = XpEvent::new(now, date, reason, base, heat_multiplier_pct(heat));
        e.note = note.to_string();
        e.id = self.store.add_xp_event(&e)?;
        Ok(e)
    }

    /// Awards habit XP once per habit per day (+25 for a perfect day). Unticking never takes XP back:
    /// the ledger is append-only, and re-ticking doesn't pay twice.
    fn award_habits(&self, now: DateTime<Utc>, date: NaiveDate) -> Result<Vec<XpEvent>> {
        let day = self.store.habit_day(date)?.unwrap_or_default();
        let goal = steps_goal(phase_info(self.profile.program_start, date).program_week);
        let protein = self.store.protein_for(date)?;
        let met = habits_met(&day, goal, protein, self.nutrition()?.protein_g);
        let mut out = Vec::new();
        for h in &met {
            if !self.store.has_xp_event(date, XpReason::Habit, h.id())? {
                out.push(self.award(now, date, XpReason::Habit, XP_HABIT, h.id())?);
            }
        }
        if met.len() == Habit::ALL.len()
            && !self.store.has_xp_event(date, XpReason::PerfectHabits, "")?
        {
            out.push(self.award(now, date, XpReason::PerfectHabits, XP_PERFECT_HABITS, "")?);
        }
        Ok(out)
    }

    pub fn update_habits(&self, now: DateTime<Utc>, day: &HabitDay) -> Result<Vec<XpEvent>> {
        let date = local_date(now);
        self.store.upsert_habit_day(date, day)?;
        self.award_habits(now, date)
    }

    /// Logs a meal estimated from a photo (Claude). Its protein also counts for today's protein habit.
    pub fn log_meal(
        &self,
        now: DateTime<Utc>,
        estimate: &MealEstimate,
        photo_ref: Option<String>,
    ) -> Result<(Meal, Vec<XpEvent>)> {
        let date = local_date(now);
        let mut meal = Meal::from_estimate(date, now, estimate, photo_ref);
        meal.id = self.store.add_meal(&meal)?;
        let mut xp = Vec::new();
        if meal.protein_g > 0 {
            xp = self.add_protein(
                now,
                meal.protein_g,
                &format!("Comida: {}", meal.description),
            )?;
        }
        Ok((meal, xp))
    }

    pub fn add_protein(&self, now: DateTime<Utc>, grams: u32, label: &str) -> Result<Vec<XpEvent>> {
        let date = local_date(now);
        self.store.add_protein(&ProteinEntry {
            date,
            at: now,
            grams,
            label: label.to_string(),
        })?;
        self.award_habits(now, date)
    }

    pub fn log_weight(&self, now: DateTime<Utc>, weight: Grams) -> Result<Vec<XpEvent>> {
        if !(30_000..=300_000).contains(&weight.0) {
            return Err(Error::Invalid(
                "Revisa el peso: debe estar entre 30 y 300 kg.".into(),
            ));
        }
        let date = local_date(now);
        self.store.upsert_weight(&WeightEntry {
            date,
            weight,
            at: now,
        })?;
        let mut out = Vec::new();
        if !self.store.has_xp_event(date, XpReason::WeighIn, "")? {
            out.push(self.award(now, date, XpReason::WeighIn, XP_WEIGH_IN, "")?);
        }
        Ok(out)
    }

    pub fn record_strength_test(
        &self,
        now: DateTime<Utc>,
        mut t: StrengthTest,
    ) -> Result<Vec<XpEvent>> {
        let date = local_date(now);
        t.date = Some(date);
        self.store.add_strength_test(&t)?;
        if let Some(w) = t.weight {
            self.store.upsert_weight(&WeightEntry {
                date,
                weight: w,
                at: now,
            })?;
        }
        let mut out = Vec::new();
        if !self.store.has_xp_event(date, XpReason::StrengthTest, "")? {
            out.push(self.award(now, date, XpReason::StrengthTest, XP_STRENGTH_TEST, "")?);
        }
        Ok(out)
    }

    pub fn log_back_check(&self, date: NaiveDate, mut c: BackCheck) -> Result<()> {
        c.date = Some(date);
        self.store.upsert_back_check(&c)
    }

    pub fn latest_weight(&self) -> Result<Option<Grams>> {
        Ok(self.store.weights()?.last().map(|w| w.weight))
    }

    pub fn nutrition(&self) -> Result<NutritionTargets> {
        Ok(targets(&self.profile, self.latest_weight()?))
    }

    /// The user accepts the plateau suggestion (−150 kcal).
    pub fn accept_plateau_adjustment(&mut self, today: NaiveDate) -> Result<bool> {
        if let PlateauAdvice::Stalled { suggest_kcal, .. } =
            plateau_check(&self.store.weights()?, today)
        {
            let mut p = self.profile.clone();
            p.kcal_adjustment += suggest_kcal;
            self.update_profile(p)?;
            return Ok(true);
        }
        Ok(false)
    }

    pub fn week_summary(&self, week_start: NaiveDate, today: NaiveDate) -> Result<WeekSummary> {
        let ws = week_monday(week_start);
        let we = add_days(ws, 6);
        let in_week = |d: NaiveDate| d >= ws && d <= we;
        let sessions = self.store.sessions()?;
        let done: Vec<&SessionLog> = sessions
            .iter()
            .filter(|s| s.is_done() && in_week(s.date))
            .collect();
        let forge = self.forge(today.min(add_days(we, 1)))?;
        let mut planned = 0;
        let mut remaining = 0;
        let mut d = ws;
        while d <= we {
            if d >= self.profile.tracking_start && self.template.day(d).is_training() {
                planned += 1;
                if d >= today && !done.iter().any(|s| s.date == d) {
                    remaining += 1;
                }
            }
            d = add_days(d, 1);
        }
        let pains: Vec<u8> = done.iter().filter_map(|s| s.pain).collect();
        let weights = self.store.weights()?;
        let avg = |a: NaiveDate, b: NaiveDate| {
            let v: Vec<f64> = weights
                .iter()
                .filter(|w| w.date >= a && w.date <= b)
                .map(|w| w.weight.kg())
                .collect();
            (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
        };
        let target = self.nutrition()?.protein_g;
        let mut protein_days = 0;
        let mut d = ws;
        while d <= we {
            if self.store.protein_for(d)? >= target {
                protein_days += 1;
            }
            d = add_days(d, 1);
        }
        let events = self.store.xp_events()?;
        Ok(WeekSummary {
            in_progress: today < we && remaining > 0,
            remaining,
            week_start: ws,
            phase: phase_info(self.profile.program_start, ws),
            next_phase: phase_info(self.profile.program_start, add_days(ws, 7)),
            planned,
            done: done.len() as u32,
            minimum_versions: done
                .iter()
                .filter(|s| s.session_type == SessionType::Minimum)
                .count() as u32,
            recoveries: done
                .iter()
                .filter(|s| s.session_type == SessionType::Recovery)
                .count() as u32,
            misses: forge
                .misses
                .iter()
                .filter(|m| in_week(**m) && !forge.recovered.contains(m))
                .count() as u32,
            avg_pain: (!pains.is_empty())
                .then(|| pains.iter().map(|p| f64::from(*p)).sum::<f64>() / pains.len() as f64),
            max_pain: pains.iter().copied().max(),
            weight_avg_kg: avg(ws, we),
            weight_avg_prev_kg: avg(add_days(ws, -7), add_days(ws, -1)),
            protein_days,
            perfect_habit_days: events
                .iter()
                .filter(|e| e.reason == XpReason::PerfectHabits && in_week(e.date))
                .count() as u32,
            xp_gained: events
                .iter()
                .filter(|e| in_week(e.date))
                .map(|e| e.amount)
                .sum(),
            streak: forge.current,
            plateau: plateau_check(&weights, today.min(we)),
            red_flags: self.safety(today)?.red_flags,
        })
    }

    pub fn weekly_review(&self, week_start: NaiveDate, today: NaiveDate) -> Result<Vec<String>> {
        Ok(weekly_review(&self.week_summary(week_start, today)?))
    }

    pub fn today(&self, now: DateTime<Utc>) -> Result<TodayView> {
        let date = local_date(now);
        let now_local = local_time(now);
        let day = self.template.day(date);
        let phase = phase_info(self.profile.program_start, date);
        let forge = self.forge(date)?;
        let safety = self.safety(date)?;
        let session = self
            .open_session()?
            .or_else(|| self.session_on(date).ok().flatten());
        let sets = match &session {
            Some(s) => self.store.sets_for_session(s.id)?,
            None => Vec::new(),
        };
        let recoverable = forge
            .misses
            .iter()
            .rev()
            .copied()
            .find(|m| can_recover(&self.template, &forge, *m, date));

        let (plan, minimum_plan) = match (&session, day) {
            (Some(s), _) => (Some(self.plan_for_session(s)?), None),
            (None, DayKind::Training(kind)) => (
                Some(self.plan(date, kind, SessionVariant::Full)?),
                Some(self.plan(date, kind, SessionVariant::Minimum)?),
            ),
            (None, _) => (None, None),
        };

        let mut alerts = Vec::new();
        let done_today = session.as_ref().is_some_and(|s| s.is_done());
        if day.is_training() && session.is_none() && now_local >= self.profile.training_time {
            alerts.push(Alert::LateForTraining {
                since: self.profile.training_time,
            });
        }
        if let Some(m) = forge
            .misses
            .iter()
            .rev()
            .find(|m| forge.is_unrecovered_miss(**m))
        {
            if (date - *m).num_days() <= 2 && !done_today {
                alerts.push(Alert::MissedRecently {
                    missed: *m,
                    can_recover_today: recoverable == Some(*m),
                });
            }
        }
        if let Some(p) = forge.pending {
            alerts.push(Alert::ReviveForge(p));
        }
        if forge.cracked {
            alerts.push(Alert::WeaponCracked);
        }
        if safety.red_flags {
            alerts.push(Alert::RedFlags(safety.red_flag_reasons.clone()));
        }
        if safety.light != PainLight::Green {
            alerts.push(Alert::Pain(safety.light));
        }
        if phase.phase == crate::plan::Phase::Deload {
            alerts.push(Alert::Deload);
        }
        if phase.strength_test_week() {
            alerts.push(Alert::StrengthTestWeek);
        }
        let contract_signed = self.store.load_contract()?.is_some_and(|c| c.is_signed());
        if !contract_signed {
            alerts.push(Alert::ContractMissing);
        }

        let nutrition = self.nutrition()?;
        let protein_today = self.store.protein_for(date)?;
        let meals = self.store.meals_on(date)?;
        let kcal_today = meals.iter().map(|m| m.kcal).sum();
        let habits = self.store.habit_day(date)?.unwrap_or_default();
        let goal = steps_goal(phase.program_week);
        Ok(TodayView {
            date,
            now_local,
            day,
            phase,
            plan,
            minimum_plan,
            recoverable,
            rest_routine: if day.is_training() {
                Vec::new()
            } else {
                rest_day_routine()
            },
            session,
            sets,
            alerts,
            forge,
            level: self.level()?,
            steps_goal: goal,
            habits_met: habits_met(&habits, goal, protein_today, nutrition.protein_g),
            nutrition,
            protein_today,
            meals,
            kcal_today,
            habits,
            safety,
            contract_signed,
        })
    }
}
