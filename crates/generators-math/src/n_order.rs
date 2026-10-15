//! N-ORDER: порядок дій у виразах a + b × c та a - b × c (множення виконується першим).

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct NOrder;

impl SkillGenerator for NOrder {
    fn skill_id(&self) -> &'static str {
        "N-ORDER"
    }

    fn generate(&self, variant: u64) -> Task {
        build("N-ORDER", variant, |rng| {
            let plus = rng.range(0, 1) == 0;
            let b = rng.range(3, 19);
            let c = rng.range(3, 9);
            let product = b * c;
            let a = if plus {
                rng.range(5, 99)
            } else {
                // Різниця додатна й проміжні результати невід'ємні в обох порядках дій.
                rng.range(product + 1, product + 60)
            };
            let (sign, answer, left_to_right) = if plus {
                ("+", a + product, (a + b) * c)
            } else {
                ("-", a - product, (a - b) * c)
            };
            let task = Task {
                skill: "N-ORDER",
                variant,
                statement: format!("Обчисли: {a} {sign} {b} × {c}"),
                operands: vec![Frac::int(a), Frac::int(b), Frac::int(c)],
                operand_labels: vec![a.to_string(), b.to_string(), c.to_string()],
                params: vec![a, b, c, i64::from(plus)],
                answer: Frac::int(answer),
                steps: vec![Frac::int(product)],
                // Помилка: дії виконано зліва направо.
                bugs: vec![BugAnswer {
                    id: "left_to_right",
                    value: Frac::int(left_to_right),
                }],
            };
            // Оракул: окремо рахуємо добуток, потім додаємо чи віднімаємо.
            let prod = (0..c).map(|_| b).sum::<i64>();
            let oracle = if plus { a + prod } else { a - prod };
            Some((task, Frac::int(oracle)))
        })
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        let (a, b, c) = (
            &task.operand_labels[0],
            &task.operand_labels[1],
            &task.operand_labels[2],
        );
        match level {
            ScriptedLevel::R1 => format!(
                "У виразі є числа {a}, {b} і {c} та дві дії. Яка дія в цьому виразі виконується першою за правилами порядку дій?"
            ),
            ScriptedLevel::R2 => "Множення виконується раніше за додавання й віднімання. Підкресли в записі ту частину, яку треба порахувати першою.".to_string(),
            ScriptedLevel::R3 => "Спочатку порахуй добуток, потім виконай додавання чи віднімання з тим, що вийшло.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![
                format!(
                    "У виразі з числами {}, {} і {} спочатку виконуємо множення.",
                    c.operand_labels[0], c.operand_labels[1], c.operand_labels[2]
                ),
                "Потім виконуємо додавання або віднімання.".to_string(),
            ],
            final_prompt: "Тепер ти: порахуй вираз до кінця й запиши відповідь.".to_string(),
        })
    }
}
