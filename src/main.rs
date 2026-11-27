//! Forja — milestone 1 demo.
//!
//! A small desktop window driven entirely by `forja-core`: today's session with the real profile and
//! empty records (kept in memory, never saved), set logging with rest timer and sparks, closing a
//! session with pain/effort, the skip friction, habits, protein, weight and a developer time simulator.
//!
//! Options:
//!   --screenshot <file.png>  draw ~120 frames, save the window as PNG, exit (window stays in the back)
//!   --self-test <file.json>  play the core loop with scripted input, write results, exit

mod figure;
mod meals;
mod ui;

use forja_core::pose::{Detection, Detector, Movement, CONFIDENT};

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use forja_core::catalog::RoutineItem;
use forja_core::forja::{Alert, Forja, TodayView};
use forja_core::habits::Habit;
use forja_core::model::{HabitDay, Source};
use forja_core::nutrition::PROTEIN_QUICK;
use forja_core::plan::SessionVariant;
use forja_core::progression::ProgressEvent;
use forja_core::safety::PainLight;
use forja_core::store::Store;
use forja_core::time::{add_days, format_date_es, local_date, resolve_local};
use forja_core::units::{format_kg_number, format_one_decimal, group_thousands, Grams};
use forja_core::xp::{heat_label, heat_multiplier_pct};
use macroquad::prelude::*;
use std::collections::BTreeSet;
use ui::*;

const TITLE: &str = "Forja — demo del hito 1";

#[derive(Clone, Debug)]
enum Action {
    Start(SessionVariant),
    Recover(NaiveDate),
    ToggleSet(usize, u8),
    LessReps(usize, u8),
    NextSet,
    ToggleWarmup(usize),
    ToggleStretch(usize),
    Close,
    Pain(i32),
    Effort(i32),
    Expand(usize),
    OpenSkip,
    CancelSkip,
    ConfirmSkip,
    Protein(u32, &'static str),
    Habit(Habit),
    Steps(u32),
    Weight(i32),
    LogWeight,
    AddMinutes(i64),
    NextMorning,
    NextWeek,
    ToggleDetector(usize),
    MealPhoto,
    EditKey,
    SaveKey,
    CancelKey,
}

/// The exercise detector running on the animated figure of one planned exercise.
struct DetectorDemo {
    exercise: usize,
    id: &'static str,
    candidates: Vec<&'static str>,
    detector: Detector,
    clock: f32,
    since_update: f32,
    result: Option<Detection>,
}

impl DetectorDemo {
    fn summary(&self) -> String {
        match &self.result {
            None => "Observando…".into(),
            Some(d) if d.movement == Movement::Unknown => {
                "Aún no reconozco un movimiento claro…".into()
            }
            Some(d) => {
                let mut s = format!("Detectado: {} · {} reps", d.movement.label(), d.reps);
                if let Some(ok) = d.depth_ok {
                    s.push_str(if ok {
                        " · profundidad ok"
                    } else {
                        " · baja un poco más"
                    });
                }
                if d.posture_warning {
                    s.push_str(" · ojo con la espalda");
                }
                let conf = (d.confidence * 100.0).round();
                if d.confidence < CONFIDENT {
                    s.push_str(&format!(" · confianza {conf} %: confírmalo"));
                } else {
                    s.push_str(&format!(" · confianza {conf} %"));
                }
                s
            }
        }
    }
}

struct Spark {
    pos: Vec2,
    vel: Vec2,
    life: f32,
}

struct App {
    forja: Forja,
    base_clock: DateTime<Utc>,
    base_real: f64,
    frozen: bool,
    simulated: bool,
    view: TodayView,
    expanded: Option<usize>,
    rest: Option<(f32, f32)>,
    sparks: Vec<Spark>,
    flash: f32,
    toasts: Vec<(String, f32, Color)>,
    pain: u8,
    effort: u8,
    warm_done: Vec<bool>,
    stretch_done: Vec<bool>,
    skip_text: Option<String>,
    weight_input: u32,
    pending: Vec<Action>,
    hud: BTreeSet<&'static str>,
    scroll: f32,
    set_positions: Vec<(usize, u8, Vec2)>,
    last_outcome_xp: u32,
    review: Vec<String>,
    detector: Option<DetectorDemo>,
    hover_demo: Option<(&'static str, &'static str, Vec2)>,
    meal_rx: Option<std::sync::mpsc::Receiver<meals::Outcome>>,
    meal_busy: bool,
    key_input: Option<String>,
    /// After saving the key, go straight to the photo.
    photo_after_key: bool,
}

impl App {
    fn new(frozen: bool) -> Self {
        let clock = Utc::now();
        let store = Store::open_in_memory().expect("in-memory store");
        let forja = Forja::open(store, local_date(clock), Source::Desktop).expect("profile");
        let view = forja.today(clock).expect("today");
        let weight_input = forja.profile().start_weight.0;
        let mut app = App {
            forja,
            base_clock: clock,
            base_real: get_time(),
            frozen,
            simulated: false,
            view,
            expanded: Some(0),
            rest: None,
            sparks: Vec::new(),
            flash: 0.0,
            toasts: Vec::new(),
            pain: 0,
            effort: 6,
            warm_done: Vec::new(),
            stretch_done: Vec::new(),
            skip_text: None,
            weight_input,
            pending: Vec::new(),
            hud: BTreeSet::new(),
            scroll: 0.0,
            set_positions: Vec::new(),
            last_outcome_xp: 0,
            review: Vec::new(),
            detector: None,
            hover_demo: None,
            meal_rx: None,
            meal_busy: false,
            key_input: None,
            photo_after_key: false,
        };
        app.refresh();
        app
    }

    fn now(&self) -> DateTime<Utc> {
        if self.frozen {
            self.base_clock
        } else {
            self.base_clock
                + Duration::milliseconds(((get_time() - self.base_real) * 1000.0) as i64)
        }
    }

    fn set_clock(&mut self, t: DateTime<Utc>) {
        self.base_clock = t;
        self.base_real = get_time();
        self.simulated = true;
    }

    fn refresh(&mut self) {
        match self.forja.today(self.now()) {
            Ok(v) => self.view = v,
            Err(e) => self.toast(format!("Error: {e}"), ALERT),
        }
        self.review = self
            .forja
            .weekly_review(self.view.date, self.view.date)
            .unwrap_or_default();
        let open = self.open_session_id().is_some();
        let warm_n = self.view.plan.as_ref().map(|p| p.warmup.len()).unwrap_or(0);
        let str_n = self
            .view
            .plan
            .as_ref()
            .map(|p| p.stretches.len())
            .unwrap_or(0);
        if !open {
            self.warm_done = vec![false; warm_n];
            self.stretch_done = vec![false; str_n];
        } else {
            self.warm_done.resize(warm_n, false);
            self.stretch_done.resize(str_n, false);
        }
    }

