//! N-MUL-MULTI: множення багатоцифрового числа на двоцифрове стовпчиком.

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct NMulMulti;

impl SkillGenerator for NMulMulti {
    fn skill_id(&self) -> &'static str {
        "N-MUL-MULTI"
    }

    fn generate(&self, variant: u64) -> Task {
        build("N-MUL-MULTI", variant, |rng| {
            // Двоцифрове × двоцифрове або трицифрове × двоцифрове.
            let a = if rng.range(0, 1) == 0 {
                rng.range(12, 99)
            } else {
                rng.range(101, 499)
            };
            let b = rng.range(12, 99);
            let (bu, bt) = (b % 10, b / 10);
            // Обидві цифри множника ненульові: є два справжні часткові добутки.
            if bu == 0 {
                return None;
            }
            let first = a * bu;
            let second = a * bt * 10;
            let task = Task {
                skill: "N-MUL-MULTI",
                variant,
                statement: format!("Обчисли: {a} × {b}"),
                operands: vec![Frac::int(a), Frac::int(b)],
                operand_labels: vec![a.to_string(), b.to_string()],
                params: vec![a, b],
                answer: Frac::int(a * b),
                steps: vec![Frac::int(first), Frac::int(second)],
                // Помилка: другий частковий добуток не зсунуто на розряд.
                bugs: vec![BugAnswer {
                    id: "no_shift",
                    value: Frac::int(first + a * bt),
                }],
            };
            // Оракул: сума часткових добутків за розрядами множника.
            let oracle: i64 = b
                .to_string()
                .chars()
                .rev()
                .enumerate()
                .map(|(i, ch)| a * i64::from(ch.to_digit(10).unwrap_or(0)) * 10_i64.pow(i as u32))
                .sum();
            Some((task, Frac::int(oracle)))
        })
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Ми множимо {} на {}. Як ти множиш на двоцифрове число стовпчиком?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Помнож перше число спочатку на цифру одиниць множника, потім на цифру десятків. Отримаєш два часткові добутки.".to_string(),
            ScriptedLevel::R3 => "Другий частковий добуток рахуємо на десятки, тому його записуємо на один розряд лівіше: у розряді одиниць цього рядка нічого не пишемо (там нуль). Потім додай обидва часткові добутки.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![
                format!(
                    "Множимо {} окремо на цифру одиниць і на цифру десятків числа {}.",
                    c.operand_labels[0], c.operand_labels[1]
                ),
                "Другий добуток записуємо зі зсувом на один розряд ліворуч.".to_string(),
            ],
            final_prompt: "Тепер ти: додай часткові добутки й запиши відповідь.".to_string(),
        })
    }
}
