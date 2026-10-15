//! Експорт задач з математики 5 класу в JSON Lines для відкритого датасету (див. scripts/export_math_dataset.py).
//!
//! `cargo run -p generators-math --release --example export_tasks -- 200 > tasks.jsonl`
//!
//! Для кожної з 35 навичок генерує варіанти 1..=N (детерміновано). Кожна задача вже пройшла `validate_task` у генераторі
//! (відповідь звірено з оракулом, обчисленим іншим шляхом). Тут додатково: ключ повинен розбиратися назад у ту саму
//! відповідь (`parse_answer`). Повтори умови пропускаються. Малюнки (правило І1: лише дані умови, не відповідь) додаються
//! структурованим полем `figures`; умова задачі повна й без малюнка.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use generators_math::{all_generators, bug_message};
use tutor_core::figure::Figure;
use tutor_core::frac::{parse_answer, Frac};
use tutor_core::generator::{AnswerKind, ScriptedLevel};
use tutor_core::task::Task;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(o, "\\u{:04x}", c as u32);
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn arr(items: &[String]) -> String {
    format!(
        "[{}]",
        items.iter().map(|s| esc(s)).collect::<Vec<_>>().join(",")
    )
}

/// Число так, як його записує учень: десятковий дріб з комою, якщо в умові десяткові дроби (як у ключах робіт вчителя).
fn key(task: &Task, value: Frac) -> String {
    let decimal = task.operand_labels.iter().any(|l| l.contains(','));
    if decimal && value.den() != 1 {
        let mut d = value.den();
        let (mut twos, mut fives) = (0u32, 0u32);
        while d % 2 == 0 {
            d /= 2;
            twos += 1;
        }
        while d % 5 == 0 {
            d /= 5;
            fives += 1;
        }
        if d == 1 {
            let scale = twos.max(fives);
            let den = 10i128.pow(scale);
            let num = i128::from(value.num()) * den / i128::from(value.den());
            let (int, frac) = (num.abs() / den, num.abs() % den);
            let digits = format!("{frac:0>width$}", width = scale as usize);
            let digits = digits.trim_end_matches('0');
            let sign = if num < 0 { "-" } else { "" };
            return if digits.is_empty() {
                format!("{sign}{int}")
            } else {
                format!("{sign}{int},{digits}")
            };
        }
    }
    value.to_string()
}

/// Малюнок як JSON-об'єкт (той самий набір полів, що й у `tutor_core::figure::Figure`).
fn figure_json(f: &Figure) -> String {
    match f {
        Figure::Diagram { kind, title, rows } => format!(
            "{{\"type\":\"diagram\",\"kind\":\"{}\",\"title\":{},\"rows\":[{}]}}",
            kind.code(),
            esc(title),
            rows.iter().map(|r| arr(r)).collect::<Vec<_>>().join(",")
        ),
        Figure::FractionBar { num, den } => {
            format!("{{\"type\":\"fraction_bar\",\"num\":{num},\"den\":{den}}}")
        }
        Figure::DecimalGrid { hundredths } => {
            format!("{{\"type\":\"decimal_grid\",\"hundredths\":{hundredths}}}")
        }
        Figure::Rect { w, h } => format!("{{\"type\":\"rect\",\"w\":{},\"h\":{}}}", esc(w), esc(h)),
        Figure::Angle { deg, label } => {
            format!("{{\"type\":\"angle\",\"deg\":{deg},\"label\":{}}}", esc(label))
        }
        Figure::NumberLine { min, max, parts, marks } => format!(
            "{{\"type\":\"number_line\",\"min\":{min},\"max\":{max},\"parts\":{parts},\"marks\":[{}]}}",
            marks.iter().map(|(p, t)| format!("[{p},{}]", esc(t))).collect::<Vec<_>>().join(",")
        ),
    }
}

fn main() {
    let n: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(200);
    let (mut kept, mut figures, mut dups) = (0usize, 0usize, 0usize);
    for g in all_generators() {
        let mut seen = BTreeSet::new();
        for v in 1..=n {
            let t = g.generate(v);
            let figs = g.figures(&t);
            if !figs.is_empty() {
                figures += 1;
            }
            if !seen.insert(t.statement.trim().to_lowercase()) {
                dups += 1;
                continue;
            }
            let hints: Vec<String> = [ScriptedLevel::R1, ScriptedLevel::R2, ScriptedLevel::R3]
                .into_iter()
                .map(|l| g.hint(&t, l))
                .collect();
            let (kind, answer, choices) = match g.answer_kind() {
                AnswerKind::Number => {
                    let k = key(&t, t.answer);
                    assert_eq!(
                        parse_answer(&k),
                        Some(t.answer),
                        "key does not parse back: {} {} -> {k}",
                        t.skill,
                        v
                    );
                    ("number", k, Vec::new())
                }
                AnswerKind::Choice => (
                    "choice",
                    g.reveal(&t).expect("choice without reveal").answer_text,
                    g.choices(&t),
                ),
                AnswerKind::Text => (
                    "text",
                    g.reveal(&t).expect("text without reveal").answer_text,
                    Vec::new(),
                ),
            };
            let bugs: Vec<String> = t
                .bugs
                .iter()
                .map(|b| {
                    format!(
                        "{{\"id\":{},\"value\":{},\"explanation\":{}}}",
                        esc(b.id),
                        esc(&key(&t, b.value)),
                        bug_message(t.skill, b.id).map_or("null".to_string(), esc)
                    )
                })
                .collect();
            let steps: Vec<String> = t.steps.iter().map(|s| key(&t, *s)).collect();
            println!(
                "{{\"skill\":{},\"variant\":{},\"statement\":{},\"answer_kind\":\"{kind}\",\"answer\":{},\"choices\":{},\"steps\":{},\"hints\":{},\"typical_errors\":[{}],\"figures\":[{}]}}",
                esc(t.skill), v, esc(&t.statement), esc(&answer), arr(&choices), arr(&steps), arr(&hints), bugs.join(","),
                figs.iter().map(figure_json).collect::<Vec<_>>().join(",")
            );
            kept += 1;
        }
    }
    eprintln!("kept {kept} (with a figure: {figures}), skipped duplicate statements {dups}");
}
