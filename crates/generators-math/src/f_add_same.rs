//! F-ADD-SAME: додавання дробів з однаковими знаменниками (сума менша за 1).

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct FAddSame;

const DENOMINATORS: [i64; 13] = [5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 18, 20];

impl SkillGenerator for FAddSame {
    fn skill_id(&self) -> &'static str {
        "F-ADD-SAME"
    }

    fn generate(&self, variant: u64) -> Task {
        build("F-ADD-SAME", variant, |rng| {
            let d = DENOMINATORS[rng.range(0, DENOMINATORS.len() as i64 - 1) as usize];
            let n1 = rng.range(1, d - 2);
            let n2 = rng.range(1, d - 1 - n1);
            let a = Frac::new(n1, d)?;
            let b = Frac::new(n2, d)?;
            let task = Task {
                skill: "F-ADD-SAME",
                variant,
                statement: format!("Обчисли: {n1}/{d} + {n2}/{d}"),
                operands: vec![a, b],
                operand_labels: vec![format!("{n1}/{d}"), format!("{n2}/{d}")],
                params: vec![n1, n2, d],
                answer: a.checked_add(b)?,
                steps: vec![Frac::int(n1 + n2)],
                bugs: vec![BugAnswer {
                    id: "add_denominators",
                    value: Frac::new(n1 + n2, 2 * d)?,
                }],
            };
            Some((task, Frac::new(n1 + n2, d)?))
        })
    }

    fn figures(&self, task: &Task) -> Vec<tutor_core::figure::Figure> {
        crate::figures::for_task(task)
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Нам треба додати дроби {} і {}. Що означає знаменник у кожного з них?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Знаменники однакові, тому частини однакового розміру. Що тоді треба зробити з чисельниками?".to_string(),
            ScriptedLevel::R3 => "Додай чисельники, а знаменник залиш без змін.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![format!(
                "Додаємо чисельники {} і {}; знаменник {} не змінюється.",
                c.params[0], c.params[1], c.params[2]
            )],
            final_prompt: "Тепер ти: допиши відповідь у цьому прикладі.".to_string(),
        })
    }
}
