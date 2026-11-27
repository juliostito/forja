//! Exercise catalogue, warm-ups, stretches and the "Espalda feliz" mobility routine.
//! User-facing text is Chilean Spanish; identifiers are English.

use crate::profile::LoadMode;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Pattern {
    Push,
    Pull,
    Squat,
    Hinge,
    Core,
    Accessory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Measure {
    Reps,
    Seconds,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Progression {
    /// Double progression on load, then technique once the equipment tops out.
    Load,
    /// Bodyweight levels (index into the list); top of the range moves to the next level.
    Levels(&'static [&'static str]),
    /// Stay at the top of the range (bodyweight, holds, McGill work).
    Hold,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Exercise {
    pub id: &'static str,
    pub name: &'static str,
    pub sets: u8,
    pub min: u16,
    pub max: u16,
    pub measure: Measure,
    pub per_side: bool,
    pub load: LoadMode,
    /// Starting load per dumbbell in grams (0 for bodyweight).
    pub initial_g: u32,
    pub tempo: Option<&'static str>,
    pub technique: &'static str,
    pub back_note: &'static str,
    pub pattern: Pattern,
    pub progression: Progression,
    /// Compresses the spine (paused when red flags are active).
    pub axial_load: bool,
    /// Flexes the lumbar spine under load (crunch, sit-up). Must be false for every entry.
    pub lumbar_flexion: bool,
    /// A unilateral version exists as the last technique step.
    pub unilateral_option: bool,
    /// Phase 2 variant of this base exercise (inherits its load).
    pub variant_of: Option<&'static str>,
}

impl Exercise {
    pub fn is_isometric(&self) -> bool {
        self.measure == Measure::Seconds
    }

    /// Rest after a set, in seconds: 90 s, 45 s for isometrics.
    pub fn rest_seconds(&self) -> u16 {
        if self.is_isometric() {
            45
        } else {
            90
        }
    }

    /// "3 × 10–15 / lado" or "3 × 30–45 s".
    pub fn dose_label(&self, sets: u8, min: u16, max: u16) -> String {
        let range = if min == max {
            format!("{min}")
        } else {
            format!("{min}–{max}")
        };
        let unit = if self.measure == Measure::Seconds {
            " s"
        } else {
            ""
        };
        let side = if self.per_side { " / lado" } else { "" };
        format!("{sets} × {range}{unit}{side}")
    }
}

pub const PUSHUP_LEVELS: &[&str] = &[
    "Flexiones en pared",
    "Flexiones inclinadas en banca",
    "Flexiones inclinadas en escalón",
    "Flexiones de rodillas",
    "Flexiones en el suelo",
];
/// The program starts with hands on the bench.
pub const PUSHUP_START_LEVEL: u8 = 1;

#[allow(clippy::too_many_arguments)]
const fn ex(
    id: &'static str,
    name: &'static str,
    sets: u8,
    min: u16,
    max: u16,
    load: LoadMode,
    initial_g: u32,
    pattern: Pattern,
) -> Exercise {
    Exercise {
        id,
        name,
        sets,
        min,
        max,
        measure: Measure::Reps,
        per_side: false,
        load,
        initial_g,
        tempo: None,
        technique: "",
        back_note: "",
        pattern,
        progression: match load {
            LoadMode::Bodyweight => Progression::Hold,
            _ => Progression::Load,
        },
        axial_load: false,
        lumbar_flexion: false,
        unilateral_option: false,
        variant_of: None,
    }
}

use LoadMode::{Bodyweight as BW, Pair, Single};
use Pattern::*;

pub static EXERCISES: &[Exercise] = &[
    // ---- Upper A ----
    Exercise {
        technique:
            "Cuerpo en tabla, de talones a cabeza. Baja el pecho hasta el borde de la banca.",
        back_note: "Si la cadera se hunde, sube la inclinación.",
        progression: Progression::Levels(PUSHUP_LEVELS),
        ..ex(
            "incline_pushup",
            "Flexiones inclinadas (manos en banca)",
            3,
            8,
            15,
            BW,
            0,
            Push,
        )
    },
    Exercise {
        per_side: true,
        technique: "Lleva el codo hacia el bolsillo, no hacia la oreja.",
        back_note: "Mano y rodilla en la banca, espalda plana, sin rotar el tronco.",
        ..ex(
            "one_arm_row",
            "Remo a una mano apoyado en banca",
            3,
            10,
            15,
            Single,
            7000,
            Pull,
        )
    },
    Exercise {
        tempo: Some("3-1-1"),
        technique: "Baja en 3 s, pausa 1 s, sube en 1 s. Muñecas firmes sobre los codos.",
        back_note: "Pies firmes en el suelo y espalda apoyada; sin arquear para empujar.",
        unilateral_option: true,
        ..ex(
            "db_bench_press",
            "Press con mancuernas en banca",
            3,
            10,
            20,
            Pair,
            7000,
            Push,
        )
    },
    Exercise {
        technique: "Sube hasta la altura de los hombros, sin impulso, codos apenas doblados.",
        back_note: "Hazlas sentado en la banca si te balanceas.",
        ..ex(
            "lateral_raise",
            "Elevaciones laterales",
            2,
            12,
            20,
            Pair,
            2000,
            Accessory,
        )
    },
    Exercise {
        technique: "Codos pegados al cuerpo, sube y baja controlado.",
        back_note: "Sin balanceo: si necesitas impulso, el peso es mucho.",
        unilateral_option: true,
        ..ex(
            "biceps_curl",
            "Curl de bíceps",
            2,
            10,
            15,
            Pair,
            5000,
            Accessory,
        )
    },
    Exercise {
        technique: "Codos apuntando al techo; solo se mueven los antebrazos.",
        back_note: "Acostado en la banca, la lumbar queda apoyada.",
        ..ex(
            "lying_triceps_ext",
            "Extensión de tríceps acostado",
            2,
            10,
            15,
            Pair,
            4000,
            Accessory,
        )
    },
    Exercise {
        per_side: true,
        technique: "Brazo y pierna contraria se estiran lento mientras exhalas.",
        back_note: "La lumbar queda pegada al suelo todo el tiempo.",
        ..ex("dead_bug", "Dead bug", 2, 6, 10, BW, 0, Core)
    },
    // ---- Lower A ----
    Exercise {
        tempo: Some("3-1-1"),
        technique: "Mancuerna al pecho; siéntate hacia atrás hasta tocar la banca y sube.",
        back_note: "La banca marca una profundidad segura. Columna neutra.",
        axial_load: true,
        ..ex(
            "goblet_box_squat",
            "Sentadilla goblet a la banca",
            3,
            10,
            15,
            Single,
            9000,
            Squat,
        )
    },
    Exercise {
        technique: "Cadera hacia atrás, mancuernas rozando los muslos, rodillas suaves.",
        back_note: "Columna neutra siempre. Si duele, cambia ese día por hip thrust.",
        axial_load: true,
        ..ex(
            "romanian_deadlift",
            "Peso muerto rumano",
            3,
            10,
            15,
            Pair,
            8000,
            Hinge,
        )
    },
    Exercise {
        technique: "Hombros en la banca, empuja con los talones y aprieta glúteos arriba.",
        back_note: "Costillas abajo: no arquees la lumbar al subir.",
        unilateral_option: true,
        ..ex(
            "hip_thrust",
            "Hip thrust con hombros en banca",
            3,
            12,
            20,
            Single,
            9000,
            Hinge,
        )
    },
    Exercise {
        technique: "Sube lento, pausa 1 s arriba, baja completo.",
        back_note: "Apóyate con una mano en la pared para no compensar con la espalda.",
        ..ex(
            "calf_raise",
            "Elevación de talones con apoyo",
            2,
            15,
            25,
            BW,
            0,
            Accessory,
        )
    },
    Exercise {
        measure: Measure::Seconds,
        per_side: true,
        technique: "Rodillas, cadera y hombros en línea; codo bajo el hombro.",
        back_note: "Parte del Big 3 de McGill: estabiliza sin mover la columna.",
        ..ex(
            "side_plank_knees",
            "Plancha lateral sobre rodillas",
            2,
            20,
            30,
            BW,
            0,
            Core,
        )
    },
    // ---- Upper B ----
    Exercise {
        tempo: Some("2-2-1"),
        technique: "Baja hasta que los codos toquen el suelo, pausa 2 s, sube.",
        back_note: "En el suelo la espalda queda apoyada y protegida.",
        ..ex(
            "paused_floor_press",
            "Floor press con pausa",
            3,
            10,
            15,
            Pair,
            8000,
            Push,
        )
    },
    Exercise {
        technique: "Boca abajo en la banca, junta los omóplatos al subir.",
        back_note: "El remo más seguro para tu espalda: el pecho apoyado descarga la lumbar.",
        ..ex(
            "chest_supported_row",
            "Remo con pecho apoyado (boca abajo en banca)",
            3,
            10,
            15,
            Pair,
            7000,
            Pull,
        )
    },
    Exercise {
        technique: "Sentado, espalda contra la pared; sube sin arquear.",
        back_note: "Si te despegas de la pared o te arqueas, baja el peso.",
        axial_load: true,
        unilateral_option: true,
        ..ex(
            "seated_shoulder_press",
            "Press de hombros sentado contra la pared",
            3,
            8,
            12,
            Pair,
            5000,
            Push,
        )
    },
    Exercise {
        technique: "Una mancuerna con ambas manos; baja detrás de la cabeza con codos suaves.",
        back_note: "Costillas abajo: la lumbar no se despega de la banca.",
        ..ex(
            "bench_pullover",
            "Pullover en banca",
            2,
            12,
            15,
            Single,
            6000,
            Pull,
        )
    },
    Exercise {
        technique: "Boca abajo en la banca, abre los brazos como alas, pausa arriba.",
        back_note: "Pecho apoyado: cero carga en la lumbar.",
        ..ex(
            "reverse_fly",
            "Pájaros con pecho apoyado",
            2,
            15,
            20,
            Pair,
            2000,
            Pull,
        )
    },
    Exercise {
        technique: "Palmas mirándose, codos quietos.",
        back_note: "Sin balanceo del tronco.",
        unilateral_option: true,
        ..ex(
            "hammer_curl",
            "Curl martillo",
            2,
            10,
            15,
            Pair,
            6000,
            Accessory,
        )
    },
    Exercise {
        technique: "Manos juntas bajo el pecho, codos pegados al cuerpo.",
        back_note: "Cuerpo en tabla; si la cadera se hunde, sube la inclinación.",
        ..ex(
            "close_incline_pushup",
            "Flexiones cerradas inclinadas",
            2,
            8,
            15,
            BW,
            0,
            Push,
        )
    },
    // ---- Lower B ----
    Exercise {
        per_side: true,
        technique: "Una mano en la banca o la pared para equilibrio; baja en línea recta.",
        back_note: "Tronco erguido. Si molesta, cambia por sentadilla isométrica en pared.",
        axial_load: true,
        ..ex(
            "supported_split_squat",
            "Sentadilla split con apoyo",
            3,
            8,
            12,
            Pair,
            4000,
            Squat,
        )
    },
    Exercise {
        per_side: true,
        technique: "Una pierna estirada, empuja con el talón de la otra.",
        back_note: "Costillas abajo, sin arquear la lumbar.",
        ..ex(
            "single_leg_glute_bridge",
            "Puente de glúteo a una pierna",
            3,
            8,
            15,
            BW,
            0,
            Hinge,
        )
    },
    Exercise {
        per_side: true,
        technique: "El pie de atrás solo apoya la punta; cadera hacia atrás.",
        back_note: "Columna neutra. Si duele, cambia por puente de glúteo a una pierna.",
        axial_load: true,
        ..ex(
            "single_leg_rdl",
            "Peso muerto rumano a una pierna (pie de apoyo atrás)",
            3,
            8,
            12,
            Pair,
            6000,
            Hinge,
        )
    },
    Exercise {
        measure: Measure::Seconds,
        technique: "Espalda plana contra la pared, rodillas a 90° o un poco más arriba.",
        back_note: "La pared sostiene tu espalda: es la sentadilla más segura.",
        ..ex(
            "wall_sit",
            "Sentadilla isométrica en pared",
            3,
            30,
            45,
            BW,
            0,
            Squat,
        )
    },
    Exercise {
        per_side: true,
        technique: "Brazo y pierna contraria se estiran; pausa de 10 s en cada rep.",
        back_note: "Parte del Big 3 de McGill: la columna quieta como una mesa.",
        ..ex(
            "bird_dog",
            "Bird dog con pausa de 10 s",
            2,
            5,
            6,
            BW,
            0,
            Core,
        )
    },
    Exercise {
        technique:
            "Una pierna doblada, manos bajo la lumbar; despega cabeza y hombros unos cm, 10 s.",
        back_note: "No es un abdominal: la lumbar no se flexiona.",
        ..ex(
            "mcgill_curl_up",
            "Curl-up de McGill (10 s cada rep)",
            2,
            5,
            6,
            BW,
            0,
            Core,
        )
    },
    // ---- Minimum version / substitutes ----
    Exercise {
        technique: "Talones cerca de la cadera, empuja con los talones y aprieta glúteos.",
        back_note: "Costillas abajo, sin arquear la lumbar.",
        ..ex("glute_bridge", "Puente de glúteo", 3, 12, 20, BW, 0, Hinge)
    },
    Exercise {
        technique: "Siéntate en una superficie más alta que la banca y párate sin peso.",
        back_note: "Menos profundidad = menos exigencia para la lumbar.",
        ..ex(
            "high_box_squat_bw",
            "Sentadilla a banca alta sin peso",
            3,
            10,
            15,
            BW,
            0,
            Squat,
        )
    },
    // ---- Phase 2 variants ----
    Exercise {
        tempo: Some("3-1-1"),
        technique: "Baja, sube hasta la mitad, vuelve a bajar y sube completo: eso es 1 rep.",
        back_note: "La banca sigue marcando la profundidad. Columna neutra.",
        axial_load: true,
        variant_of: Some("goblet_box_squat"),
        ..ex(
            "goblet_1_5",
            "Sentadilla goblet 1½",
            3,
            8,
            12,
            Single,
            9000,
            Squat,
        )
    },
    Exercise {
        per_side: true,
        technique: "Una pierna en el aire, empuja con el talón de la otra.",
        back_note: "Costillas abajo; la cadera sube recta, sin girar.",
        variant_of: Some("hip_thrust"),
        ..ex(
            "single_leg_hip_thrust",
            "Hip thrust a una pierna",
            3,
            8,
            15,
            Single,
            9000,
            Hinge,
        )
    },
    Exercise {
        tempo: Some("3-2-1"),
        technique: "Baja en 3 s, pausa 2 s, sube a la mitad, vuelve a bajar y sube completo.",
        back_note: "Pies firmes, espalda apoyada.",
        variant_of: Some("db_bench_press"),
        ..ex(
            "db_bench_1_5_pause",
            "Press en banca 1½ con pausa",
            3,
            8,
            15,
            Pair,
            7000,
            Push,
        )
    },
    Exercise {
        per_side: true,
        technique: "Sentado contra la pared, un brazo a la vez; el otro en la cintura.",
        back_note: "Sin inclinarte hacia el lado: si pasa, baja el peso.",
        axial_load: true,
        variant_of: Some("seated_shoulder_press"),
        ..ex(
            "single_arm_shoulder_press",
            "Press de hombro a un brazo",
            3,
            8,
            12,
            Single,
            5000,
            Push,
        )
    },
    Exercise {
        technique: "Sube, mantén 3 s con los omóplatos juntos, baja lento.",
        back_note: "Pecho apoyado: el remo más seguro para tu espalda.",
        variant_of: Some("chest_supported_row"),
        ..ex(
            "chest_supported_row_pause",
            "Remo con pecho apoyado y pausa de 3 s",
            3,
            8,
            12,
            Pair,
            7000,
            Pull,
        )
    },
    Exercise {
        tempo: Some("4-0-1"),
        technique: "Baja en 4 s con la cadera atrás, sube en 1 s.",
        back_note: "Columna neutra siempre. Si duele, cambia ese día por hip thrust.",
        axial_load: true,
        variant_of: Some("romanian_deadlift"),
        ..ex(
            "rdl_tempo4",
            "Peso muerto rumano tempo 4 s",
            3,
            8,
            12,
            Pair,
            8000,
            Hinge,
        )
    },
    Exercise {
        per_side: true,
        technique: "Empeine de atrás sobre la banca; baja recto con el tronco erguido.",
        back_note: "Si molesta la espalda, cambia por sentadilla isométrica en pared.",
        axial_load: true,
        variant_of: Some("supported_split_squat"),
        ..ex(
            "bulgarian_split_squat",
            "Sentadilla búlgara",
            3,
            8,
            12,
            Pair,
            4000,
            Squat,
        )
    },
];

pub fn exercise(id: &str) -> Option<&'static Exercise> {
    EXERCISES.iter().find(|e| e.id == id)
}

/// How much of a warm-up / stretch item to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Dose {
    Seconds(u16),
    SecondsPerSide(u16),
    Reps(u16),
    RepsPerSide(u16),
    /// "10 + 10" (each direction).
    RepsEachWay(u16),
    /// reps × seconds hold.
    Holds {
        reps: u16,
        secs: u16,
    },
    HoldsPerSide {
        reps: u16,
        secs: u16,
    },
    Sets {
        sets: u16,
        secs: u16,
    },
    SetsPerSide {
        sets: u16,
        secs: u16,
    },
}

