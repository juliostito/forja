//! Units and Chilean number formatting (decimal comma, dot for thousands).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Mass in grams. Integer arithmetic keeps load progression exact with 1.25 kg plates.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Grams(pub u32);

impl Grams {
    pub const ZERO: Grams = Grams(0);

    pub fn from_kg(kg: f64) -> Self {
        Grams((kg * 1000.0).round().max(0.0) as u32)
    }

    pub fn kg(self) -> f64 {
        f64::from(self.0) / 1000.0
    }
}

impl std::ops::Add for Grams {
    type Output = Grams;
    fn add(self, rhs: Grams) -> Grams {
        Grams(self.0 + rhs.0)
    }
}

impl fmt::Display for Grams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} kg", format_kg_number(*self))
    }
}

/// "7,25" / "9" / "16,5" — decimal comma, trailing zeros trimmed.
pub fn format_kg_number(g: Grams) -> String {
    let whole = g.0 / 1000;
    let frac = g.0 % 1000;
    if frac == 0 {
        return group_thousands(u64::from(whole));
    }
    let mut s = format!("{frac:03}");
    while s.ends_with('0') {
        s.pop();
    }
    format!("{},{}", group_thousands(u64::from(whole)), s)
}

/// 2250 → "2.250".
pub fn group_thousands(n: u64) -> String {
    let s = n.to_string();
    let len = s.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// One decimal with a comma: 3.432 → "3,4".
pub fn format_one_decimal(x: f64) -> String {
    let tenths = (x * 10.0).round() as i64;
    let sign = if tenths < 0 { "-" } else { "" };
    let t = tenths.abs();
    format!("{sign}{},{}", group_thousands((t / 10) as u64), t % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_chilean_numbers() {
        assert_eq!(format_kg_number(Grams(7250)), "7,25");
        assert_eq!(format_kg_number(Grams(9000)), "9");
        assert_eq!(format_kg_number(Grams(16500)), "16,5");
        assert_eq!(Grams(1250).to_string(), "1,25 kg");
        assert_eq!(group_thousands(2250), "2.250");
        assert_eq!(group_thousands(10000), "10.000");
        assert_eq!(group_thousands(999), "999");
        assert_eq!(format_one_decimal(3.432), "3,4");
        assert_eq!(Grams::from_kg(104.0), Grams(104_000));
    }
}