    fn toast(&mut self, s: String, c: Color) {
        self.toasts.push((s, 4.0, c));
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    fn open_session_id(&self) -> Option<i64> {
        self.view
            .session
            .as_ref()
            .filter(|s| !s.is_closed())
            .map(|s| s.id)
    }

    fn burst(&mut self, at: Vec2, n: usize) {
        for i in 0..n {
            let spread = (i as f32 / n as f32 - 0.5) * 2.4 + rand::gen_range(-0.2, 0.2);
            let a = -std::f32::consts::FRAC_PI_2 + spread;
            let speed = rand::gen_range(140.0, 320.0);
            self.sparks.push(Spark {
                pos: at,
                vel: vec2(a.cos() * speed, a.sin() * speed),
                life: rand::gen_range(0.5, 0.9),
            });
        }
    }

    fn run_pending(&mut self) {
        let actions: Vec<Action> = std::mem::take(&mut self.pending);
        for a in actions {
            self.apply(a);
        }
    }

    fn next_open_set(&self) -> Option<(usize, u8)> {
        let plan = self.view.plan.as_ref()?;
        plan.exercises.iter().enumerate().find_map(|(ei, e)| {
            (0..e.sets)
                .find(|si| {
                    !self
                        .view
                        .sets
                        .iter()
                        .any(|s| s.exercise_id == e.exercise_id && s.index == *si)
                })
                .map(|si| (ei, si))
        })
    }

    fn apply(&mut self, a: Action) {
        if let Err(e) = self.try_apply(a) {
            self.toast(e, ALERT);
        }
        self.refresh();
    }

    fn try_apply(&mut self, a: Action) -> Result<(), String> {
        let now = self.now();
        let err = |e: forja_core::Error| e.to_string();
        match a {
            Action::Start(v) => {
                self.forja.start_session(now, v).map_err(err)?;
                self.skip_text = None;
                let msg = if v == SessionVariant::Minimum {
                    "Versión mínima: 15 min y la racha sigue viva."
                } else {
                    "¡A forjar!"
                };
                self.toast(msg.into(), EMBER2);
            }
            Action::Recover(d) => {
                self.forja.start_recovery(now, d).map_err(err)?;
                self.toast("Recuperando la sesión: cuenta como hecha.".into(), TEMPER);
            }
            Action::ToggleSet(ei, si) | Action::LessReps(ei, si) => {
                let id = self.open_session_id().ok_or("Primero empieza la sesión.")?;
                let plan = self.view.plan.as_ref().ok_or("Sin plan")?;
                let e = plan.exercises.get(ei).ok_or("Ejercicio")?.clone();
                let logged = self
                    .view
                    .sets
                    .iter()
                    .find(|s| s.exercise_id == e.exercise_id && s.index == si)
                    .cloned();
                let toggle = matches!(a, Action::ToggleSet(..));
                match (toggle, logged) {
                    (true, Some(_)) => self.forja.unlog_set(id, e.exercise_id, si).map_err(err)?,
                    (true, None) => {
                        let load = e.load.unwrap_or(Grams::ZERO);
                        self.forja
                            .log_set(id, e.exercise_id, si, e.max, load, true, false)
                            .map_err(err)?;
                        self.rest = Some((f32::from(e.rest_s), f32::from(e.rest_s)));
                        let pos = self
                            .set_positions
                            .iter()
                            .find(|p| p.0 == ei && p.1 == si)
                            .map(|p| p.2);
                        let n = rand::gen_range(8, 13);
                        self.burst(pos.unwrap_or(vec2(600.0, 300.0)), n);
                        self.flash = 0.25;
                    }
                    (false, Some(s)) => {
                        let reps = s.reps.saturating_sub(1);
                        self.forja
                            .log_set(id, e.exercise_id, si, reps, s.load, true, false)
                            .map_err(err)?
                    }
                    (false, None) => {}
                }
            }
            Action::NextSet => {
                if self.open_session_id().is_none() {
                    return Err("Primero empieza la sesión.".into());
                }
                if let Some((ei, si)) = self.next_open_set() {
                    return self.try_apply(Action::ToggleSet(ei, si));
                }
            }
            Action::ToggleWarmup(i) | Action::ToggleStretch(i) => {
                let id = self.open_session_id().ok_or("Primero empieza la sesión.")?;
                let is_warm = matches!(a, Action::ToggleWarmup(_));
                let list = if is_warm {
                    &mut self.warm_done
                } else {
                    &mut self.stretch_done
                };
                if let Some(v) = list.get_mut(i) {
                    *v = !*v;
                }
                let all = !list.is_empty() && list.iter().all(|b| *b);
                if is_warm {
                    self.forja
                        .set_routines_done(id, Some(all), None)
                        .map_err(err)?;
                } else {
                    self.forja
                        .set_routines_done(id, None, Some(all))
                        .map_err(err)?;
                }
            }
            Action::Close => {
                let id = self.open_session_id().ok_or("No hay sesión abierta.")?;
                let out = self
                    .forja
                    .close_session(id, now, self.pain, self.effort, "")
                    .map_err(err)?;
                self.last_outcome_xp = out.xp_gained;
                self.rest = None;
                self.toast(
                    format!("+{} XP · Racha {}", out.xp_gained, out.forge.current),
                    EMBER2,
                );
                if !out.counts_for_boss {
                    self.toast("Sesión muy corta o sin series: XP reducido.".into(), STEEL);
                }
                if out.level_after.level > out.level_before.level {
                    self.toast(
                        format!(
                            "¡Nivel {}! {}",
                            out.level_after.level, out.level_after.title
                        ),
                        EMBER,
                    );
                    self.burst(vec2(W - 150.0, 60.0), 40);
                    self.flash = 0.6;
                }
                if out.revived_forge {
                    self.toast("Forja reavivada: la racha sigue.".into(), DONE);
                }
                for (id, e) in out.progress {
                    let name = forja_core::catalog::exercise(&id)
                        .map(|x| x.name)
                        .unwrap_or("");
                    let msg = match e {
                        ProgressEvent::LoadUp { from, to } => format!(
                            "Sube la carga — {name}: {} → {} kg",
                            format_kg_number(from),
                            format_kg_number(to)
                        ),
                        ProgressEvent::LevelUp { .. } => format!("Subes de nivel en {name}"),
                        ProgressEvent::Technique(m) => format!("{name}: {}", m.label()),
                        ProgressEvent::Ceiling => continue,
                    };
                    self.toast(msg, DONE);
                }
            }
            Action::Pain(d) => self.pain = (i32::from(self.pain) + d).clamp(0, 10) as u8,
            Action::Effort(d) => self.effort = (i32::from(self.effort) + d).clamp(0, 10) as u8,
            Action::Expand(i) => {
                self.expanded = if self.expanded == Some(i) {
                    None
                } else {
                    Some(i)
                }
            }
            Action::OpenSkip => self.skip_text = Some(String::new()),
            Action::CancelSkip => self.skip_text = None,
            Action::ConfirmSkip => {
                let reason = self.skip_text.clone().unwrap_or_default();
                self.forja.skip_session(now, &reason).map_err(err)?;
                self.skip_text = None;
                self.toast(
                    "Registrado. Mañana: nunca dos veces seguidas.".into(),
                    STEEL,
                );
            }
            Action::Protein(g, label) => {
                let ev = self.forja.add_protein(now, g, label).map_err(err)?;
                let xp: u32 = ev.iter().map(|e| e.amount).sum();
                if xp > 0 {
                    self.toast(format!("+{xp} XP · hábito cumplido"), DONE);
                }
            }
            Action::Habit(h) => {
                let mut d: HabitDay = self.view.habits.clone();
                match h {
                    Habit::Water => d.water = !d.water,
                    Habit::Vegetables => d.vegetables = !d.vegetables,
                    Habit::NoSugarAlcohol => d.no_sugar_alcohol = !d.no_sugar_alcohol,
                    Habit::McGillBig3 => d.mcgill_big3 = !d.mcgill_big3,
                    Habit::Sleep7h => d.sleep_7h = !d.sleep_7h,
                    Habit::Steps | Habit::Protein => {}
                }
                let ev = self.forja.update_habits(now, &d).map_err(err)?;
                let xp: u32 = ev.iter().map(|e| e.amount).sum();
                if xp > 0 {
                    self.toast(format!("+{xp} XP"), DONE);
                }
            }
            Action::Steps(add) => {
                let mut d = self.view.habits.clone();
                d.steps = Some(d.steps.unwrap_or(0) + add);
                let ev = self.forja.update_habits(now, &d).map_err(err)?;
                let xp: u32 = ev.iter().map(|e| e.amount).sum();
                if xp > 0 {
                    self.toast(format!("+{xp} XP · pasos"), DONE);
                }
            }
            Action::Weight(d) => {
                self.weight_input =
                    (i64::from(self.weight_input) + i64::from(d)).clamp(30_000, 300_000) as u32
            }
            Action::LogWeight => {
                let ev = self
                    .forja
                    .log_weight(now, Grams(self.weight_input))
                    .map_err(err)?;
                self.toast(
                    format!(
                        "Peso registrado: {} kg",
                        format_kg_number(Grams(self.weight_input))
                    ),
                    TEMPER,
                );
                if let Some(e) = ev.first() {
                    self.toast(format!("+{} XP · pesaje", e.amount), DONE);
                }
            }
            Action::AddMinutes(m) => self.set_clock(now + Duration::minutes(m)),
            Action::NextMorning | Action::NextWeek => {
                let days = if matches!(a, Action::NextWeek) { 7 } else { 1 };
                let d = add_days(local_date(now), days);
                self.set_clock(resolve_local(
                    d,
                    NaiveTime::from_hms_opt(6, 45, 0).expect("time"),
                ));
                self.rest = None;
            }
            Action::ToggleDetector(ei) => {
                if self.detector.as_ref().is_some_and(|d| d.exercise == ei) {
                    self.detector = None;
                } else {
                    let plan = self.view.plan.as_ref().ok_or("Sin plan")?;
                    let e = plan.exercises.get(ei).ok_or("Ejercicio")?;
                    self.detector = Some(DetectorDemo {
                        exercise: ei,
                        id: e.exercise_id,
                        candidates: plan.exercises.iter().map(|x| x.exercise_id).collect(),
                        detector: Detector::new(30 * 20),
                        clock: 0.0,
                        since_update: 0.0,
                        result: None,
                    });
                    self.expanded = Some(ei);
                }
            }
            Action::MealPhoto => {
                if self.meal_busy {
                    return Ok(());
                }
                let Some(key) = meals::api_key() else {
                    self.key_input = Some(String::new());
                    self.photo_after_key = true;
                    return Ok(());
                };
                if let Some(path) = meals::pick_photo() {
                    self.meal_rx = Some(meals::analyze(path, key));
                    self.meal_busy = true;
                    self.toast("Analizando la foto con Claude…".into(), TEMPER);
                }
            }
            Action::EditKey => {
                self.key_input = Some(String::new());
                self.photo_after_key = false;
            }
            Action::CancelKey => {
                self.key_input = None;
                self.photo_after_key = false;
            }
            Action::SaveKey => {
                let key = self.key_input.clone().unwrap_or_default();
                if key.trim().len() < 20 {
                    return Err("Esa clave parece incompleta.".into());
                }
                meals::save_key(&key)?;
                self.key_input = None;
                self.toast(
                    "Clave guardada en el Administrador de credenciales de Windows.".into(),
                    DONE,
                );
                if std::mem::take(&mut self.photo_after_key) {
                    return self.try_apply(Action::MealPhoto);
                }
            }
        }
        Ok(())
    }

    /// Meal analysis finished on its thread → log it.
    fn poll_meal(&mut self) {
        let Some(rx) = &self.meal_rx else { return };
        let Ok(outcome) = rx.try_recv() else { return };
        self.meal_rx = None;
        self.meal_busy = false;
        match outcome {
            Ok((estimate, path)) => match self.forja.log_meal(self.now(), &estimate, Some(path)) {
                Ok((meal, xp)) => {
                    self.toast(
                        format!(
                            "{} · {} kcal · {} g proteína (confianza {})",
                            meal.description, meal.kcal, meal.protein_g, meal.confidence
                        ),
                        DONE,
                    );
                    if !estimate.note.trim().is_empty() {
                        self.toast(estimate.note.trim().to_string(), STEEL);
                    }
                    let gained: u32 = xp.iter().map(|e| e.amount).sum();
                    if gained > 0 {
                        self.toast(format!("+{gained} XP · proteína del día"), EMBER2);
                    }
                }
                Err(e) => self.toast(format!("No pude guardar la comida: {e}"), ALERT),
            },
            Err(e) => self.toast(e, ALERT),
        }
        self.refresh();
    }

    /// Feeds the exercise detector with the figure's joints (stand-in for camera keypoints).
    fn tick_detector(&mut self, dt: f32) {
        let Some(d) = self.detector.as_mut() else {
            return;
        };
        d.clock += dt;
        d.since_update += dt;
        let m = forja_core::motion::motion_for(d.id);
        d.detector.push(&forja_core::motion::sample(&m, d.clock));
        if d.since_update > 0.4 {
            d.since_update = 0.0;
            d.result = Some(d.detector.detect(&d.candidates));
        }
    }

    fn hover_popup(&mut self, f: &Fonts, t: f32) {
        let Some((id, name, at)) = self.hover_demo.take() else {
            return;
        };
        if self.skip_text.is_some() || self.key_input.is_some() {
            return;
        }
        let (w, h) = (200.0, 170.0);
        let x = at.x.min(780.0 - w);
        let y = if at.y + h > H - 50.0 {
            at.y - h - 30.0
        } else {
            at.y + 4.0
        };
        draw_rectangle(x, y, w, h, PANEL2);
        draw_rectangle_lines(x, y, w, h, 1.0, EMBER);
        fit_text(
            f,
            name,
            x + 10.0,
            y + 20.0,
            14.0,
            w - 20.0,
            TEXT,
            Face::Bold,
        );
        figure::draw(id, x + w / 2.0, y + h - 14.0, 1.0, t);
    }

    fn key_modal(&mut self, f: &Fonts) {
        let Some(text) = self.key_input.clone() else {
            return;
        };
        draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, 0.7));
        let r = Rect::new(W / 2.0 - 300.0, 220.0, 600.0, 300.0);
        draw_rectangle(r.x, r.y, r.w, r.h, PANEL);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, TEMPER);
        f.text(
            "Tu clave de Claude",
            r.x + 24.0,
            r.y + 44.0,
            30.0,
            TEXT,
            Face::Display,
        );
        let info = "Para analizar fotos de comida, Forja usa tu propia clave de la API de Claude (console.anthropic.com). \
                    Se guarda en el Administrador de credenciales de Windows. Solo se envía la foto: nada de tu perfil.";
        let mut y = r.y + 74.0;
        y += f.wrap(info, r.x + 24.0, y, r.w - 48.0, 15.0, STEEL, Face::Body);
        draw_rectangle(r.x + 24.0, y + 8.0, r.w - 48.0, 40.0, IRON);
        let n = text.chars().count();
        let shown = if n == 0 {
            "Pega aquí tu clave (Ctrl+V)".to_string()
        } else {
            let tail: String = text.chars().skip(n.saturating_sub(4)).collect();
            format!("{}{tail}", "•".repeat(n.saturating_sub(4).min(40)))
        };
        fit_text(
            f,
            &shown,
            r.x + 34.0,
            y + 34.0,
            16.0,
            r.w - 68.0,
            if n == 0 { MUTED } else { TEXT },
            Face::Mono,
        );
        let back = Rect::new(r.x + r.w - 300.0, r.y + r.h - 50.0, 130.0, 36.0);
        self.button(f, back, "Cancelar", false, true, Action::CancelKey);
        let save = Rect::new(r.x + r.w - 160.0, r.y + r.h - 50.0, 136.0, 36.0);
        self.button(f, save, "Guardar", true, n >= 20, Action::SaveKey);
        if let Some(t) = self.key_input.as_mut() {
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if ctrl && is_key_pressed(KeyCode::V) {
                if let Some(clip) = macroquad::miniquad::window::clipboard_get() {
                    t.push_str(clip.trim());
                }
            }
            while let Some(c) = get_char_pressed() {
                if !c.is_control() && !ctrl && t.chars().count() < 300 {
                    t.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                t.pop();
            }
        }
    }

