//! # forja-core
//!
//! Deterministic domain core for Forja: training plan, double progression with the user's real
//! equipment, streaks and "never miss twice", lower-back safety rules, nutrition, XP/levels/forge heat,
//! weekly review and versioned SQLite storage. No UI, no clock reads, no unsafe code.

#![forbid(unsafe_code)]

pub mod catalog;
pub mod forja;
pub mod habits;
pub mod meal;
pub mod model;
pub mod motion;
pub mod nutrition;
pub mod plan;
pub mod pose;
pub mod profile;
pub mod progression;
pub mod review;
pub mod safety;
pub mod store;
pub mod streak;
pub mod time;
pub mod units;
pub mod xp;

pub use forja::{Alert, Forja, SessionOutcome, TodayView};
pub use store::{Error, Result, Store};