impl Dose {
    pub fn label(&self) -> String {
        match *self {
            Dose::Seconds(s) => format!("{s} s"),
            Dose::SecondsPerSide(s) => format!("{s} s / lado"),
            Dose::Reps(r) => format!("×{r}"),
            Dose::RepsPerSide(r) => format!("{r} / lado"),
            Dose::RepsEachWay(r) => format!("{r} + {r}"),
            Dose::Holds { reps, secs } => format!("{reps} × {secs} s"),
            Dose::HoldsPerSide { reps, secs } => format!("{reps} × {secs} s / lado"),
            Dose::Sets { sets, secs } => format!("{sets} × {secs} s"),
            Dose::SetsPerSide { sets, secs } => format!("{sets} × {secs} s / lado"),
        }
    }

    /// Rough duration for the built-in timer (estimated: 3 s per rep).
    pub fn seconds(&self) -> u32 {
        let s = match *self {
            Dose::Seconds(s) => s as u32,
            Dose::SecondsPerSide(s) => 2 * s as u32,
            Dose::Reps(r) => 3 * r as u32,
            Dose::RepsPerSide(r) => 6 * r as u32,
            Dose::RepsEachWay(r) => 4 * r as u32,
            Dose::Holds { reps, secs } => (reps as u32) * (secs as u32 + 2),
            Dose::HoldsPerSide { reps, secs } => 2 * (reps as u32) * (secs as u32 + 2),
            Dose::Sets { sets, secs } => (sets as u32) * (secs as u32 + 5),
            Dose::SetsPerSide { sets, secs } => 2 * (sets as u32) * (secs as u32 + 5),
        };
        s.max(10)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct RoutineItem {
    pub id: &'static str,
    pub name: &'static str,
    pub dose: Dose,
    pub cue: &'static str,
}

const fn item(id: &'static str, name: &'static str, dose: Dose, cue: &'static str) -> RoutineItem {
    RoutineItem {
        id,
        name,
        dose,
        cue,
    }
}

pub static WARMUP_BASE: &[RoutineItem] = &[
    item(
        "march",
        "Marcha en el lugar",
        Dose::Seconds(90),
        "Rodillas arriba, brazos sueltos.",
    ),
    item(
        "cat_camel",
        "Gato-camello",
        Dose::Reps(8),
        "Movimiento suave, sin forzar los extremos.",
    ),
    item(
        "warm_curl_up",
        "Curl-up de McGill",
        Dose::Holds { reps: 5, secs: 10 },
        "La lumbar no se flexiona.",
    ),
    item(
        "warm_side_plank",
        "Plancha lateral sobre rodillas",
        Dose::HoldsPerSide { reps: 3, secs: 10 },
        "Cadera alta, en línea.",
    ),
    item(
        "warm_bird_dog",
        "Bird dog",
        Dose::HoldsPerSide { reps: 4, secs: 10 },
        "Columna quieta como una mesa.",
    ),
];

pub static WARMUP_UPPER: &[RoutineItem] = &[
    item(
        "arm_circles",
        "Círculos de brazos",
        Dose::RepsEachWay(10),
        "Hacia adelante y hacia atrás.",
    ),
    item(
        "wall_pushups",
        "Flexiones en pared",
        Dose::Reps(12),
        "Cuerpo en tabla.",
    ),
];

pub static WARMUP_LOWER: &[RoutineItem] = &[
    item(
        "box_squat_bw",
        "Sentadilla a banca sin peso",
        Dose::Reps(10),
        "Toca la banca y sube.",
    ),
    item(
        "warm_glute_bridge",
        "Puente de glúteo",
        Dose::Reps(12),
        "Costillas abajo.",
    ),
    item(
        "leg_swings",
        "Balanceo de pierna",
        Dose::RepsPerSide(10),
        "Apóyate en la pared.",
    ),
    item(
        "hinge_drill",
        "Práctica de bisagra",
        Dose::Reps(10),
        "Cadera atrás, espalda neutra.",
    ),
];

pub static STRETCH_UPPER: &[RoutineItem] = &[
    item(
        "pec_wall",
        "Pectoral en pared",
        Dose::SecondsPerSide(30),
        "Brazo en la pared, gira el cuerpo suave.",
    ),
    item(
        "lat_bench",
        "Dorsal en banca",
        Dose::Sets { sets: 2, secs: 30 },
        "Codos en la banca, cadera atrás.",
    ),
    item(
        "triceps_stretch",
        "Tríceps",
        Dose::SecondsPerSide(30),
        "Codo arriba, mano a la espalda.",
    ),
    item(
        "breath_46",
        "Respiración 4-6",
        Dose::Seconds(60),
        "Inhala 4 s, exhala 6 s.",
    ),
];

pub static STRETCH_LOWER: &[RoutineItem] = &[
    item(
        "hip_flexor",
        "Flexor de cadera semi-arrodillado",
        Dose::SetsPerSide { sets: 2, secs: 30 },
        "Aprieta el glúteo de la rodilla apoyada.",
    ),
    item(
        "hamstring_lying",
        "Isquiotibiales acostado",
        Dose::SecondsPerSide(30),
        "Pierna arriba con una toalla.",
    ),
    item(
        "figure_four",
        "Glúteo en figura 4",
        Dose::SecondsPerSide(30),
        "Tobillo sobre la rodilla contraria.",
    ),
    item(
        "calf_wall",
        "Pantorrilla en pared",
        Dose::SecondsPerSide(30),
        "Talón de atrás en el suelo.",
    ),
    item(
        "breath_60",
        "Respiración",
        Dose::Seconds(60),
        "Lenta, por la nariz.",
    ),
];

/// "Espalda feliz": ~10 min mobility for rest days (composition estimated from the brief's items).
pub static HAPPY_BACK: &[RoutineItem] = &[
    item(
        "hb_march",
        "Marcha en el lugar",
        Dose::Seconds(120),
        "Suelta el cuerpo.",
    ),
    item(
        "hb_cat_camel",
        "Gato-camello",
        Dose::Reps(10),
        "Sin forzar los extremos.",
    ),
    item(
        "hb_curl_up",
        "Curl-up de McGill",
        Dose::Holds { reps: 5, secs: 10 },
        "La lumbar no se flexiona.",
    ),
    item(
        "hb_side_plank",
        "Plancha lateral sobre rodillas",
        Dose::HoldsPerSide { reps: 3, secs: 10 },
        "Cadera alta.",
    ),
    item(
        "hb_bird_dog",
        "Bird dog",
        Dose::HoldsPerSide { reps: 5, secs: 10 },
        "Columna quieta.",
    ),
    item(
        "hb_hip_flexor",
        "Flexor de cadera semi-arrodillado",
        Dose::SecondsPerSide(30),
        "Glúteo apretado.",
    ),
    item(
        "hb_figure_four",
        "Glúteo en figura 4",
        Dose::SecondsPerSide(30),
        "Suave.",
    ),
    item(
        "hb_breath",
        "Respiración 4-6",
        Dose::Seconds(60),
        "Inhala 4 s, exhala 6 s.",
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_variants_resolve() {
        for (i, a) in EXERCISES.iter().enumerate() {
            for b in &EXERCISES[i + 1..] {
                assert_ne!(a.id, b.id);
            }
            if let Some(base) = a.variant_of {
                assert!(exercise(base).is_some(), "{} → {}", a.id, base);
            }
            assert!(a.min <= a.max);
            assert!(
                !a.technique.is_empty() && !a.back_note.is_empty(),
                "{}",
                a.id
            );
        }
    }

    #[test]
    fn no_loaded_lumbar_flexion_anywhere() {
        assert!(EXERCISES.iter().all(|e| !e.lumbar_flexion));
        for banned in ["crunch", "sit_up", "situp", "abdominal"] {
            assert!(EXERCISES.iter().all(|e| !e.id.contains(banned)));
        }
    }

    #[test]
    fn rest_and_labels() {
        let wall = exercise("wall_sit").unwrap();
        assert_eq!(wall.rest_seconds(), 45);
        assert_eq!(wall.dose_label(3, 30, 45), "3 × 30–45 s");
        let row = exercise("one_arm_row").unwrap();
        assert_eq!(row.rest_seconds(), 90);
        assert_eq!(row.dose_label(3, 10, 15), "3 × 10–15 / lado");
        assert_eq!(Dose::RepsEachWay(10).label(), "10 + 10");
        let happy: u32 = HAPPY_BACK.iter().map(|i| i.dose.seconds()).sum();
        assert!(
            (480..=780).contains(&happy),
            "Espalda feliz ≈ 10 min, got {happy} s"
        );
    }
}