    // ------------------------------------------------------------------ drawing

    #[allow(clippy::too_many_arguments)]
    fn button(
        &mut self,
        f: &Fonts,
        r: Rect,
        label: &str,
        primary: bool,
        enabled: bool,
        action: Action,
    ) {
        if draw_button(f, r, label, primary, enabled)
            && (!self.modal_open() || is_modal_action(&action))
        {
            self.pending.push(action);
        }
    }

    fn header(&mut self, f: &Fonts, t: f32) {
        draw_rectangle(0.0, 0.0, W, 104.0, IRON);
        draw_line(0.0, 104.0, W, 104.0, 1.0, LINE);
        f.text("FORJA", 24.0, 52.0, 46.0, EMBER2, Face::Display);
        let local = self.now().with_timezone(&forja_core::time::ZONE);
        let when = format!(
            "{} · {}",
            format_date_es(self.view.date),
            local.format("%H:%M")
        );
        f.text(&when, 24.0, 80.0, 18.0, STEEL, Face::Body);
        f.text(
            &self.view.phase.label(),
            24.0,
            98.0,
            15.0,
            TEMPER,
            Face::Body,
        );

        // Forge heat ember: breathes; colour and particles follow the heat.
        let heat = self.view.forge.heat;
        let h = f32::from(heat) / 100.0;
        let (cx, cy) = (520.0, 52.0);
        let breathe = if h > 0.0 {
            (t * (1.0 + 2.0 * h)).sin() * (2.0 + 4.0 * h)
        } else {
            0.0
        };
        let core = mix(STEEL, mix(EMBER, EMBER2, h), (h * 1.4).min(1.0));
        for i in 0..6 {
            let r = 18.0 + breathe + i as f32 * 5.0 * h;
            draw_circle(
                cx,
                cy,
                r,
                Color::new(core.r, core.g, core.b, 0.10 * h + 0.02),
            );
        }
        draw_circle(cx, cy, 16.0 + breathe * 0.5, core);
        draw_circle(
            cx,
            cy,
            7.0 + breathe * 0.3,
            Color::new(1.0, 0.95, 0.8, 0.5 * h + 0.1),
        );
        if heat >= 70 {
            for i in 0..5 {
                let p = (t * 0.6 + i as f32 * 0.2).fract();
                let x = cx - 12.0 + (i as f32 * 6.0) + (t * 3.0 + i as f32).sin() * 3.0;
                draw_circle(
                    x,
                    cy - 20.0 - p * 30.0,
                    2.0 * (1.0 - p),
                    Color::new(1.0, 0.7, 0.3, 1.0 - p),
                );
            }
        }
        let hs = format!("{heat}");
        f.text(&hs, cx + 40.0, cy + 4.0, 34.0, TEXT, Face::Display);
        let label = format!(
            "Calor de la forja · {} · ×{}",
            heat_label(heat),
            mult_label(heat_multiplier_pct(heat))
        );
        f.text(
            &label,
            cx + 50.0 + f.width(&hs, 34.0, Face::Display),
            cy - 2.0,
            15.0,
            STEEL,
            Face::Body,
        );
        let bar = Rect::new(cx + 40.0, cy + 14.0, 250.0, 6.0);
        draw_rectangle(bar.x, bar.y, bar.w, bar.h, PANEL2);
        draw_rectangle(bar.x, bar.y, bar.w * h, bar.h, core);
        self.hud.insert("heat");

        // Streak flame grows by tiers (1, 4, 8, 16, 24, 52).
        let tier = self.view.forge.flame_tier();
        let fx = 900.0;
        let fh = 14.0 + f32::from(tier) * 5.0;
        let flick = (t * 9.0).sin() * 1.5;
        let flame_col = if tier == 0 { STEEL } else { EMBER };
        draw_triangle(
            vec2(fx - 10.0, 70.0),
            vec2(fx + 10.0, 70.0),
            vec2(fx + flick, 70.0 - fh * 2.0),
            flame_col,
        );
        draw_triangle(
            vec2(fx - 5.0, 70.0),
            vec2(fx + 5.0, 70.0),
            vec2(fx - flick, 70.0 - fh),
            EMBER2,
        );
        f.text(
            &format!("{}", self.view.forge.current),
            fx + 18.0,
            66.0,
            34.0,
            TEXT,
            Face::Display,
        );
        f.text(
            "sesiones de racha",
            fx + 18.0,
            86.0,
            14.0,
            STEEL,
            Face::Body,
        );
        if self.view.forge.cracked {
            f.text("Arma agrietada", fx + 18.0, 100.0, 14.0, ALERT, Face::Bold);
        }
        self.hud.insert("streak");

        // Level + XP.
        let lv = self.view.level;
        let lx = 1060.0;
        f.text(
            &format!("Nivel {}", lv.level),
            lx,
            46.0,
            30.0,
            TEXT,
            Face::Display,
        );
        f.text(lv.title, lx, 66.0, 15.0, EMBER2, Face::Bold);
        draw_rectangle(lx, 76.0, 190.0, 8.0, PANEL2);
        draw_rectangle(
            lx,
            76.0,
            190.0 * lv.into_level as f32 / lv.needed.max(1) as f32,
            8.0,
            EMBER,
        );
        f.text(
            &format!("{} / {} XP", lv.into_level, lv.needed),
            lx,
            100.0,
            14.0,
            STEEL,
            Face::Mono,
        );
        self.hud.insert("level");
    }

