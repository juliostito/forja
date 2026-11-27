//! Dates in America/Santiago.
//!
//! Chile moves its clocks at midnight: when summer time starts (September) local 00:00–00:59 on Sunday
//! does not exist, and when it ends (April) 23:00–23:59 on Saturday happens twice. So days are always
//! handled as local `NaiveDate`s and wall-clock times go through [`resolve_local`].

use chrono::{
    DateTime, Datelike, Duration, LocalResult, NaiveDate, NaiveTime, TimeZone, Utc, Weekday,
};
use chrono_tz::Tz;

pub const ZONE: Tz = chrono_tz::America::Santiago;

pub fn local_date(instant: DateTime<Utc>) -> NaiveDate {
    instant.with_timezone(&ZONE).date_naive()
}

pub fn local_time(instant: DateTime<Utc>) -> NaiveTime {
    instant.with_timezone(&ZONE).time()
}

/// The instant of a Santiago wall-clock time. Times inside the spring gap move forward to the first
/// valid minute; repeated autumn times resolve to the earlier instant.
pub fn resolve_local(date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    let naive = date.and_time(time);
    match ZONE.from_local_datetime(&naive) {
        LocalResult::Single(t) => t.with_timezone(&Utc),
        LocalResult::Ambiguous(a, _) => a.with_timezone(&Utc),
        LocalResult::None => {
            let mut n = naive;
            for _ in 0..240 {
                n += Duration::minutes(1);
                if let Some(t) = ZONE.from_local_datetime(&n).earliest() {
                    return t.with_timezone(&Utc);
                }
            }
            Utc.from_utc_datetime(&naive)
        }
    }
}

/// First instant of a local day (01:00 on the Chilean spring-forward Sunday).
pub fn start_of_day(date: NaiveDate) -> DateTime<Utc> {
    resolve_local(date, NaiveTime::from_hms_opt(0, 0, 0).expect("valid time"))
}

pub fn week_monday(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(date.weekday().num_days_from_monday()))
}

pub fn days_between(from: NaiveDate, to: NaiveDate) -> i64 {
    (to - from).num_days()
}

pub fn add_days(date: NaiveDate, days: i64) -> NaiveDate {
    date + Duration::days(days)
}

pub fn weekday_es(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "lunes",
        Weekday::Tue => "martes",
        Weekday::Wed => "miércoles",
        Weekday::Thu => "jueves",
        Weekday::Fri => "viernes",
        Weekday::Sat => "sábado",
        Weekday::Sun => "domingo",
    }
}

pub fn month_es(m: u32) -> &'static str {
    const NAMES: [&str; 12] = [
        "enero",
        "febrero",
        "marzo",
        "abril",
        "mayo",
        "junio",
        "julio",
        "agosto",
        "septiembre",
        "octubre",
        "noviembre",
        "diciembre",
    ];
    NAMES[(m.clamp(1, 12) - 1) as usize]
}

/// "viernes 27 de noviembre".
pub fn format_date_es(d: NaiveDate) -> String {
    format!(
        "{} {} de {}",
        weekday_es(d.weekday()),
        d.day(),
        month_es(d.month())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn chile_spring_gap_is_at_midnight_and_handled() {
        // Find the 2026 day whose local midnight does not exist.
        let mut gap_days = Vec::new();
        let mut day = d(2026, 1, 1);
        while day.year() == 2026 {
            let naive = day.and_hms_opt(0, 0, 0).unwrap();
            if matches!(ZONE.from_local_datetime(&naive), LocalResult::None) {
                gap_days.push(day);
            }
            day = day.succ_opt().unwrap();
        }
        assert_eq!(
            gap_days.len(),
            1,
            "Chile has one spring-forward gap per year at midnight"
        );
        let gap = gap_days[0];
        assert_eq!(gap.month(), 9);
        assert_eq!(gap.weekday(), Weekday::Sun);
        // start_of_day still works and is 01:00 local.
        let start = start_of_day(gap);
        assert_eq!(local_date(start), gap);
        assert_eq!(local_time(start), NaiveTime::from_hms_opt(1, 0, 0).unwrap());
        // That day is 23 hours long.
        let next = start_of_day(gap.succ_opt().unwrap());
        assert_eq!((next - start).num_hours(), 23);
    }

    #[test]
    fn training_time_always_exists_and_autumn_repeat_resolves_early() {
        let seven = NaiveTime::from_hms_opt(7, 0, 0).unwrap();
        let mut day = d(2026, 1, 1);
        let mut ambiguous = 0;
        while day.year() == 2026 {
            let t = resolve_local(day, seven);
            assert_eq!(local_date(t), day);
            assert_eq!(local_time(t), seven);
            let late = day.and_hms_opt(23, 30, 0).unwrap();
            if matches!(
                ZONE.from_local_datetime(&late),
                LocalResult::Ambiguous(_, _)
            ) {
                ambiguous += 1;
                let r = resolve_local(day, NaiveTime::from_hms_opt(23, 30, 0).unwrap());
                assert_eq!(local_date(r), day);
            }
            day = day.succ_opt().unwrap();
        }
        assert_eq!(ambiguous, 1, "one repeated hour per year (April)");
    }

    #[test]
    fn spanish_dates() {
        assert_eq!(format_date_es(d(2026, 11, 27)), "viernes 27 de noviembre");
        assert_eq!(week_monday(d(2026, 11, 27)), d(2026, 11, 23));
        assert_eq!(week_monday(d(2026, 11, 23)), d(2026, 11, 23));
    }
}
