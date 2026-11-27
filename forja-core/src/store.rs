//! SQLite storage with a versioned schema (`PRAGMA user_version`) and ordered migrations.

use crate::meal::Meal;
use crate::model::*;
use crate::profile::{Contract, Profile};
use crate::units::Grams;
use crate::xp::{XpEvent, XpReason};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("base de datos: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("formato: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
    #[error("la base de datos es de una versión más nueva ({found}); esta app entiende hasta {supported}")]
    SchemaTooNew { found: u32, supported: u32 },
}

pub type Result<T> = std::result::Result<T, Error>;

/// Migrations in order; index + 1 = schema version.
pub const MIGRATIONS: &[&str] = &[
    // v1 — milestone 1: core records.
    r#"
    CREATE TABLE profile (id INTEGER PRIMARY KEY CHECK (id = 1), data TEXT NOT NULL);
    CREATE TABLE contract (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        why TEXT NOT NULL, commitment TEXT NOT NULL, penalty TEXT NOT NULL,
        partner TEXT, signed_on TEXT
    );
    CREATE TABLE session_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        date TEXT NOT NULL, kind TEXT NOT NULL, session_type TEXT NOT NULL,
        recovered_from TEXT, started_at TEXT NOT NULL, ended_at TEXT,
        pain INTEGER CHECK (pain BETWEEN 0 AND 10), effort INTEGER CHECK (effort BETWEEN 0 AND 10),
        note TEXT NOT NULL DEFAULT '', source TEXT NOT NULL,
        warmup_done INTEGER NOT NULL DEFAULT 0, stretches_done INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX session_log_date ON session_log(date);
    CREATE TABLE set_log (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id INTEGER NOT NULL REFERENCES session_log(id) ON DELETE CASCADE,
        exercise_id TEXT NOT NULL, set_index INTEGER NOT NULL,
        reps INTEGER NOT NULL, load_g INTEGER NOT NULL, ok INTEGER NOT NULL,
        watch_verified INTEGER NOT NULL DEFAULT 0,
        UNIQUE (session_id, exercise_id, set_index)
    );
    CREATE TABLE habit_day (
        date TEXT PRIMARY KEY, steps INTEGER,
        water INTEGER NOT NULL DEFAULT 0, vegetables INTEGER NOT NULL DEFAULT 0,
        no_sugar_alcohol INTEGER NOT NULL DEFAULT 0, mcgill_big3 INTEGER NOT NULL DEFAULT 0,
        sleep_7h INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE protein_entry (
        id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT NOT NULL, at TEXT NOT NULL,
        grams INTEGER NOT NULL, label TEXT NOT NULL
    );
    CREATE INDEX protein_entry_date ON protein_entry(date);
    CREATE TABLE weight (date TEXT PRIMARY KEY, grams INTEGER NOT NULL, at TEXT NOT NULL);
    CREATE TABLE strength_test (
        id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT NOT NULL,
        pushups_max INTEGER, pushup_level INTEGER, box_squats_60s INTEGER,
        side_plank_worst_s INTEGER, waist_mm INTEGER, weight_g INTEGER
    );
    -- Only a reference to a file on the device; image bytes never enter the database.
    CREATE TABLE photo (id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT NOT NULL, local_ref TEXT NOT NULL);
    CREATE TABLE xp_event (
        id INTEGER PRIMARY KEY AUTOINCREMENT, at TEXT NOT NULL, date TEXT NOT NULL,
        reason TEXT NOT NULL, base INTEGER NOT NULL, multiplier_pct INTEGER NOT NULL,
        amount INTEGER NOT NULL, ref_id INTEGER, note TEXT NOT NULL DEFAULT ''
    );
    CREATE TRIGGER xp_event_no_update BEFORE UPDATE ON xp_event
        BEGIN SELECT RAISE(ABORT, 'xp_event is append-only'); END;
    CREATE TRIGGER xp_event_no_delete BEFORE DELETE ON xp_event
        BEGIN SELECT RAISE(ABORT, 'xp_event is append-only'); END;
    CREATE TABLE health_snapshot (
        date TEXT PRIMARY KEY, steps INTEGER, active_kcal INTEGER, exercise_min INTEGER,
        resting_hr INTEGER, hrv_ms INTEGER, sleep_min INTEGER, weight_g INTEGER
    );
    CREATE TABLE back_check (
        date TEXT PRIMARY KEY, radiating_leg_pain INTEGER NOT NULL,
        tingling INTEGER NOT NULL, weakness INTEGER NOT NULL
    );
    "#,
    // v2 — game tables, filled by milestone 2.
    r#"
    CREATE TABLE quest (
        id TEXT PRIMARY KEY, scope TEXT NOT NULL, period_start TEXT NOT NULL, kind TEXT NOT NULL,
        target INTEGER NOT NULL, progress INTEGER NOT NULL DEFAULT 0, completed_at TEXT
    );
    CREATE TABLE boss_week (
        week_start TEXT PRIMARY KEY, boss_id TEXT NOT NULL, hp_pct INTEGER NOT NULL, defeated_at TEXT
    );
    CREATE TABLE achievement (id TEXT PRIMARY KEY, unlocked_at TEXT NOT NULL);
    CREATE TABLE skill_node (id TEXT PRIMARY KEY, unlocked_at TEXT NOT NULL);
    CREATE TABLE loot_item (id TEXT PRIMARY KEY, obtained_at TEXT NOT NULL, equipped INTEGER NOT NULL DEFAULT 0);
    "#,
    // v3 — meals logged from a photo (Claude) or by hand. Only a reference to the photo is stored.
    r#"
    CREATE TABLE meal (
        id INTEGER PRIMARY KEY AUTOINCREMENT, date TEXT NOT NULL, at TEXT NOT NULL,
        description TEXT NOT NULL, kcal INTEGER NOT NULL, protein_g INTEGER NOT NULL,
        carbs_g INTEGER NOT NULL, fat_g INTEGER NOT NULL, source TEXT NOT NULL,
        confidence TEXT NOT NULL DEFAULT '', photo_ref TEXT, detail TEXT NOT NULL DEFAULT ''
    );
    CREATE INDEX meal_date ON meal(date);
    "#,
];

pub const SCHEMA_VERSION: u32 = MIGRATIONS.len() as u32;

fn date_s(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn parse_date(s: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn instant_s(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn parse_instant(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

fn bad_enum(what: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("unknown {what}").into(),
    )
}

/// Full backup, serialised with serde (export / import).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Backup {
    pub app: String,
    pub schema_version: u32,
    pub profile: Option<Profile>,
    pub contract: Option<Contract>,
    pub sessions: Vec<SessionLog>,
    pub sets: Vec<SetLog>,
    pub habit_days: Vec<HabitDay>,
    pub protein: Vec<ProteinEntry>,
    pub weights: Vec<WeightEntry>,
    pub strength_tests: Vec<StrengthTest>,
    pub back_checks: Vec<BackCheck>,
    pub xp_events: Vec<XpEvent>,
    #[serde(default)]
    pub meals: Vec<Meal>,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut s = Store {
            conn: Connection::open(path)?,
        };
        s.init()?;
        Ok(s)
    }

    pub fn open_in_memory() -> Result<Self> {
        let mut s = Store {
            conn: Connection::open_in_memory()?,
        };
        s.init()?;
        Ok(s)
    }

    fn init(&mut self) -> Result<()> {
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate_to(&mut self.conn, SCHEMA_VERSION)
    }

    /// For migration tests: open without migrating.
    pub fn from_connection(conn: Connection) -> Result<Self> {
        let mut s = Store { conn };
        s.init()?;
        Ok(s)
    }

    pub fn schema_version(&self) -> Result<u32> {
        schema_version(&self.conn)
    }

    // ---- profile & contract ----
    pub fn save_profile(&self, p: &Profile) -> Result<()> {
        let json = serde_json::to_string(p)?;
        self.conn.execute(
            "INSERT INTO profile (id, data) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            params![json],
        )?;
        Ok(())
    }

    pub fn load_profile(&self) -> Result<Option<Profile>> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT data FROM profile WHERE id = 1", [], |r| r.get(0))
            .optional()?;
        Ok(match json {
            Some(j) => Some(serde_json::from_str(&j)?),
            None => None,
        })
    }

    pub fn save_contract(&self, c: &Contract) -> Result<()> {
        self.conn.execute(
            "INSERT INTO contract (id, why, commitment, penalty, partner, signed_on) VALUES (1, ?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET why = excluded.why, commitment = excluded.commitment,
             penalty = excluded.penalty, partner = excluded.partner, signed_on = excluded.signed_on",
            params![c.why, c.commitment, c.penalty, c.partner, c.signed_on.map(date_s)],
        )?;
        Ok(())
    }

    pub fn load_contract(&self) -> Result<Option<Contract>> {
        Ok(self
            .conn
            .query_row(
                "SELECT why, commitment, penalty, partner, signed_on FROM contract WHERE id = 1",
                [],
                |r| {
                    let signed: Option<String> = r.get(4)?;
                    Ok(Contract {
                        why: r.get(0)?,
                        commitment: r.get(1)?,
                        penalty: r.get(2)?,
                        partner: r.get(3)?,
                        signed_on: signed.as_deref().map(parse_date).transpose()?,
                    })
                },
            )
            .optional()?)
    }

    // ---- sessions ----
    pub fn insert_session(&self, s: &SessionLog) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO session_log (date, kind, session_type, recovered_from, started_at, ended_at, pain, effort,
             note, source, warmup_done, stretches_done) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                date_s(s.date),
                s.kind.as_str(),
                s.session_type.as_str(),
                s.recovered_from.map(date_s),
                instant_s(s.started_at),
                s.ended_at.map(instant_s),
                s.pain,
                s.effort,
                s.note,
                s.source.as_str(),
                s.warmup_done,
                s.stretches_done
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_session(&self, s: &SessionLog) -> Result<()> {
        let n = self.conn.execute(
            "UPDATE session_log SET ended_at = ?2, pain = ?3, effort = ?4, note = ?5, warmup_done = ?6,
             stretches_done = ?7, session_type = ?8 WHERE id = ?1",
            params![
                s.id,
                s.ended_at.map(instant_s),
                s.pain,
                s.effort,
                s.note,
                s.warmup_done,
                s.stretches_done,
                s.session_type.as_str()
            ],
        )?;
        if n == 1 {
            Ok(())
        } else {
            Err(Error::Invalid(format!("no existe la sesión {}", s.id)))
        }
    }

    fn session_from_row(r: &Row<'_>) -> rusqlite::Result<SessionLog> {
        let kind: String = r.get(2)?;
        let ty: String = r.get(3)?;
        let rec: Option<String> = r.get(4)?;
        let start: String = r.get(5)?;
        let end: Option<String> = r.get(6)?;
        let source: String = r.get(10)?;
        Ok(SessionLog {
            id: r.get(0)?,
            date: parse_date(&r.get::<_, String>(1)?)?,
            kind: SessionKind::parse(&kind).ok_or_else(|| bad_enum("session kind"))?,
            session_type: SessionType::parse(&ty).ok_or_else(|| bad_enum("session type"))?,
            recovered_from: rec.as_deref().map(parse_date).transpose()?,
            started_at: parse_instant(&start)?,
            ended_at: end.as_deref().map(parse_instant).transpose()?,
            pain: r.get(7)?,
            effort: r.get(8)?,
            note: r.get(9)?,
            source: Source::parse(&source).ok_or_else(|| bad_enum("source"))?,
            warmup_done: r.get(11)?,
            stretches_done: r.get(12)?,
        })
    }

    const SESSION_COLS: &'static str =
        "id, date, kind, session_type, recovered_from, started_at, ended_at, pain, effort, \
        note, source, warmup_done, stretches_done";

    pub fn session(&self, id: i64) -> Result<Option<SessionLog>> {
        let sql = format!(
            "SELECT {} FROM session_log WHERE id = ?1",
            Self::SESSION_COLS
        );
        Ok(self
            .conn
            .query_row(&sql, params![id], Self::session_from_row)
            .optional()?)
    }

    /// All sessions, chronological.
    pub fn sessions(&self) -> Result<Vec<SessionLog>> {
        let sql = format!(
            "SELECT {} FROM session_log ORDER BY date, started_at, id",
            Self::SESSION_COLS
        );
        let mut st = self.conn.prepare(&sql)?;
        let rows = st.query_map([], Self::session_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_set(&self, s: &SetLog) -> Result<()> {
        self.conn.execute(
            "INSERT INTO set_log (session_id, exercise_id, set_index, reps, load_g, ok, watch_verified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(session_id, exercise_id, set_index) DO UPDATE SET reps = excluded.reps,
             load_g = excluded.load_g, ok = excluded.ok, watch_verified = excluded.watch_verified",
            params![s.session_id, s.exercise_id, s.index, s.reps, s.load.0, s.ok, s.watch_verified],
        )?;
        Ok(())
    }

    pub fn delete_set(&self, session_id: i64, exercise_id: &str, index: u8) -> Result<()> {
        self.conn.execute(
            "DELETE FROM set_log WHERE session_id = ?1 AND exercise_id = ?2 AND set_index = ?3",
            params![session_id, exercise_id, index],
        )?;
        Ok(())
    }

    fn set_from_row(r: &Row<'_>) -> rusqlite::Result<SetLog> {
        Ok(SetLog {
            session_id: r.get(0)?,
            exercise_id: r.get(1)?,
            index: r.get(2)?,
            reps: r.get(3)?,
            load: Grams(r.get(4)?),
            ok: r.get(5)?,
            watch_verified: r.get(6)?,
        })
    }

    pub fn sets_for_session(&self, session_id: i64) -> Result<Vec<SetLog>> {
        let mut st = self.conn.prepare(
            "SELECT session_id, exercise_id, set_index, reps, load_g, ok, watch_verified FROM set_log
             WHERE session_id = ?1 ORDER BY id",
        )?;
        let rows = st.query_map(params![session_id], Self::set_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn all_sets(&self) -> Result<Vec<SetLog>> {
        let mut st = self.conn.prepare(
            "SELECT session_id, exercise_id, set_index, reps, load_g, ok, watch_verified FROM set_log ORDER BY id",
        )?;
        let rows = st.query_map([], Self::set_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ---- habits, protein, weight ----
    pub fn upsert_habit_day(&self, date: NaiveDate, h: &HabitDay) -> Result<()> {
        self.conn.execute(
            "INSERT INTO habit_day (date, steps, water, vegetables, no_sugar_alcohol, mcgill_big3, sleep_7h)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT(date) DO UPDATE SET steps = excluded.steps,
             water = excluded.water, vegetables = excluded.vegetables, no_sugar_alcohol = excluded.no_sugar_alcohol,
             mcgill_big3 = excluded.mcgill_big3, sleep_7h = excluded.sleep_7h",
            params![date_s(date), h.steps, h.water, h.vegetables, h.no_sugar_alcohol, h.mcgill_big3, h.sleep_7h],
        )?;
        Ok(())
    }

    fn habit_from_row(r: &Row<'_>) -> rusqlite::Result<HabitDay> {
        Ok(HabitDay {
            date: Some(parse_date(&r.get::<_, String>(0)?)?),
            steps: r.get(1)?,
            water: r.get(2)?,
            vegetables: r.get(3)?,
            no_sugar_alcohol: r.get(4)?,
            mcgill_big3: r.get(5)?,
            sleep_7h: r.get(6)?,
        })
    }

    pub fn habit_day(&self, date: NaiveDate) -> Result<Option<HabitDay>> {
        Ok(self
            .conn
            .query_row(
                "SELECT date, steps, water, vegetables, no_sugar_alcohol, mcgill_big3, sleep_7h FROM habit_day
                 WHERE date = ?1",
                params![date_s(date)],
                Self::habit_from_row,
            )
            .optional()?)
    }

    pub fn habit_days(&self) -> Result<Vec<HabitDay>> {
        let mut st = self.conn.prepare(
            "SELECT date, steps, water, vegetables, no_sugar_alcohol, mcgill_big3, sleep_7h FROM habit_day ORDER BY date",
        )?;
        let rows = st.query_map([], Self::habit_from_row)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn add_protein(&self, e: &ProteinEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO protein_entry (date, at, grams, label) VALUES (?1, ?2, ?3, ?4)",
            params![date_s(e.date), instant_s(e.at), e.grams, e.label],
        )?;
        Ok(())
    }

    pub fn protein_entries(&self) -> Result<Vec<ProteinEntry>> {
        let mut st = self
            .conn
            .prepare("SELECT date, at, grams, label FROM protein_entry ORDER BY at, id")?;
        let rows = st.query_map([], |r| {
            Ok(ProteinEntry {
                date: parse_date(&r.get::<_, String>(0)?)?,
                at: parse_instant(&r.get::<_, String>(1)?)?,
                grams: r.get(2)?,
                label: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn protein_for(&self, date: NaiveDate) -> Result<u32> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(SUM(grams), 0) FROM protein_entry WHERE date = ?1",
            params![date_s(date)],
            |r| r.get(0),
        )?)
    }

    pub fn upsert_weight(&self, w: &WeightEntry) -> Result<()> {
        self.conn.execute(
            "INSERT INTO weight (date, grams, at) VALUES (?1, ?2, ?3)
             ON CONFLICT(date) DO UPDATE SET grams = excluded.grams, at = excluded.at",
            params![date_s(w.date), w.weight.0, instant_s(w.at)],
        )?;
        Ok(())
    }

    pub fn weights(&self) -> Result<Vec<WeightEntry>> {
        let mut st = self
            .conn
            .prepare("SELECT date, grams, at FROM weight ORDER BY date")?;
        let rows = st.query_map([], |r| {
            Ok(WeightEntry {
                date: parse_date(&r.get::<_, String>(0)?)?,
                weight: Grams(r.get(1)?),
                at: parse_instant(&r.get::<_, String>(2)?)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn add_strength_test(&self, t: &StrengthTest) -> Result<()> {
        let date = t
            .date
            .ok_or_else(|| Error::Invalid("la prueba necesita fecha".into()))?;
        self.conn.execute(
            "INSERT INTO strength_test (date, pushups_max, pushup_level, box_squats_60s, side_plank_worst_s,
             waist_mm, weight_g) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                date_s(date),
                t.pushups_max,
                t.pushup_level,
                t.box_squats_60s,
                t.side_plank_worst_s,
                t.waist_mm,
                t.weight.map(|g| g.0)
            ],
        )?;
        Ok(())
    }

    pub fn strength_tests(&self) -> Result<Vec<StrengthTest>> {
        let mut st = self.conn.prepare(
            "SELECT date, pushups_max, pushup_level, box_squats_60s, side_plank_worst_s, waist_mm, weight_g
             FROM strength_test ORDER BY date, id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(StrengthTest {
                date: Some(parse_date(&r.get::<_, String>(0)?)?),
                pushups_max: r.get(1)?,
                pushup_level: r.get(2)?,
                box_squats_60s: r.get(3)?,
                side_plank_worst_s: r.get(4)?,
                waist_mm: r.get(5)?,
                weight: r.get::<_, Option<u32>>(6)?.map(Grams),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn upsert_back_check(&self, c: &BackCheck) -> Result<()> {
        let date = c
            .date
            .ok_or_else(|| Error::Invalid("el registro necesita fecha".into()))?;
        self.conn.execute(
            "INSERT INTO back_check (date, radiating_leg_pain, tingling, weakness) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(date) DO UPDATE SET radiating_leg_pain = excluded.radiating_leg_pain,
             tingling = excluded.tingling, weakness = excluded.weakness",
            params![date_s(date), c.radiating_leg_pain, c.tingling, c.weakness],
        )?;
        Ok(())
    }

    pub fn back_checks(&self) -> Result<Vec<BackCheck>> {
        let mut st = self.conn.prepare(
            "SELECT date, radiating_leg_pain, tingling, weakness FROM back_check ORDER BY date",
        )?;
        let rows = st.query_map([], |r| {
            Ok(BackCheck {
                date: Some(parse_date(&r.get::<_, String>(0)?)?),
                radiating_leg_pain: r.get(1)?,
                tingling: r.get(2)?,
                weakness: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ---- XP ledger ----
    pub fn add_xp_event(&self, e: &XpEvent) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO xp_event (at, date, reason, base, multiplier_pct, amount, ref_id, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                instant_s(e.at),
                date_s(e.date),
                e.reason.as_str(),
                e.base,
                e.multiplier_pct,
                e.amount,
                e.ref_id,
                e.note
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn xp_events(&self) -> Result<Vec<XpEvent>> {
        let mut st = self.conn.prepare(
            "SELECT id, at, date, reason, base, multiplier_pct, amount, ref_id, note FROM xp_event ORDER BY id",
        )?;
        let rows = st.query_map([], |r| {
            let reason: String = r.get(3)?;
            Ok(XpEvent {
                id: r.get(0)?,
                at: parse_instant(&r.get::<_, String>(1)?)?,
                date: parse_date(&r.get::<_, String>(2)?)?,
                reason: XpReason::parse(&reason).ok_or_else(|| bad_enum("xp reason"))?,
                base: r.get(4)?,
                multiplier_pct: r.get(5)?,
                amount: r.get(6)?,
                ref_id: r.get(7)?,
                note: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn xp_total(&self) -> Result<u64> {
        let v: i64 =
            self.conn
                .query_row("SELECT COALESCE(SUM(amount), 0) FROM xp_event", [], |r| {
                    r.get(0)
                })?;
        Ok(v.max(0) as u64)
    }

    pub fn has_xp_event(&self, date: NaiveDate, reason: XpReason, note: &str) -> Result<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM xp_event WHERE date = ?1 AND reason = ?2 AND note = ?3",
            params![date_s(date), reason.as_str(), note],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    // ---- backup ----
    pub fn export_backup(&self) -> Result<Backup> {
        Ok(Backup {
            app: "forja".into(),
            schema_version: self.schema_version()?,
            profile: self.load_profile()?,
            contract: self.load_contract()?,
            sessions: self.sessions()?,
            sets: self.all_sets()?,
            habit_days: self.habit_days()?,
            protein: self.protein_entries()?,
            weights: self.weights()?,
            strength_tests: self.strength_tests()?,
            back_checks: self.back_checks()?,
            xp_events: self.xp_events()?,
            meals: self.meals()?,
        })
    }

    // ---- meals ----
    pub fn add_meal(&self, m: &Meal) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO meal (date, at, description, kcal, protein_g, carbs_g, fat_g, source, confidence,
             photo_ref, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                date_s(m.date),
                instant_s(m.at),
                m.description,
                m.kcal,
                m.protein_g,
                m.carbs_g,
                m.fat_g,
                m.source,
                m.confidence,
                m.photo_ref,
                m.detail
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn meals(&self) -> Result<Vec<Meal>> {
        let mut st = self.conn.prepare(
            "SELECT id, date, at, description, kcal, protein_g, carbs_g, fat_g, source, confidence, photo_ref,
             detail FROM meal ORDER BY at, id",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Meal {
                id: r.get(0)?,
                date: parse_date(&r.get::<_, String>(1)?)?,
                at: parse_instant(&r.get::<_, String>(2)?)?,
                description: r.get(3)?,
                kcal: r.get(4)?,
                protein_g: r.get(5)?,
                carbs_g: r.get(6)?,
                fat_g: r.get(7)?,
                source: r.get(8)?,
                confidence: r.get(9)?,
                photo_ref: r.get(10)?,
                detail: r.get(11)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn meals_on(&self, date: NaiveDate) -> Result<Vec<Meal>> {
        Ok(self
            .meals()?
            .into_iter()
            .filter(|m| m.date == date)
            .collect())
    }

    /// Restore a backup into an empty store (session ids are remapped).
    pub fn import_backup(&mut self, b: &Backup) -> Result<()> {
        if b.app != "forja" {
            return Err(Error::Invalid("no es una copia de Forja".into()));
        }
        if b.schema_version > SCHEMA_VERSION {
            return Err(Error::SchemaTooNew {
                found: b.schema_version,
                supported: SCHEMA_VERSION,
            });
        }
        let has_data: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM session_log", [], |r| r.get(0))?;
        if has_data > 0 || self.xp_total()? > 0 {
            return Err(Error::Invalid(
                "la base no está vacía: importa en un perfil nuevo".into(),
            ));
        }
        let tx = self.conn.transaction()?;
        {
            let s = StoreTx { conn: &tx };
            if let Some(p) = &b.profile {
                s.exec(
                    "INSERT INTO profile (id, data) VALUES (1, ?1)",
                    params![serde_json::to_string(p)?],
                )?;
            }
            if let Some(c) = &b.contract {
                s.exec(
                    "INSERT INTO contract (id, why, commitment, penalty, partner, signed_on) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
                    params![c.why, c.commitment, c.penalty, c.partner, c.signed_on.map(date_s)],
                )?;
            }
            let mut id_map = std::collections::HashMap::new();
            for x in &b.sessions {
                s.exec(
                    "INSERT INTO session_log (date, kind, session_type, recovered_from, started_at, ended_at, pain,
                     effort, note, source, warmup_done, stretches_done)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        date_s(x.date),
                        x.kind.as_str(),
                        x.session_type.as_str(),
                        x.recovered_from.map(date_s),
                        instant_s(x.started_at),
                        x.ended_at.map(instant_s),
                        x.pain,
                        x.effort,
                        x.note,
                        x.source.as_str(),
                        x.warmup_done,
                        x.stretches_done
                    ],
                )?;
                id_map.insert(x.id, tx.last_insert_rowid());
            }
            for x in &b.sets {
                let sid = *id_map
                    .get(&x.session_id)
                    .ok_or_else(|| Error::Invalid("serie sin sesión".into()))?;
                s.exec(
                    "INSERT INTO set_log (session_id, exercise_id, set_index, reps, load_g, ok, watch_verified)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![sid, x.exercise_id, x.index, x.reps, x.load.0, x.ok, x.watch_verified],
                )?;
            }
            for h in &b.habit_days {
                let d = h
                    .date
                    .ok_or_else(|| Error::Invalid("hábito sin fecha".into()))?;
                s.exec(
                    "INSERT INTO habit_day (date, steps, water, vegetables, no_sugar_alcohol, mcgill_big3, sleep_7h)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![date_s(d), h.steps, h.water, h.vegetables, h.no_sugar_alcohol, h.mcgill_big3, h.sleep_7h],
                )?;
            }
            for p in &b.protein {
                s.exec(
                    "INSERT INTO protein_entry (date, at, grams, label) VALUES (?1, ?2, ?3, ?4)",
                    params![date_s(p.date), instant_s(p.at), p.grams, p.label],
                )?;
            }
            for w in &b.weights {
                s.exec(
                    "INSERT INTO weight (date, grams, at) VALUES (?1, ?2, ?3)",
                    params![date_s(w.date), w.weight.0, instant_s(w.at)],
                )?;
            }
            for t in &b.strength_tests {
                let d = t
                    .date
                    .ok_or_else(|| Error::Invalid("prueba sin fecha".into()))?;
                s.exec(
                    "INSERT INTO strength_test (date, pushups_max, pushup_level, box_squats_60s, side_plank_worst_s,
                     waist_mm, weight_g) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        date_s(d),
                        t.pushups_max,
                        t.pushup_level,
                        t.box_squats_60s,
                        t.side_plank_worst_s,
                        t.waist_mm,
                        t.weight.map(|g| g.0)
                    ],
                )?;
            }
            for c in &b.back_checks {
                let d = c
                    .date
                    .ok_or_else(|| Error::Invalid("registro sin fecha".into()))?;
                s.exec(
                    "INSERT INTO back_check (date, radiating_leg_pain, tingling, weakness) VALUES (?1, ?2, ?3, ?4)",
                    params![date_s(d), c.radiating_leg_pain, c.tingling, c.weakness],
                )?;
            }
            for m in &b.meals {
                s.exec(
                    "INSERT INTO meal (date, at, description, kcal, protein_g, carbs_g, fat_g, source, confidence,
                     photo_ref, detail) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        date_s(m.date),
                        instant_s(m.at),
                        m.description,
                        m.kcal,
                        m.protein_g,
                        m.carbs_g,
                        m.fat_g,
                        m.source,
                        m.confidence,
                        m.photo_ref,
                        m.detail
                    ],
                )?;
            }
            for e in &b.xp_events {
                let ref_id = e.ref_id.map(|r| id_map.get(&r).copied().unwrap_or(r));
                s.exec(
                    "INSERT INTO xp_event (at, date, reason, base, multiplier_pct, amount, ref_id, note)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        instant_s(e.at),
                        date_s(e.date),
                        e.reason.as_str(),
                        e.base,
                        e.multiplier_pct,
                        e.amount,
                        ref_id,
                        e.note
                    ],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

struct StoreTx<'a> {
    conn: &'a rusqlite::Transaction<'a>,
}

impl StoreTx<'_> {
    fn exec(&self, sql: &str, p: impl rusqlite::Params) -> Result<()> {
        self.conn.execute(sql, p)?;
        Ok(())
    }
}

pub fn schema_version(conn: &Connection) -> Result<u32> {
    Ok(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0))?)
}

/// Apply migrations up to `target` (each in its own transaction).
pub fn migrate_to(conn: &mut Connection, target: u32) -> Result<()> {
    let current = schema_version(conn)?;
    if current > SCHEMA_VERSION {
        return Err(Error::SchemaTooNew {
            found: current,
            supported: SCHEMA_VERSION,
        });
    }
    for v in current..target.min(SCHEMA_VERSION) {
        let tx = conn.transaction()?;
        tx.execute_batch(MIGRATIONS[v as usize])?;
        tx.pragma_update(None, "user_version", v + 1)?;
        tx.commit()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::resolve_local;
    use chrono::NaiveTime;

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 23).unwrap()
    }

    fn sample_session() -> SessionLog {
        let start = resolve_local(day(), NaiveTime::from_hms_opt(7, 0, 0).unwrap());
        SessionLog {
            id: 0,
            date: day(),
            kind: SessionKind::UpperA,
            session_type: SessionType::Full,
            recovered_from: None,
            started_at: start,
            ended_at: Some(start + chrono::Duration::minutes(40)),
            pain: Some(2),
            effort: Some(7),
            note: "bien".into(),
            source: Source::Simulation,
            warmup_done: true,
            stretches_done: false,
        }
    }

    #[test]
    fn fresh_store_is_empty_and_current() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(s.load_profile().unwrap().is_none());
        assert!(s.sessions().unwrap().is_empty());
        assert_eq!(s.xp_total().unwrap(), 0);
    }

    #[test]
    fn migration_v1_to_v2_keeps_data() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate_to(&mut conn, 1).unwrap();
        assert_eq!(schema_version(&conn).unwrap(), 1);
        conn.execute(
            "INSERT INTO weight (date, grams, at) VALUES ('2026-11-29', 103500, '2026-11-29T10:00:00Z')",
            [],
        )
        .unwrap();
        let has_quest: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'quest'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(has_quest, 0);
        let store = Store::from_connection(conn).unwrap();
        assert_eq!(store.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(store.meals().unwrap().is_empty());
        assert_eq!(store.weights().unwrap()[0].weight, Grams(103_500));
        store
            .conn
            .execute(
                "INSERT INTO achievement (id, unlocked_at) VALUES ('first_session', 'x')",
                [],
            )
            .unwrap();
    }

    #[test]
    fn newer_schema_is_refused() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", 99).unwrap();
        assert!(matches!(
            Store::from_connection(conn),
            Err(Error::SchemaTooNew { found: 99, .. })
        ));
    }

    #[test]
    fn xp_ledger_is_append_only() {
        let s = Store::open_in_memory().unwrap();
        let e = XpEvent::new(
            resolve_local(day(), NaiveTime::from_hms_opt(8, 0, 0).unwrap()),
            day(),
            XpReason::Warmup,
            15,
            100,
        );
        s.add_xp_event(&e).unwrap();
        assert!(s
            .conn
            .execute("UPDATE xp_event SET amount = 9999", [])
            .is_err());
        assert!(s.conn.execute("DELETE FROM xp_event", []).is_err());
        assert_eq!(s.xp_total().unwrap(), 15);
    }

    #[test]
    fn roundtrip_and_backup() {
        let s = Store::open_in_memory().unwrap();
        let p = Profile::initial(day());
        s.save_profile(&p).unwrap();
        let id = s.insert_session(&sample_session()).unwrap();
        s.upsert_set(&SetLog {
            session_id: id,
            exercise_id: "db_bench_press".into(),
            index: 0,
            reps: 12,
            load: Grams(6750),
            ok: true,
            watch_verified: false,
        })
        .unwrap();
        s.upsert_habit_day(
            day(),
            &HabitDay {
                water: true,
                steps: Some(7000),
                ..Default::default()
            },
        )
        .unwrap();
        s.add_protein(&ProteinEntry {
            date: day(),
            at: sample_session().started_at,
            grams: 31,
            label: "Pechuga".into(),
        })
        .unwrap();
        s.add_protein(&ProteinEntry {
            date: day(),
            at: sample_session().started_at,
            grams: 6,
            label: "Huevo".into(),
        })
        .unwrap();
        assert_eq!(s.protein_for(day()).unwrap(), 37);
        let got = s.session(id).unwrap().unwrap();
        assert_eq!(got.started_at, sample_session().started_at);
        assert_eq!(got.kind, SessionKind::UpperA);
        assert_eq!(s.load_profile().unwrap().unwrap(), p);
        let b = s.export_backup().unwrap();
        let json = serde_json::to_string(&b).unwrap();
        let back: Backup = serde_json::from_str(&json).unwrap();
        let mut fresh = Store::open_in_memory().unwrap();
        fresh.import_backup(&back).unwrap();
        let b2 = fresh.export_backup().unwrap();
        assert_eq!(b2.sessions.len(), 1);
        assert_eq!(b2.sets.len(), 1);
        assert_eq!(b2.profile, b.profile);
        assert_eq!(b2.protein.len(), 2);
        // Importing twice is refused.
        assert!(fresh.import_backup(&back).is_err());
    }
}