    fn left(&mut self, f: &Fonts, t: f32) {
        let x0 = 20.0;
        let w = 760.0;
        let mut y = 124.0 - self.scroll;
        let view_day = self.view.day;
        let session = self.view.session.clone();
        let open = session.as_ref().is_some_and(|s| !s.is_closed());
        let done = session.as_ref().is_some_and(|s| s.is_done());
        let late = self
            .view
            .alerts
            .iter()
            .any(|a| matches!(a, Alert::LateForTraining { .. }));

        // Title card (pulses red when the training time has passed).
        let card_h = 92.0;
        draw_rectangle(x0, y, w, card_h, PANEL);
        if late {
            let pulse = 0.5 + 0.5 * (t * 4.0).sin();
            draw_rectangle_lines(
                x0,
                y,
                w,
                card_h,
                3.0,
                Color::new(ALERT.r, ALERT.g, ALERT.b, 0.4 + 0.6 * pulse),
            );
        }
        let (title, focus) = match &session {
            Some(s) => (s.kind.name().to_string(), s.kind.focus().to_string()),
            None => (view_day.title().to_string(), view_day.focus().to_string()),
        };
        f.text("HOY", x0 + 16.0, y + 26.0, 14.0, MUTED, Face::Bold);
        fit_text(
            f,
            &title,
            x0 + 16.0,
            y + 58.0,
            34.0,
            420.0,
            TEXT,
            Face::Display,
        );
        f.text(&focus, x0 + 16.0, y + 80.0, 16.0, STEEL, Face::Body);
        self.hud.insert("session");
        let bx = x0 + w - 16.0;
        if done {
            draw_check(vec2(bx - 132.0, y + 46.0), 9.0, DONE);
            f.text("Hecho", bx - 110.0, y + 56.0, 26.0, DONE, Face::Display);
        } else if open {
            if let Some((left, total)) = self.rest {
                draw_rest_ring(f, vec2(bx - 40.0, y + 46.0), left, total);
            } else {
                f.text(
                    "Sesión en curso",
                    bx - 150.0,
                    y + 56.0,
                    18.0,
                    EMBER2,
                    Face::Bold,
                );
            }
        } else if view_day.is_training() {
            let full = Action::Start(SessionVariant::Full);
            self.button(
                f,
                Rect::new(bx - 150.0, y + 14.0, 150.0, 34.0),
                "Empezar",
                true,
                true,
                full,
            );
            let min = Action::Start(SessionVariant::Minimum);
            self.button(
                f,
                Rect::new(bx - 150.0, y + 52.0, 150.0, 30.0),
                "Versión mínima",
                false,
                true,
                min,
            );
            self.button(
                f,
                Rect::new(bx - 250.0, y + 52.0, 92.0, 30.0),
                "Saltar…",
                false,
                true,
                Action::OpenSkip,
            );
        } else if let Some(m) = self.view.recoverable {
            let r = Rect::new(bx - 240.0, y + 26.0, 240.0, 40.0);
            self.button(
                f,
                r,
                "Recuperar sesión fallada",
                true,
                true,
                Action::Recover(m),
            );
        }
        y += card_h + 12.0;

        let Some(plan) = self.view.plan.clone() else {
            // Rest day: "Espalda feliz".
            let routine = self.view.rest_routine.clone();
            if !routine.is_empty() {
                f.text(
                    "Movilidad «Espalda feliz» · 10 min",
                    x0,
                    y + 18.0,
                    20.0,
                    TEXT,
                    Face::Bold,
                );
                y += 34.0;
                for it in routine {
                    draw_rectangle(x0, y, w, 34.0, PANEL);
                    f.text(it.name, x0 + 14.0, y + 23.0, 17.0, TEXT, Face::Body);
                    let l = it.dose.label();
                    f.text(
                        &l,
                        x0 + w - 14.0 - f.width(&l, 16.0, Face::Mono),
                        y + 23.0,
                        16.0,
                        EMBER2,
                        Face::Mono,
                    );
                    y += 38.0;
                }
                self.hud.insert("exercises");
                self.hud.insert("warmup");
            }
            return;
        };

        for m in &plan.safety_messages {
            draw_rectangle(x0, y, w, 28.0, Color::new(0.49, 0.61, 1.0, 0.12));
            f.text(m, x0 + 12.0, y + 19.0, 15.0, TEMPER, Face::Body);
            y += 32.0;
        }

        y = self.chips(f, "Calentamiento", &plan.warmup, true, x0, y, w);
        self.hud.insert("warmup");

        self.set_positions.clear();
        let mouse: Vec2 = mouse_position().into();
        for (ei, e) in plan.exercises.iter().enumerate() {
            let expanded = self.expanded == Some(ei);
            let row_h = if expanded { 236.0 } else { 52.0 };
            draw_rectangle(x0, y, w, row_h, PANEL);
            let head = Rect::new(x0, y, 380.0, 52.0);
            if head.contains(mouse)
                && is_mouse_button_released(MouseButton::Left)
                && !self.modal_open()
            {
                self.pending.push(Action::Expand(ei));
            }
            draw_chevron(vec2(x0 + 15.0, y + 17.0), expanded, MUTED);
            let name = match e.level_name {
                Some(l) => format!("{} · {l}", e.name),
                None => e.name.clone(),
            };
            fit_text(f, &name, x0 + 28.0, y + 22.0, 17.0, 370.0, TEXT, Face::Bold);
            let load = match e.load {
                Some(g) if e.load_mode == forja_core::profile::LoadMode::Pair => {
                    format!("{} kg c/u", format_kg_number(g))
                }
                Some(g) => format!("{} kg", format_kg_number(g)),
                None => "peso corporal".into(),
            };
            let mut sub = format!("{} · {}", e.dose_label(), load);
            if let Some(m) = e.modifier {
                sub = format!("{sub} · {}", m.label());
            }
            fit_text(f, &sub, x0 + 28.0, y + 42.0, 15.0, 370.0, STEEL, Face::Mono);

            for si in 0..e.sets {
                let c = vec2(x0 + 430.0 + f32::from(si) * 50.0, y + 26.0);
                self.set_positions.push((ei, si, c));
                let logged = self
                    .view
                    .sets
                    .iter()
                    .find(|s| s.exercise_id == e.exercise_id && s.index == si);
                let hover = c.distance(mouse) < 18.0;
                match logged {
                    Some(s) => {
                        draw_circle(c.x, c.y, 18.0, DONE);
                        let r = format!("{}", s.reps);
                        f.text(
                            &r,
                            c.x - f.width(&r, 16.0, Face::Mono) / 2.0,
                            c.y + 6.0,
                            16.0,
                            IRON,
                            Face::Mono,
                        );
                    }
                    None => {
                        draw_circle_lines(
                            c.x,
                            c.y,
                            18.0,
                            2.0,
                            if hover && open { EMBER2 } else { STEEL },
                        );
                        let r = format!("{}", si + 1);
                        f.text(
                            &r,
                            c.x - f.width(&r, 14.0, Face::Mono) / 2.0,
                            c.y + 5.0,
                            14.0,
                            MUTED,
                            Face::Mono,
                        );
                    }
                }
                if hover && !self.modal_open() {
                    if is_mouse_button_released(MouseButton::Left) {
                        self.pending.push(Action::ToggleSet(ei, si));
                    } else if is_mouse_button_released(MouseButton::Right) {
                        self.pending.push(Action::LessReps(ei, si));
                    }
                }
            }
            if expanded {
                figure::draw(e.exercise_id, x0 + 95.0, y + 190.0, 1.05, t);
                // Exercise detector, fed by the animated figure (the real camera arrives with the iPhone app).
                let det_on = self.detector.as_ref().is_some_and(|d| d.exercise == ei);
                let label = if det_on {
                    "Detener detector"
                } else {
                    "Detector de ejercicio"
                };
                let r = Rect::new(x0 + w - 214.0, y + row_h - 40.0, 200.0, 30.0);
                self.button(f, r, label, false, true, Action::ToggleDetector(ei));
                let line = match self.detector.as_ref().filter(|d| d.exercise == ei) {
                    Some(d) => d.summary(),
                    None => {
                        "Reconoce el ejercicio y cuenta reps con la cámara (aquí, con la figura)."
                            .into()
                    }
                };
                fit_text(
                    f,
                    &line,
                    x0 + 190.0,
                    y + row_h - 20.0,
                    14.0,
                    w - 420.0,
                    EMBER2,
                    Face::Body,
                );
                let tx = x0 + 190.0;
                let mut ty = y + 72.0;
                f.text("Técnica", tx, ty, 14.0, MUTED, Face::Bold);
                ty += 18.0;
                ty += f.wrap(e.technique, tx, ty, w - 210.0, 16.0, TEXT, Face::Body);
                if let Some(tempo) = e.tempo {
                    f.text(
                        &format!("Tempo {tempo} (bajar-pausa-subir)"),
                        tx,
                        ty,
                        15.0,
                        TEMPER,
                        Face::Mono,
                    );
                    ty += 20.0;
                }
                draw_rectangle(tx, ty - 12.0, 10.0, 10.0, DONE);
                f.text("Espalda", tx + 16.0, ty - 2.0, 14.0, DONE, Face::Bold);
                ty += 16.0;
                ty += f.wrap(e.back_note, tx, ty, w - 210.0, 15.0, STEEL, Face::Body);
                if let Some(alt) = e.alternative.and_then(forja_core::catalog::exercise) {
                    f.text(
                        &format!("Si molesta: {}", alt.name),
                        tx,
                        ty + 2.0,
                        15.0,
                        TEMPER,
                        Face::Body,
                    );
                }
                if let Some(from) = e.substituted_from.and_then(forja_core::catalog::exercise) {
                    let s = format!("Reemplaza a: {} (dolor lumbar)", from.name);
                    f.text(&s, tx + 330.0, y + 72.0, 14.0, TEMPER, Face::Body);
                }
            }
            y += row_h + 6.0;
        }
        self.hud.insert("exercises");

        y = self.chips(f, "Estiramientos", &plan.stretches, false, x0, y + 4.0, w);

        if open {
            draw_rectangle(x0, y, w, 64.0, PANEL);
            f.text("Dolor lumbar", x0 + 14.0, y + 26.0, 15.0, STEEL, Face::Body);
            let pc = if self.pain > 3 { ALERT } else { DONE };
            f.text(
                &format!("{}/10", self.pain),
                x0 + 14.0,
                y + 52.0,
                22.0,
                pc,
                Face::Mono,
            );
            self.button(
                f,
                Rect::new(x0 + 90.0, y + 30.0, 30.0, 28.0),
                "−",
                false,
                true,
                Action::Pain(-1),
            );
            self.button(
                f,
                Rect::new(x0 + 124.0, y + 30.0, 30.0, 28.0),
                "+",
                false,
                true,
                Action::Pain(1),
            );
            f.text("Esfuerzo", x0 + 190.0, y + 26.0, 15.0, STEEL, Face::Body);
            f.text(
                &format!("{}/10", self.effort),
                x0 + 190.0,
                y + 52.0,
                22.0,
                TEXT,
                Face::Mono,
            );
            self.button(
                f,
                Rect::new(x0 + 266.0, y + 30.0, 30.0, 28.0),
                "−",
                false,
                true,
                Action::Effort(-1),
            );
            self.button(
                f,
                Rect::new(x0 + 300.0, y + 30.0, 30.0, 28.0),
                "+",
                false,
                true,
                Action::Effort(1),
            );
            let r = Rect::new(x0 + w - 200.0, y + 14.0, 186.0, 38.0);
            self.button(f, r, "Cerrar sesión", true, true, Action::Close);
            y += 72.0;
        }
        let content_bottom = y + self.scroll;
        let max_scroll = (content_bottom - (H - 60.0)).max(0.0);
        let (_, wheel) = mouse_wheel();
        if mouse.x < 790.0 && wheel != 0.0 {
            self.scroll = (self.scroll - wheel.signum() * 40.0).clamp(0.0, max_scroll);
        }
        self.scroll = self.scroll.min(max_scroll);
    }

