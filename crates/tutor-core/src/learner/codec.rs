//! Текстовий кодек стану учня для сховища. Версійований, детермінований (BTree-порядок), точний для `f64`
//! (через біти). Декодер захищений: будь-який зіпсований текст дає помилку, а не паніку.

use super::LearnerState;
use crate::bkt::SkillState;
use crate::review::ReviewSchedule;
use std::collections::BTreeSet;

/// Найбільший дозволений розмір тексту стану.
pub const MAX_TEXT_BYTES: usize = 8 * 1024 * 1024;
const HEADER: &str = "KMT-LEARNER 1";

#[derive(Debug, PartialEq, Eq)]
pub enum CodecError {
    TooLarge,
    /// Немає заголовка або невідома версія.
    Header,
    /// Некоректний рядок (номер рядка з одиниці).
    Line(usize),
}

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '%' => out.push_str("%25"),
            ' ' => out.push_str("%20"),
            '\n' => out.push_str("%0A"),
            '\r' => out.push_str("%0D"),
            '\t' => out.push_str("%09"),
            _ => out.push(c),
        }
    }
    out
}

fn unesc(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hi = b.get(i + 1).copied().and_then(hex_val)?;
            let lo = b.get(i + 2).copied().and_then(hex_val)?;
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_val(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

impl LearnerState {
    /// Стан у вигляді тексту для збереження.
    pub fn encode(&self) -> String {
        let mut out = String::new();
        out.push_str(HEADER);
        out.push('\n');
        out.push_str(&format!("today {}\n", self.today));
        out.push_str(&format!("reviews_today {}\n", self.reviews_today));
        for (id, st) in &self.skills {
            let days = if st.clean_days().is_empty() {
                "-".to_string()
            } else {
                st.clean_days()
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            };
            out.push_str(&format!(
                "skill {} {:016x} {} {} {}\n",
                esc(id),
                st.p_known.to_bits(),
                st.streak,
                st.attempts,
                days
            ));
        }
        for id in &self.mastered {
            out.push_str(&format!("mastered {}\n", esc(id)));
        }
        for (id, sch) in &self.schedules {
            out.push_str(&format!(
                "schedule {} {} {}\n",
                esc(id),
                sch.level(),
                sch.due_day()
            ));
        }
        for (skill, order) in &self.seen_order {
            for statement in order {
                out.push_str(&format!("seen {} {}\n", esc(skill), esc(statement)));
            }
        }
        for (skill, n) in &self.cursors {
            out.push_str(&format!("cursor {} {}\n", esc(skill), n));
        }
        for skill in &self.exhausted {
            out.push_str(&format!("exhausted {}\n", esc(skill)));
        }
        for ((skill, bug), n) in &self.bug_counts {
            out.push_str(&format!("bug {} {} {}\n", esc(skill), esc(bug), n));
        }
        for (skill, n) in &self.tasks_today {
            out.push_str(&format!("tasks_today {} {}\n", esc(skill), n));
        }
        for (skill, statement) in &self.ask_adult {
            out.push_str(&format!("ask_adult {} {}\n", esc(skill), esc(statement)));
        }
        for (skill, (done, clean)) in &self.microtests {
            out.push_str(&format!("microtest {} {} {}\n", esc(skill), done, clean));
        }
        out
    }

    /// Відновлює стан із тексту, створеного `encode`. Некоректний ввід дає помилку.
    pub fn decode(text: &str) -> Result<LearnerState, CodecError> {
        if text.len() > MAX_TEXT_BYTES {
            return Err(CodecError::TooLarge);
        }
        let mut lines = text.lines();
        if lines.next() != Some(HEADER) {
            return Err(CodecError::Header);
        }
        let mut l = LearnerState::new();
        for (idx, line) in lines.enumerate() {
            let n = idx + 2;
            if line.is_empty() {
                continue;
            }
            let bad = CodecError::Line(n);
            let parts: Vec<&str> = line.split(' ').collect();
            let num = |s: &str| s.parse::<u32>().map_err(|_| CodecError::Line(n));
            match (parts[0], parts.len()) {
                ("today", 2) => l.today = num(parts[1])?,
                ("reviews_today", 2) => {
                    l.reviews_today =
                        usize::try_from(num(parts[1])?).map_err(|_| CodecError::Line(n))?;
                }
                ("skill", 6) => {
                    let id = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    let bits =
                        u64::from_str_radix(parts[2], 16).map_err(|_| CodecError::Line(n))?;
                    let p = f64::from_bits(bits);
                    if !(p.is_finite() && p > 0.0 && p < 1.0) {
                        return Err(bad);
                    }
                    let mut days = BTreeSet::new();
                    if parts[5] != "-" {
                        for d in parts[5].split(',') {
                            days.insert(num(d)?);
                        }
                    }
                    let st = SkillState::from_parts(p, num(parts[3])?, num(parts[4])?, days);
                    if l.skills.insert(id, st).is_some() {
                        return Err(bad);
                    }
                }
                ("mastered", 2) => {
                    l.mastered
                        .insert(unesc(parts[1]).ok_or(CodecError::Line(n))?);
                }
                ("schedule", 4) => {
                    let id = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    let level = u8::try_from(num(parts[2])?).map_err(|_| CodecError::Line(n))?;
                    let sch = ReviewSchedule::from_parts(level, num(parts[3])?)
                        .ok_or(CodecError::Line(n))?;
                    if l.schedules.insert(id, sch).is_some() {
                        return Err(bad);
                    }
                }
                ("seen", 3) => {
                    let skill = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    let statement = unesc(parts[2]).ok_or(CodecError::Line(n))?;
                    if l.seen
                        .entry(skill.clone())
                        .or_default()
                        .insert(statement.clone())
                    {
                        l.seen_order.entry(skill).or_default().push(statement);
                    }
                }
                ("cursor", 3) => {
                    let skill = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    let v = parts[2].parse::<u64>().map_err(|_| CodecError::Line(n))?;
                    l.cursors.insert(skill, v);
                }
                ("exhausted", 2) => {
                    l.exhausted
                        .insert(unesc(parts[1]).ok_or(CodecError::Line(n))?);
                }
                ("bug", 4) => {
                    let skill = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    let bug = unesc(parts[2]).ok_or(CodecError::Line(n))?;
                    l.bug_counts.insert((skill, bug), num(parts[3])?);
                }
                ("tasks_today", 3) => {
                    let skill = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    l.tasks_today.insert(skill, num(parts[2])?);
                }
                ("ask_adult", 3) => {
                    let skill = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    let statement = unesc(parts[2]).ok_or(CodecError::Line(n))?;
                    let item = (skill, statement);
                    if !l.ask_adult.contains(&item) {
                        l.ask_adult.push(item);
                    }
                }
                ("microtest", 4) => {
                    let skill = unesc(parts[1]).ok_or(CodecError::Line(n))?;
                    let done = u8::try_from(num(parts[2])?).map_err(|_| CodecError::Line(n))?;
                    let clean = u8::try_from(num(parts[3])?).map_err(|_| CodecError::Line(n))?;
                    l.microtests.insert(skill, (done, clean));
                }
                _ => return Err(bad),
            }
        }
        Ok(l)
    }
}
