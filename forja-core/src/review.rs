//! Weekly review: plain rules → short, direct Chilean-Spanish text.

use crate::nutrition::PlateauAdvice;
use crate::plan::{Phase, PhaseInfo};
use crate::units::format_one_decimal;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeekSummary {
    pub week_start: NaiveDate,
    pub phase: PhaseInfo,
    pub next_phase: PhaseInfo,
    pub planned: u32,
    pub done: u32,
    pub minimum_versions: u32,
    pub recoveries: u32,
    pub misses: u32,
    pub avg_pain: Option<f64>,
    pub max_pain: Option<u8>,
    pub weight_avg_kg: Option<f64>,
    pub weight_avg_prev_kg: Option<f64>,
    pub protein_days: u32,
    pub perfect_habit_days: u32,
    pub xp_gained: u32,
    pub streak: u32,
    pub plateau: PlateauAdvice,
    pub red_flags: bool,
    /// The week isn't over yet (reviewed before Sunday night).
    pub in_progress: bool,
    /// Planned sessions from today on that aren't done yet.
    pub remaining: u32,
}

pub fn weekly_review(s: &WeekSummary) -> Vec<String> {
    let mut out = Vec::new();
    // Sessions.
    out.push(match (s.done, s.planned) {
        (d, p) if p > 0 && d >= p => format!("Semana completa: {d}/{p} sesiones. Así se forja."),
        (d, p) if s.in_progress => format!(
            "Semana en curso: {d}/{p} sesiones, quedan {} por hacer. La versión mínima también cuenta.",
            s.remaining
        ),
        (0, p) => format!(
            "0/{p} sesiones. No pasa nada grave si vuelves mañana: nunca dos semanas seguidas."
        ),
        (d, p) if d + 1 == p => format!(
            "{d}/{p} sesiones. Te faltó una; la próxima vez usa la versión mínima o recupérala."
        ),
        (d, p) => format!(
            "{d}/{p} sesiones. Esta semana busca al menos la versión mínima los días que cueste."
        ),
    });
    if s.minimum_versions > 0 {
        out.push(format!(
            "Usaste la versión mínima {} {}: bien, mantuviste la forja encendida.",
            s.minimum_versions,
            if s.minimum_versions == 1 {
                "vez"
            } else {
                "veces"
            }
        ));
    }
    if s.recoveries > 0 {
        out.push(format!(
            "Recuperaste {} sesión(es) en día de descanso. Eso es no fallar dos veces.",
            s.recoveries
        ));
    }
    // Back.
    if s.red_flags {
        out.push("Reportaste señales de alerta en la espalda: consulta a un médico o kinesiólogo antes de cargar peso.".into());
    } else if let Some(m) = s.max_pain {
        if m > 3 {
            out.push(format!(
                "Tu dolor lumbar llegó a {m}/10. La app cambiará los ejercicios de riesgo por alternativas mientras siga sobre 3."
            ));
        } else if let Some(a) = s.avg_pain {
            out.push(format!(
                "Espalda en verde: dolor promedio {}/10.",
                format_one_decimal(a)
            ));
        }
    }
    // Weight.
    match (s.weight_avg_kg, s.weight_avg_prev_kg) {
        (Some(now), Some(prev)) => {
            let delta = now - prev;
            if delta < 0.0 {
                out.push(format!(
                    "Peso promedio {} kg ({} kg vs. la semana pasada). Vas bien.",
                    format_one_decimal(now),
                    format_one_decimal(delta)
                ));
            } else {
                out.push(format!(
                    "Peso promedio {} kg (+{} kg). Una semana no define nada; mira la tendencia.",
                    format_one_decimal(now),
                    format_one_decimal(delta)
                ));
            }
        }
        (Some(now), None) => out.push(format!(
            "Peso promedio {} kg. Primera referencia registrada.",
            format_one_decimal(now)
        )),
        _ => out.push("Sin pesajes esta semana: pésate el domingo en ayunas.".into()),
    }
    if let PlateauAdvice::Stalled { suggest_kcal, .. } = s.plateau {
        out.push(format!(
            "El promedio de 2 semanas no bajó: te sugiero {suggest_kcal} kcal diarias (lo decides tú)."
        ));
    }
    // Habits.
    if s.protein_days >= 5 {
        out.push(format!(
            "Proteína completa {} días. Excelente para cuidar el músculo.",
            s.protein_days
        ));
    } else {
        out.push(format!(
            "Proteína completa {} días de 7. Apunta a 5: usa los botones rápidos.",
            s.protein_days
        ));
    }
    if s.perfect_habit_days > 0 {
        out.push(format!(
            "{} día(s) con todos los hábitos al 100 %.",
            s.perfect_habit_days
        ));
    }
    out.push(format!(
        "Ganaste {} XP. Racha actual: {} sesiones.",
        s.xp_gained, s.streak
    ));
    // Next week.
    match s.next_phase.phase {
        Phase::Deload if s.phase.phase != Phase::Deload => {
            out.push("La próxima semana es de descarga: la mitad de las series. Es parte del plan, no un retroceso.".into())
        }
        Phase::Variants if s.phase.phase != Phase::Variants => {
            out.push("La próxima semana empieza la Fase 2: llegan las variantes.".into())
        }
        Phase::Base if s.phase.phase == Phase::Deload => {
            out.push("La próxima semana empieza un ciclo nuevo de Fase 1.".into())
        }
        _ => {}
    }
    if s.next_phase.strength_test_week() {
        out.push("La próxima semana toca prueba de fuerza.".into());
    }
    out
}