    #[allow(clippy::too_many_arguments)]
    fn chips(
        &mut self,
        f: &Fonts,
        title: &str,
        items: &[RoutineItem],
        warm: bool,
        x0: f32,
        y0: f32,
        w: f32,
    ) -> f32 {
        let open = self.open_session_id().is_some();
        let done_list = if warm {
            &self.warm_done
        } else {
            &self.stretch_done
        };
        let n_done = done_list.iter().filter(|b| **b).count();
        f.text(
            &format!("{title} · {n_done}/{}", items.len()),
            x0,
            y0 + 16.0,
            16.0,
            TEXT,
            Face::Bold,
        );
        let mut x = x0;
        let mut y = y0 + 24.0;
        let mut clicks = Vec::new();
        let mouse: Vec2 = mouse_position().into();
        for (i, it) in items.iter().enumerate() {
            let label = format!("{} {}", it.name, it.dose.label());
            let cw = f.width(&label, 14.0, Face::Body) + 24.0;
            if x + cw > x0 + w {
                x = x0;
                y += 30.0;
            }
            let r = Rect::new(x, y, cw, 26.0);
            let is_done = done_list.get(i).copied().unwrap_or(false);
            let bg = if is_done {
                Color::new(0.37, 0.82, 0.56, 0.18)
            } else {
                PANEL
            };
            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, if is_done { DONE } else { LINE });
            f.text(
                &label,
                r.x + 12.0,
                r.y + 18.0,
                14.0,
                if is_done { DONE } else { STEEL },
                Face::Body,
            );
            if r.contains(mouse) {
                // Hover shows the animated demo of the warm-up / stretch.
                self.hover_demo = Some((it.id, it.name, vec2(r.x, r.y + r.h)));
            }
            if open && r.contains(mouse) && is_mouse_button_released(MouseButton::Left) {
                clicks.push(i);
            }
            x += cw + 6.0;
        }
        if !self.modal_open() {
            for i in clicks {
                self.pending.push(if warm {
                    Action::ToggleWarmup(i)
                } else {
                    Action::ToggleStretch(i)
                });
            }
        }
        y + 36.0
    }

    fn alert_text(a: &Alert) -> (String, Color) {
        match a {
            Alert::LateForTraining { since } => (
                format!(
                    "Ya pasaron las {}: tu sesión te espera.",
                    since.format("%H:%M")
                ),
                ALERT,
            ),
            Alert::MissedRecently {
                can_recover_today: true,
                ..
            } => (
                "Fallaste: nunca dos veces seguidas. Recupérala hoy.".into(),
                EMBER,
            ),
            Alert::MissedRecently { .. } => ("Fallaste: nunca dos veces seguidas.".into(), EMBER),
            Alert::ReviveForge(r) => (
                format!(
                    "Misión: Reavivar la forja (hasta el {})",
                    format_date_es(r.deadline)
                ),
                EMBER2,
            ),
            Alert::WeaponCracked => (
                "Arma agrietada: dos fallos seguidos. Lee tu contrato.".into(),
                ALERT,
            ),
            Alert::RedFlags(_) => (
                "Señales de alerta: consulta a un médico. Carga axial en pausa.".into(),
                ALERT,
            ),
            Alert::Pain(PainLight::Red) => (
                "Dolor sobre 3: hoy usamos alternativas seguras.".into(),
                TEMPER,
            ),
            Alert::Pain(_) => ("El dolor subió: tienes alternativas a mano.".into(), TEMPER),
            Alert::Deload => ("Semana de descarga: mitad de series.".into(), TEMPER),
            Alert::StrengthTestWeek => ("Esta semana toca prueba de fuerza.".into(), TEMPER),
            Alert::ContractMissing => (
                "Aún no firmas tu Contrato (llega con el onboarding).".into(),
                MUTED,
            ),
        }
    }

    fn right(&mut self, f: &Fonts, t: f32) {
        let x0 = 800.0;
        let w = 460.0;
        let mut y = 124.0;
        let mouse: Vec2 = mouse_position().into();
        let click = is_mouse_button_released(MouseButton::Left) && !self.modal_open();

        for a in self.view.alerts.clone().iter().take(4) {
            let (txt, col) = Self::alert_text(a);
            let late = matches!(a, Alert::LateForTraining { .. });
            let pulse = if late {
                0.5 + 0.5 * (t * 4.0).sin()
            } else {
                1.0
            };
            draw_rectangle(
                x0,
                y,
                6.0,
                26.0,
                Color::new(col.r, col.g, col.b, 0.4 + 0.6 * pulse),
            );
            draw_rectangle(x0 + 6.0, y, w - 6.0, 26.0, PANEL);
            fit_text(
                f,
                &txt,
                x0 + 16.0,
                y + 18.0,
                14.0,
                w - 26.0,
                col,
                Face::Body,
            );
            y += 30.0;
        }
        y += 6.0;

        // Habits.
        draw_rectangle(x0, y, w, 176.0, PANEL);
        let met = self.view.habits_met.clone();
        f.text(
            &format!("Hábitos de hoy · {}/7", met.len()),
            x0 + 14.0,
            y + 24.0,
            17.0,
            TEXT,
            Face::Bold,
        );
        self.hud.insert("habits");
        let hy = y + 36.0;
        let steps = self.view.habits.steps.unwrap_or(0);
        for (i, h) in Habit::ALL.iter().enumerate() {
            let r = Rect::new(
                x0 + 14.0 + (i % 2) as f32 * 222.0,
                hy + (i / 2) as f32 * 32.0,
                214.0,
                28.0,
            );
            let ok = met.contains(h);
            draw_rectangle(
                r.x,
                r.y,
                r.w,
                r.h,
                if ok {
                    Color::new(0.37, 0.82, 0.56, 0.16)
                } else {
                    PANEL2
                },
            );
            if ok {
                draw_check(vec2(r.x + 14.0, r.y + 14.0), 6.0, DONE);
            } else {
                draw_circle_lines(r.x + 14.0, r.y + 14.0, 6.0, 1.5, MUTED);
            }
            let label = match h {
                Habit::Steps => format!(
                    "Pasos {}/{}",
                    group_thousands(u64::from(steps)),
                    group_thousands(u64::from(self.view.steps_goal))
                ),
                Habit::Protein => format!("Proteína {} g", self.view.protein_today),
                other => other.label().to_string(),
            };
            fit_text(
                f,
                &label,
                r.x + 28.0,
                r.y + 19.0,
                14.0,
                r.w - 34.0,
                if ok { DONE } else { TEXT },
                Face::Body,
            );
            if !matches!(h, Habit::Steps | Habit::Protein) && r.contains(mouse) && click {
                self.pending.push(Action::Habit(*h));
            }
        }
        let by = hy + 3.0 * 32.0;
        f.text("Pasos", x0 + 244.0, by + 19.0, 14.0, STEEL, Face::Body);
        self.button(
            f,
            Rect::new(x0 + 292.0, by, 74.0, 28.0),
            "+1.000",
            false,
            true,
            Action::Steps(1000),
        );
        self.button(
            f,
            Rect::new(x0 + 372.0, by, 74.0, 28.0),
            "+5.000",
            false,
            true,
            Action::Steps(5000),
        );
        y += 184.0;

        // Protein.
        let n = self.view.nutrition;
        draw_rectangle(x0, y, w, 150.0, PANEL);
        f.text("Proteína", x0 + 14.0, y + 24.0, 17.0, TEXT, Face::Bold);
        let pt = format!("{} / {} g", self.view.protein_today, n.protein_g);
        f.text(
            &pt,
            x0 + w - 14.0 - f.width(&pt, 18.0, Face::Mono),
            y + 24.0,
            18.0,
            EMBER2,
            Face::Mono,
        );
        let frac = (self.view.protein_today as f32 / n.protein_g.max(1) as f32).min(1.0);
        draw_rectangle(x0 + 14.0, y + 34.0, w - 28.0, 6.0, PANEL2);
        draw_rectangle(
            x0 + 14.0,
            y + 34.0,
            (w - 28.0) * frac,
            6.0,
            if frac >= 1.0 { DONE } else { EMBER },
        );
        self.hud.insert("protein");
        for (i, (_, label, g)) in PROTEIN_QUICK.iter().enumerate() {
            let r = Rect::new(
                x0 + 14.0 + (i % 3) as f32 * 146.0,
                y + 48.0 + (i / 3) as f32 * 25.0,
                140.0,
                22.0,
            );
            let hover = r.contains(mouse);
            draw_rectangle(r.x, r.y, r.w, r.h, if hover { HOVER } else { PANEL2 });
            fit_text(
                f,
                &format!("{label} +{g}"),
                r.x + 6.0,
                r.y + 16.0,
                12.5,
                r.w - 10.0,
                STEEL,
                Face::Body,
            );
            if hover && click {
                self.pending.push(Action::Protein(*g, label));
            }
        }
        y += 158.0;

        // Nutrition + weight.
        draw_rectangle(x0, y, w, 120.0, PANEL);
        f.text("Meta diaria", x0 + 14.0, y + 24.0, 17.0, TEXT, Face::Bold);
        let kcal = format!("{} kcal", group_thousands(u64::from(n.kcal)));
        f.text(
            &kcal,
            x0 + w - 14.0 - f.width(&kcal, 22.0, Face::Display),
            y + 26.0,
            22.0,
            EMBER2,
            Face::Display,
        );
        let line = format!(
            "Proteína {} g · Grasas {} g · Carbos {} g · Agua {} L · Fibra {}+ g",
            n.protein_g,
            n.fat_g,
            n.carbs_g,
            format_one_decimal(f64::from(n.water_ml) / 1000.0),
            n.fiber_g
        );
        fit_text(
            f,
            &line,
            x0 + 14.0,
            y + 48.0,
            14.0,
            w - 28.0,
            STEEL,
            Face::Body,
        );
        let basis = format!(
            "Mifflin-St Jeor con {} kg · ×1,45 · −600 kcal",
            format_kg_number(n.weight)
        );
        f.text(&basis, x0 + 14.0, y + 68.0, 13.0, MUTED, Face::Body);
        self.hud.insert("nutrition");
        f.text("Peso", x0 + 14.0, y + 102.0, 15.0, STEEL, Face::Body);
        let wtxt = format!("{} kg", format_kg_number(Grams(self.weight_input)));
        f.text(&wtxt, x0 + 60.0, y + 103.0, 20.0, TEXT, Face::Mono);
        self.button(
            f,
            Rect::new(x0 + 170.0, y + 82.0, 34.0, 28.0),
            "−",
            false,
            true,
            Action::Weight(-100),
        );
        self.button(
            f,
            Rect::new(x0 + 208.0, y + 82.0, 34.0, 28.0),
            "+",
            false,
            true,
            Action::Weight(100),
        );
        let r = Rect::new(x0 + 260.0, y + 82.0, 186.0, 28.0);
        self.button(f, r, "Registrar pesaje", false, true, Action::LogWeight);
        y += 128.0;

        // Meals: one photo → Claude estimates → logged.
        draw_rectangle(x0, y, w, 98.0, PANEL);
        f.text(
            "Comidas de hoy",
            x0 + 14.0,
            y + 24.0,
            17.0,
            TEXT,
            Face::Bold,
        );
        let eaten = format!(
            "{} / {} kcal",
            group_thousands(u64::from(self.view.kcal_today)),
            group_thousands(u64::from(n.kcal))
        );
        f.text(
            &eaten,
            x0 + w - 14.0 - f.width(&eaten, 17.0, Face::Mono),
            y + 24.0,
            17.0,
            EMBER2,
            Face::Mono,
        );
        let frac = (self.view.kcal_today as f32 / n.kcal.max(1) as f32).min(1.2);
        draw_rectangle(x0 + 14.0, y + 34.0, w - 28.0, 6.0, PANEL2);
        let bar_col = if frac > 1.0 { ALERT } else { DONE };
        draw_rectangle(
            x0 + 14.0,
            y + 34.0,
            (w - 28.0) * frac.min(1.0),
            6.0,
            bar_col,
        );
        let last = match (self.meal_busy, self.view.meals.last()) {
            (true, _) => "Analizando la foto con Claude…".to_string(),
            (false, Some(m)) => format!(
                "{} · {} kcal · {} g proteína",
                m.description, m.kcal, m.protein_g
            ),
            (false, None) => {
                "Saca una foto a tu plato: Claude calcula y queda registrado.".to_string()
            }
        };
        fit_text(
            f,
            &last,
            x0 + 14.0,
            y + 60.0,
            14.0,
            w - 28.0,
            STEEL,
            Face::Body,
        );
        let r = Rect::new(x0 + 14.0, y + 66.0, 200.0, 26.0);
        self.button(
            f,
            r,
            "Foto de comida",
            true,
            !self.meal_busy,
            Action::MealPhoto,
        );
        let r = Rect::new(x0 + 222.0, y + 66.0, 150.0, 26.0);
        self.button(
            f,
            r,
            "Clave de Claude",
            false,
            !self.meal_busy,
            Action::EditKey,
        );
        self.hud.insert("meals");
        y += 106.0;

        // Weekly review (rules-based coach), this week so far.
        let space = H - 56.0 - y;
        if space > 60.0 {
            draw_rectangle(x0, y, w, space, PANEL);
            f.text(
                "Revisión de la semana",
                x0 + 14.0,
                y + 24.0,
                17.0,
                TEXT,
                Face::Bold,
            );
            let mut ly = y + 46.0;
            for line in self.review.clone() {
                if ly + 18.0 > y + space {
                    break;
                }
                ly += f.wrap(&line, x0 + 14.0, ly, w - 28.0, 13.5, STEEL, Face::Body) + 2.0;
            }
            self.hud.insert("review");
        }
    }

    fn bottom(&mut self, f: &Fonts) {
        let y = H - 48.0;
        draw_rectangle(0.0, y, W, 48.0, IRON);
        draw_line(0.0, y, W, y, 1.0, LINE);
        f.text(
            "Simulador de tiempo (modo desarrollador)",
            20.0,
            y + 30.0,
            15.0,
            MUTED,
            Face::Bold,
        );
        self.button(
            f,
            Rect::new(330.0, y + 9.0, 90.0, 30.0),
            "+30 min",
            false,
            true,
            Action::AddMinutes(30),
        );
        self.button(
            f,
            Rect::new(426.0, y + 9.0, 150.0, 30.0),
            "Mañana 06:45",
            false,
            true,
            Action::NextMorning,
        );
        self.button(
            f,
            Rect::new(582.0, y + 9.0, 110.0, 30.0),
            "+1 semana",
            false,
            true,
            Action::NextWeek,
        );
        let note = if self.simulated {
            "Tiempo simulado · demo en memoria: nada se guarda"
        } else {
            "Demo del hito 1 · tu perfil, registros vacíos, nada se guarda"
        };
        f.text(
            note,
            W - 20.0 - f.width(note, 14.0, Face::Body),
            y + 30.0,
            14.0,
            STEEL,
            Face::Body,
        );
    }

    fn skip_modal(&mut self, f: &Fonts) {
        let Some(text) = self.skip_text.clone() else {
            return;
        };
        draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, 0.7));
        let r = Rect::new(W / 2.0 - 300.0, 180.0, 600.0, 390.0);
        draw_rectangle(r.x, r.y, r.w, r.h, PANEL);
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, EMBER);
        f.text(
            "Antes de fallar",
            r.x + 24.0,
            r.y + 44.0,
            30.0,
            TEXT,
            Face::Display,
        );
        let why = self
            .forja
            .store()
            .load_contract()
            .ok()
            .flatten()
            .filter(|c| c.is_signed())
            .map(|c| format!("Tu porqué: «{}»", c.why))
            .unwrap_or_else(|| "Tu porqué aparecerá aquí cuando firmes el Contrato.".into());
        let mut y = r.y + 74.0;
        y += f.wrap(&why, r.x + 24.0, y, r.w - 48.0, 17.0, EMBER2, Face::Body);
        let offer = "¿Y si haces la versión mínima? 15 minutos, cuenta para la racha.";
        y += f.wrap(
            offer,
            r.x + 24.0,
            y + 6.0,
            r.w - 48.0,
            16.0,
            STEEL,
            Face::Body,
        );
        let min = Action::Start(SessionVariant::Minimum);
        self.button(
            f,
            Rect::new(r.x + 24.0, y + 14.0, 250.0, 36.0),
            "Hacer versión mínima",
            true,
            true,
            min,
        );
        y += 76.0;
        let n = text.trim().chars().count();
        f.text(
            &format!("Motivo ({n}/25 caracteres mínimo)"),
            r.x + 24.0,
            y,
            15.0,
            STEEL,
            Face::Body,
        );
        draw_rectangle(r.x + 24.0, y + 10.0, r.w - 48.0, 60.0, IRON);
        let caret = if (get_time() * 2.0) as i64 % 2 == 0 {
            "|"
        } else {
            ""
        };
        f.wrap(
            &format!("{text}{caret}"),
            r.x + 34.0,
            y + 34.0,
            r.w - 68.0,
            16.0,
            TEXT,
            Face::Body,
        );
        let back = Rect::new(r.x + r.w - 300.0, r.y + r.h - 50.0, 130.0, 36.0);
        self.button(f, back, "Volver", false, true, Action::CancelSkip);
        let confirm = Rect::new(r.x + r.w - 160.0, r.y + r.h - 50.0, 136.0, 36.0);
        self.button(
            f,
            confirm,
            "Confirmar falla",
            false,
            n >= 25,
            Action::ConfirmSkip,
        );
        if let Some(t) = self.skip_text.as_mut() {
            while let Some(c) = get_char_pressed() {
                if !c.is_control() && t.chars().count() < 200 {
                    t.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                t.pop();
            }
        }
    }

    fn effects(&mut self, f: &Fonts, dt: f32) {
        for s in &mut self.sparks {
            s.vel.y += 600.0 * dt;
            s.pos += s.vel * dt;
            s.life -= dt;
        }
        self.sparks.retain(|s| s.life > 0.0);
        for s in &self.sparks {
            let a = (s.life / 0.9).clamp(0.0, 1.0);
            let c = mix(EMBER, EMBER2, a);
            let tail = s.pos - s.vel * 0.02;
            draw_line(
                s.pos.x,
                s.pos.y,
                tail.x,
                tail.y,
                2.0,
                Color::new(c.r, c.g, c.b, a),
            );
        }
        if self.flash > 0.0 {
            draw_rectangle(
                0.0,
                0.0,
                W,
                H,
                Color::new(1.0, 0.55, 0.15, self.flash * 0.15),
            );
            self.flash -= dt;
        }
        let mut ty = H - 70.0;
        for (msg, life, col) in self.toasts.iter_mut().rev() {
            *life -= dt;
            let a = life.clamp(0.0, 1.0);
            let wdt = f.width(msg, 16.0, Face::Bold) + 28.0;
            let x = W / 2.0 - wdt / 2.0;
            draw_rectangle(
                x,
                ty - 22.0,
                wdt,
                32.0,
                Color::new(PANEL.r, PANEL.g, PANEL.b, 0.95 * a),
            );
            draw_rectangle(x, ty - 22.0, 4.0, 32.0, Color::new(col.r, col.g, col.b, a));
            f.text(
                msg,
                x + 16.0,
                ty,
                16.0,
                Color::new(TEXT.r, TEXT.g, TEXT.b, a),
                Face::Bold,
            );
            ty -= 38.0;
        }
        self.toasts.retain(|t| t.1 > 0.0);
    }

    fn modal_open(&self) -> bool {
        self.skip_text.is_some() || self.key_input.is_some()
    }

    fn frame(&mut self, f: &Fonts, dt: f32, t: f32) {
        self.poll_meal();
        self.tick_detector(dt);
        if let Some((left, total)) = self.rest {
            let l = left - dt;
            if l <= 0.0 {
                self.rest = None;
                self.toast("Descanso listo: siguiente serie.".into(), TEMPER);
            } else {
                self.rest = Some((l, total));
            }
        }
        // Follow the real clock: the late alert at 07:00 and the day change appear on their own.
        if !self.frozen && (t as i64) != ((t - dt) as i64) {
            self.refresh();
        }
        clear_background(IRON);
        self.left(f, t);
        self.right(f, t);
        self.header(f, t);
        self.bottom(f);
        self.effects(f, dt);
        self.hover_popup(f, t);
        self.skip_modal(f);
        self.key_modal(f);

        if !self.modal_open() {
            if is_key_pressed(KeyCode::Space) {
                self.pending.push(Action::NextSet);
            }
            if is_key_pressed(KeyCode::N) {
                self.pending.push(Action::NextMorning);
            }
        } else if is_key_pressed(KeyCode::Escape) {
            self.pending.push(Action::CancelSkip);
        }
        self.run_pending();
    }
}

