//! N-SUB-MULTI: віднімання трицифрових чисел стовпчиком із позичанням у розряді одиниць.

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct NSubMulti;

/// Незалежний оракул: віднімання стовпчиком із позичанням (a >= b).
pub fn column_sub(a: i64, b: i64) -> i64 {
    let (mut x, mut y, mut borrow, mut place, mut res) = (a, b, 0, 1, 0);
    while x > 0 || y > 0 {
        let mut d = x % 10 - y % 10 - borrow;
        if d < 0 {
            d += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        res += d * place;
        place *= 10;
        x /= 10;
        y /= 10;
    }
    res
}

/// Помилка «від більшої цифри віднімають меншу» в кожному розряді.
pub fn smaller_from_larger(a: i64, b: i64) -> i64 {
    let (mut x, mut y, mut place, mut res) = (a, b, 1, 0);
    while x > 0 || y > 0 {
        res += (x % 10 - y % 10).abs() * place;
        place *= 10;
        x /= 10;
        y /= 10;
    }
    res
}

impl SkillGenerator for NSubMulti {
    fn skill_id(&self) -> &'static str {
        "N-SUB-MULTI"
    }

    fn generate(&self, variant: u64) -> Task {
        build("N-SUB-MULTI", variant, |rng| {
            let a = rng.range(200, 999);
            let b = rng.range(101, a - 1);
            let (au, bu) = (a % 10, b % 10);
            // Навичка про позичання: цифра одиниць зменшуваного менша за цифру від'ємника.
            if au >= bu {
                return None;
            }
            let task = Task {
                skill: "N-SUB-MULTI",
                variant,
                statement: format!("Обчисли: {a} - {b}"),
                operands: vec![Frac::int(a), Frac::int(b)],
                operand_labels: vec![a.to_string(), b.to_string()],
                params: vec![a, b],
                answer: Frac::int(a - b),
                // Одиниці після позичання десятка.
                steps: vec![Frac::int(au + 10 - bu)],
                bugs: vec![BugAnswer {
                    id: "smaller_from_larger",
                    value: Frac::int(smaller_from_larger(a, b)),
                }],
            };
            Some((task, Frac::int(column_sub(a, b))))
        })
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Ми віднімаємо {} від {}. Як ти віднімаєш числа стовпчиком? З якого розряду починаєш?",
                task.operand_labels[1], task.operand_labels[0]
            ),
            ScriptedLevel::R2 => "Віднімай розряд за розрядом, починаючи з одиниць. Подивись: чи вистачає одиниць у верхньому числі?".to_string(),
            ScriptedLevel::R3 => "Якщо у верхньому числі одиниць менше, ніж треба відняти, позич один десяток: він дає ще десять одиниць, а в розряді десятків стає на один менше.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![
                format!(
                    "Розв'язуємо {} - {}.",
                    c.operand_labels[0], c.operand_labels[1]
                ),
                "Одиниць зверху не вистачає, тому позичаємо десяток у розряді десятків."
                    .to_string(),
            ],
            final_prompt: "Тепер ти: доведи цей приклад до кінця й запиши різницю.".to_string(),
        })
    }
}
