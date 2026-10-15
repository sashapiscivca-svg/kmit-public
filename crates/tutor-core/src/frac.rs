//! Точний дріб на цілих числах. Жодних f64: результати однакові на всіх платформах.

use std::cmp::Ordering;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Frac {
    num: i64,
    den: i64,
}

fn gcd(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

impl Frac {
    /// Нормалізований дріб (скорочений, знаменник додатний). `None`, якщо знаменник 0 або переповнення.
    pub fn new(num: i64, den: i64) -> Option<Frac> {
        Self::from_i128(i128::from(num), i128::from(den))
    }

    fn from_i128(num: i128, den: i128) -> Option<Frac> {
        if den == 0 {
            return None;
        }
        let g = gcd(num, den);
        let (mut n, mut d) = (num / g, den / g);
        if d < 0 {
            n = -n;
            d = -d;
        }
        Some(Frac {
            num: i64::try_from(n).ok()?,
            den: i64::try_from(d).ok()?,
        })
    }

    pub fn int(n: i64) -> Frac {
        Frac { num: n, den: 1 }
    }

    pub fn num(&self) -> i64 {
        self.num
    }

    pub fn den(&self) -> i64 {
        self.den
    }

    pub fn is_integer(&self) -> bool {
        self.den == 1
    }

    pub fn checked_add(self, o: Frac) -> Option<Frac> {
        Self::from_i128(
            i128::from(self.num) * i128::from(o.den) + i128::from(o.num) * i128::from(self.den),
            i128::from(self.den) * i128::from(o.den),
        )
    }

    pub fn checked_sub(self, o: Frac) -> Option<Frac> {
        Self::from_i128(
            i128::from(self.num) * i128::from(o.den) - i128::from(o.num) * i128::from(self.den),
            i128::from(self.den) * i128::from(o.den),
        )
    }

    pub fn checked_mul(self, o: Frac) -> Option<Frac> {
        Self::from_i128(
            i128::from(self.num) * i128::from(o.num),
            i128::from(self.den) * i128::from(o.den),
        )
    }

    pub fn checked_div(self, o: Frac) -> Option<Frac> {
        if o.num == 0 {
            return None;
        }
        Self::from_i128(
            i128::from(self.num) * i128::from(o.den),
            i128::from(self.den) * i128::from(o.num),
        )
    }
}

impl PartialOrd for Frac {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Frac {
    fn cmp(&self, other: &Self) -> Ordering {
        (i128::from(self.num) * i128::from(other.den))
            .cmp(&(i128::from(other.num) * i128::from(self.den)))
    }
}

impl fmt::Display for Frac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            write!(f, "{}", self.num)
        } else {
            write!(f, "{}/{}", self.num, self.den)
        }
    }
}

fn digits(s: &str) -> Option<&str> {
    if !s.is_empty() && s.len() <= 15 && s.bytes().all(|b| b.is_ascii_digit()) {
        Some(s)
    } else {
        None
    }
}

fn parse_int(s: &str) -> Option<i64> {
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let v: i64 = digits(rest)?.parse().ok()?;
    Some(if neg { -v } else { v })
}

/// Розбір відповіді дитини: `12`, `3/4`, `3,5`, `3.5`, `-2`. Інше дає `None`.
pub fn parse_answer(input: &str) -> Option<Frac> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }
    if let Some((a, b)) = s.split_once('/') {
        return Frac::new(parse_int(a.trim())?, parse_int(b.trim())?);
    }
    if let Some((int_part, frac_part)) = s.split_once([',', '.']) {
        let (neg, int_digits) = match int_part.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, int_part),
        };
        let int_digits = digits(int_digits)?;
        let frac_digits = digits(frac_part)?;
        if frac_digits.len() > 9 {
            return None;
        }
        let scale = 10_i64.pow(u32::try_from(frac_digits.len()).ok()?);
        let int_v: i64 = int_digits.parse().ok()?;
        let frac_v: i64 = frac_digits.parse().ok()?;
        let total = int_v.checked_mul(scale)?.checked_add(frac_v)?;
        return Frac::new(if neg { -total } else { total }, scale);
    }
    Some(Frac::int(parse_int(s)?))
}