fn is_modal_action(a: &Action) -> bool {
    matches!(
        a,
        Action::CancelSkip
            | Action::ConfirmSkip
            | Action::Start(SessionVariant::Minimum)
            | Action::SaveKey
            | Action::CancelKey
    )
}

fn mult_label(pct: u32) -> String {
    let (whole, frac) = (pct / 100, pct % 100);
    match frac {
        0 => format!("{whole}"),
        f if f % 10 == 0 => format!("{whole},{}", f / 10),
        f => format!("{whole},{f:02}"),
    }
}

// ---------------------------------------------------------------- run modes

#[cfg(windows)]
mod winhelp {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        FindWindowW, GetForegroundWindow, SetForegroundWindow, SetWindowPos, HWND_BOTTOM,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    };

    static PREVIOUS: AtomicIsize = AtomicIsize::new(0);

    pub fn remember_foreground() {
        // SAFETY: plain Win32 call without arguments.
        let h = unsafe { GetForegroundWindow() };
        PREVIOUS.store(h as isize, Ordering::SeqCst);
    }

    /// Push our window behind the others without activating it, and hand focus back.
    pub fn send_to_back(title: &str) {
        let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the calls; handles are checked.
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), wide.as_ptr());
            if !hwnd.is_null() {
                SetWindowPos(
                    hwnd,
                    HWND_BOTTOM,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
            let prev = PREVIOUS.load(Ordering::SeqCst) as HWND;
            if !prev.is_null() && prev != hwnd {
                SetForegroundWindow(prev);
            }
        }
    }
}

