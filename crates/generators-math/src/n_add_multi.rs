//! N-ADD-MULTI: додавання двох трицифрових чисел стовпчиком.

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct NAddMulti;

/// Незалежний оракул: додавання стовпчиком із перенесенням.
fn column_add(a: i64, b: i64) -> i64 {
    let (mut x, mut y, mut carry, mut place, mut res) = (a, b, 0, 1, 0);
    while x > 0 || y > 0 || carry > 0 {
        let s = x % 10 + y % 10 + carry;
        res += (s % 10) * place;
        carry = s / 10;
        place *= 10;
        x /= 10;
        y /= 10;
    }
    res
}

impl SkillGenerator for NAddMulti {
    fn skill_id(&self) -> &'static str {
        "N-ADD-MULTI"
    }

    fn generate(&self, variant: u64) -> Task {
        build("N-ADD-MULTI", variant, |rng| {
            let a = rng.range(120, 899);
            let b = rng.range(120, 899);
            let units = a % 10 + b % 10;
            // Навичка про перенесення: у розряді одиниць має бути перехід через 10.
            if units < 10 {
                return None;
            }
            let task = Task {
                skill: "N-ADD-MULTI",
                variant,
                statement: format!("Обчисли: {a} + {b}"),
                operands: vec![Frac::int(a), Frac::int(b)],
                operand_labels: vec![a.to_string(), b.to_string()],
                params: vec![a, b],
                answer: Frac::int(a + b),
                steps: vec![Frac::int(units)],
                // Типова помилка: забули додати перенесення з розряду одиниць.
                bugs: vec![BugAnswer {
                    id: "forget_carry",
                    value: Frac::int(a + b - 10),
                }],
            };
            Some((task, Frac::int(column_add(a, b))))
        })
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Ми додаємо {} і {}. Як ти додаєш числа стовпчиком? З якого розряду починаєш?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Додавай розряд за розрядом, починаючи з одиниць. Пам'ятай, що буває перенесення в наступний розряд.".to_string(),
            ScriptedLevel::R3 => "Спочатку додай цифри одиниць. Якщо вийшло двоцифрове число, одну цифру запиши, а іншу додай до розряду десятків.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![
                format!(
                    "Розв'язуємо {} + {}.",
                    c.operand_labels[0], c.operand_labels[1]
                ),
                "Додаємо одиниці, пишемо останню цифру, а перенесення додаємо до десятків."
                    .to_string(),
            ],
            final_prompt: "Тепер ти: доведи цей приклад до кінця й запиши суму.".to_string(),
        })
    }
}
