//! Пошук числових токенів у тексті (цілі, десяткові з комою чи крапкою, дроби `a/b`, також із пробілами навколо `/`).
//! Цей сканер не вужчий за `parse_answer`: цифри інших письмових систем зводяться до ASCII,
//! зайві нулі на початку та в кінці не ховають значення.
//! Числа словами («п'ять») тут НЕ розпізнаються: скриптовані тексти їх не містять,
//! а для вільної генерації моделлю (віха M4) знадобиться окремий нормалізатор.

use crate::frac::Frac;

fn digit_value(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        '\u{0660}'..='\u{0669}' => Some(c as u32 - 0x0660), // арабсько-індійські
        '\u{06F0}'..='\u{06F9}' => Some(c as u32 - 0x06F0), // розширені арабсько-індійські
        '\u{FF10}'..='\u{FF19}' => Some(c as u32 - 0xFF10), // повноширинні
        _ => None,
    }
}

fn is_space(c: char) -> bool {
    c == ' ' || c == '\u{a0}' || c == '\t'
}

/// Зчитує підряд цифри з позиції `i`; повертає ASCII-рядок і позицію після останньої цифри.
fn read_digits(cs: &[char], mut i: usize) -> (String, usize) {
    let mut s = String::new();
    while i < cs.len() {
        match digit_value(cs[i]) {
            Some(d) => s.push(char::from_digit(d, 10).unwrap_or('0')),
            None => break,
        }
        i += 1;
    }
    (s, i)
}

/// Ціле з рядка цифр без зайвих нулів на початку. `None`, якщо не вміщається.
fn parse_u(s: &str) -> Option<i64> {
    let t = s.trim_start_matches('0');
    if t.is_empty() {
        return Some(0);
    }
    if t.len() > 18 {
        return None;
    }
    t.parse().ok()
}

fn decimal(int_str: &str, frac_str: &str) -> Option<Frac> {
    let frac_trimmed = frac_str.trim_end_matches('0');
    if frac_trimmed.is_empty() {
        return Some(Frac::int(parse_u(int_str)?));
    }
    if frac_trimmed.len() > 9 {
        return None;
    }
    let scale = 10_i64.pow(u32::try_from(frac_trimmed.len()).ok()?);
    let total = parse_u(int_str)?
        .checked_mul(scale)?
        .checked_add(parse_u(frac_trimmed)?)?;
    Frac::new(total, scale)
}

pub fn numeric_tokens(text: &str) -> Vec<Frac> {
    let cs: Vec<char> = text.chars().collect();
    let n = cs.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i < n {
        if digit_value(cs[i]).is_none() {
            i += 1;
            continue;
        }
        let (int_str, after_int) = read_digits(&cs, i);
        i = after_int;

        // Дріб `a/b`, можливо з пробілами навколо косої риски.
        let mut k = i;
        while k < n && is_space(cs[k]) {
            k += 1;
        }
        if k < n && cs[k] == '/' {
            let mut m = k + 1;
            while m < n && is_space(cs[m]) {
                m += 1;
            }
            if m < n && digit_value(cs[m]).is_some() {
                let (den_str, end) = read_digits(&cs, m);
                if let (Some(a), Some(b)) = (parse_u(&int_str), parse_u(&den_str)) {
                    if let Some(f) = Frac::new(a, b) {
                        out.push(f);
                    }
                }
                i = end;
                continue;
            }
        }

        // Десятковий запис із комою чи крапкою.
        if i + 1 < n && (cs[i] == ',' || cs[i] == '.') && digit_value(cs[i + 1]).is_some() {
            let (frac_str, end) = read_digits(&cs, i + 1);
            if let Some(f) = decimal(&int_str, &frac_str) {
                out.push(f);
            }
            i = end;
            continue;
        }

        if let Some(v) = parse_u(&int_str) {
            out.push(Frac::int(v));
        }
    }
    out
}