fn draw_gallery(f: &Fonts, t: f32) {
    use forja_core::catalog::{
        EXERCISES, HAPPY_BACK, STRETCH_LOWER, STRETCH_UPPER, WARMUP_BASE, WARMUP_LOWER,
        WARMUP_UPPER,
    };
    clear_background(IRON);
    let mut items: Vec<(&str, &str)> = EXERCISES.iter().map(|e| (e.id, e.name)).collect();
    for list in [
        WARMUP_BASE,
        WARMUP_UPPER,
        WARMUP_LOWER,
        STRETCH_UPPER,
        STRETCH_LOWER,
    ] {
        items.extend(list.iter().map(|i| (i.id, i.name)));
    }
    let _ = HAPPY_BACK;
    let cols = 10;
    let (cw, ch) = (W / cols as f32, H / 6.0);
    for (n, (id, name)) in items.iter().enumerate() {
        let (cx, cy) = ((n % cols) as f32 * cw, (n / cols) as f32 * ch);
        draw_rectangle_lines(cx, cy, cw, ch, 1.0, LINE);
        fit_text(
            f,
            name,
            cx + 4.0,
            cy + 13.0,
            11.0,
            cw - 8.0,
            STEEL,
            Face::Body,
        );
        // Freeze each figure at the end pose (phase 1) to check the hardest keyframe.
        figure::draw(
            id,
            cx + cw / 2.0,
            cy + ch - 10.0,
            0.8,
            t * 0.0 + 0.5 * forja_core::motion::motion_for(id).period,
        );
    }
}

