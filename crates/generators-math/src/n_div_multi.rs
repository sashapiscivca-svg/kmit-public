//! N-DIV-MULTI: ділення націло на однозначне число; у частці є нуль усередині.

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct NDivMulti;

/// Ділення куточком у цифрах; повертає частку (оракул, незалежний від множення в генераторі).
pub fn long_division(dividend: i64, divisor: i64) -> i64 {
    let (mut rem, mut quotient) = (0, 0);
    for ch in dividend.to_string().chars() {
        rem = rem * 10 + i64::from(ch.to_digit(10).unwrap_or(0));
        quotient = quotient * 10 + rem / divisor;
        rem %= divisor;
    }
    quotient
}

/// Частка без нулів (пропущений нуль усередині).
pub fn without_zeros(q: i64) -> i64 {
    q.to_string()
        .chars()
        .filter(|c| *c != '0')
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

impl SkillGenerator for NDivMulti {
    fn skill_id(&self) -> &'static str {
        "N-DIV-MULTI"
    }

    fn generate(&self, variant: u64) -> Task {
        build("N-DIV-MULTI", variant, |rng| {
            let d = rng.range(2, 9);
            // Частка: 3 або 4 цифри, остання цифра ненульова, нуль на місці десятків чи сотень.
            let four = rng.range(0, 1) == 1;
            let lead = rng.range(1, 9);
            let unit = rng.range(1, 9);
            let (q, lead_part) = if four {
                let (h, t) = match rng.range(0, 2) {
                    0 => (0, rng.range(1, 9)),
                    1 => (rng.range(1, 9), 0),
                    _ => (0, 0),
                };
                (lead * 1000 + h * 100 + t * 10 + unit, lead * 1000)
            } else {
                (lead * 100 + unit, lead * 100)
            };
            let dividend = d * q;
            let task = Task {
                skill: "N-DIV-MULTI",
                variant,
                statement: format!("Обчисли: {dividend} : {d}"),
                operands: vec![Frac::int(dividend), Frac::int(d)],
                operand_labels: vec![dividend.to_string(), d.to_string()],
                params: vec![dividend, d],
                answer: Frac::int(q),
                // Залишок після першого кроку ділення.
                steps: vec![Frac::int(dividend - d * lead_part)],
                bugs: vec![BugAnswer {
                    id: "skip_zero",
                    value: Frac::int(without_zeros(q)),
                }],
            };
            Some((task, Frac::int(long_division(dividend, d))))
        })
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Ми ділимо {} на {}. Як ти ділиш багатоцифрове число куточком? З якої цифри починаєш?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Ділимо по одній цифрі, зносячи наступну. Стеж за кожним розрядом частки: у ньому щось має бути записано.".to_string(),
            ScriptedLevel::R3 => "Якщо після зносу цифри (не на початку ділення) число менше за дільник, у частці в цьому розряді пишемо нуль і зносимо наступну цифру. Нуль у частці не можна пропускати.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![
                format!(
                    "Ділимо {} на {} по одній цифрі.",
                    c.operand_labels[0], c.operand_labels[1]
                ),
                "Коли знесене число менше за дільник, у частці пишемо нуль.".to_string(),
            ],
            final_prompt: "Тепер ти: доведи ділення до кінця й запиши частку.".to_string(),
        })
    }
}