enum Mode {
    Normal,
    Screenshot(String),
    SelfTest(String),
    /// Every exercise / warm-up / stretch animation in a grid, saved as PNG (visual check).
    Gallery(String),
}

fn parse_mode() -> Mode {
    let args: Vec<String> = std::env::args().collect();
    for i in 1..args.len() {
        match args[i].as_str() {
            "--screenshot" if i + 1 < args.len() => return Mode::Screenshot(args[i + 1].clone()),
            "--self-test" if i + 1 < args.len() => return Mode::SelfTest(args[i + 1].clone()),
            "--gallery" if i + 1 < args.len() => return Mode::Gallery(args[i + 1].clone()),
            _ => {}
        }
    }
    Mode::Normal
}

fn save_screen(path: &str) {
    // get_screen_data already returns rows top-down here (checked on Windows/OpenGL).
    let mut out = get_screen_data();
    for px in out.bytes.chunks_mut(4) {
        px[3] = 255;
    }
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    out.export_png(path);
}

struct Step {
    step: &'static str,
    passed: bool,
    detail: String,
}

fn write_results(path: &str, steps: &[Step]) {
    let list: Vec<_> = steps
        .iter()
        .map(|s| serde_json::json!({"step": s.step, "passed": s.passed, "detail": s.detail}))
        .collect();
    let v = serde_json::json!({ "version": 1, "steps": list });
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, serde_json::to_string_pretty(&v).unwrap_or_default());
}

/// Scripted play-through for `--self-test`, one call per frame. Returns true when finished.
struct SelfTest {
    path: String,
    steps: Vec<Step>,
    start_date: NaiveDate,
    sets_marked: u32,
    sparks_seen: usize,
}

impl SelfTest {
    fn tick(&mut self, app: &mut App, frame: u32) -> bool {
        match frame {
            5 => {
                // move: the time simulator walks to the next training-day morning.
                for _ in 0..7 {
                    app.pending.push(Action::NextMorning);
                    app.run_pending();
                    if app.view.day.is_training() {
                        break;
                    }
                }
                let days = (app.view.date - self.start_date).num_days();
                self.steps.push(Step {
                    step: "move",
                    passed: days >= 1 && app.view.day.is_training() && app.view.plan.is_some(),
                    detail: format!(
                        "time simulator moved {} → {} ({days} days); today's plan: {}",
                        self.start_date,
                        app.view.date,
                        app.view.day.title()
                    ),
                });
            }
            10 => {
                app.pending.push(Action::Start(SessionVariant::Full));
                app.pending.push(Action::ToggleWarmup(0));
            }
            f if (14..60).contains(&f) && f % 2 == 0 => {
                if app.open_session_id().is_some() && app.next_open_set().is_some() {
                    let before = app.view.sets.len();
                    app.pending.push(Action::NextSet);
                    app.run_pending();
                    if app.view.sets.len() > before {
                        self.sets_marked += 1;
                    }
                    self.sparks_seen = self.sparks_seen.max(app.sparks.len());
                }
            }
            62 => {
                app.pending.push(Action::AddMinutes(45));
                app.pending.push(Action::Close);
            }
            70 => {
                let done = app.view.session.as_ref().is_some_and(|s| s.is_done());
                let streak = app.view.forge.current;
                let xp = app.view.level.total_xp;
                self.steps.push(Step {
                    step: "act",
                    passed: done && self.sets_marked > 0 && xp > 0 && streak == 1 && self.sparks_seen > 0,
                    detail: format!(
                        "marked {} sets ({} sparks), closed the session: +{} XP, level {}, streak {}",
                        self.sets_marked, self.sparks_seen, app.last_outcome_xp, app.view.level.level, streak
                    ),
                });
            }
            110 => {
                let want = [
                    "heat",
                    "streak",
                    "level",
                    "session",
                    "exercises",
                    "warmup",
                    "habits",
                    "protein",
                    "nutrition",
                ];
                let missing: Vec<&str> = want
                    .iter()
                    .copied()
                    .filter(|k| !app.hud.contains(k))
                    .collect();
                self.steps.push(Step {
                    step: "hud",
                    passed: missing.is_empty(),
                    detail: if missing.is_empty() {
                        "forge heat, streak, level/XP, today's session, exercises, warm-up, habits, protein and \
                         nutrition targets are drawn"
                            .into()
                    } else {
                        format!("missing: {missing:?}")
                    },
                });
                write_results(&self.path, &self.steps);
                return true;
            }
            _ => {}
        }
        false
    }
}

async fn run(mode: Mode) {
    let fonts = Fonts::load();
    let headless = !matches!(mode, Mode::Normal);
    let mut app = App::new(headless);
    let mut test = match &mode {
        Mode::SelfTest(p) => Some(SelfTest {
            path: p.clone(),
            steps: Vec::new(),
            start_date: app.view.date,
            sets_marked: 0,
            sparks_seen: 0,
        }),
        _ => None,
    };
    let mut frame: u32 = 0;
    let mut t = 0.0f32;
    loop {
        let dt = get_frame_time().min(0.05);
        t += dt;
        frame += 1;
        #[cfg(windows)]
        if headless && matches!(frame, 1 | 3 | 10) {
            winhelp::send_to_back(TITLE);
        }
        if let Some(st) = test.as_mut() {
            if st.tick(&mut app, frame) {
                return;
            }
        }
        app.hud.clear();
        if let Mode::Gallery(p) = &mode {
            draw_gallery(&fonts, t);
            if frame >= 60 {
                save_screen(p);
                return;
            }
            next_frame().await;
            continue;
        }
        app.frame(&fonts, dt, t);
        if let Mode::Screenshot(p) = &mode {
            if frame >= 120 {
                save_screen(p);
                return;
            }
        }
        next_frame().await;
    }
}

fn conf() -> Conf {
    Conf {
        window_title: TITLE.to_string(),
        window_width: W as i32,
        window_height: H as i32,
        high_dpi: false,
        window_resizable: false,
        ..Default::default()
    }
}

fn main() {
    let mode = parse_mode();
    #[cfg(windows)]
    if !matches!(mode, Mode::Normal) {
        winhelp::remember_foreground();
    }
    macroquad::Window::from_config(conf(), run(mode));
}
