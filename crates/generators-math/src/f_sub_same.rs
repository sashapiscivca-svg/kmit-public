//! F-SUB-SAME: віднімання дробів з однаковими знаменниками (правильні дроби, різниця додатна).

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct FSubSame;

const DENOMINATORS: [i64; 13] = [5, 6, 7, 8, 9, 10, 11, 12, 14, 15, 16, 18, 20];

impl SkillGenerator for FSubSame {
    fn skill_id(&self) -> &'static str {
        "F-SUB-SAME"
    }

    fn generate(&self, variant: u64) -> Task {
        build("F-SUB-SAME", variant, |rng| {
            let d = DENOMINATORS[rng.range(0, DENOMINATORS.len() as i64 - 1) as usize];
            let n2 = rng.range(1, d - 2);
            let n1 = rng.range(n2 + 1, d - 1);
            let a = Frac::new(n1, d)?;
            let b = Frac::new(n2, d)?;
            let task = Task {
                skill: "F-SUB-SAME",
                variant,
                statement: format!("Обчисли: {n1}/{d} - {n2}/{d}"),
                operands: vec![a, b],
                operand_labels: vec![format!("{n1}/{d}"), format!("{n2}/{d}")],
                params: vec![n1, n2, d],
                answer: a.checked_sub(b)?,
                steps: vec![Frac::int(n1 - n2)],
                // Помилка: знаменник теж змінюється (подвоюється, як при «додаванні» знаменників).
                bugs: vec![BugAnswer {
                    id: "add_denominators",
                    value: Frac::new(n1 - n2, 2 * d)?,
                }],
            };
            Some((task, Frac::new(n1 - n2, d)?))
        })
    }

    fn figures(&self, task: &Task) -> Vec<tutor_core::figure::Figure> {
        crate::figures::for_task(task)
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Нам треба відняти дроби {} і {}. Що означає знаменник у кожного з них?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Знаменники однакові, тому частини однакового розміру. Що тоді треба зробити з чисельниками?".to_string(),
            ScriptedLevel::R3 => "Відніми чисельники, а знаменник залиш без змін.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![format!(
                "Віднімаємо чисельники {} і {}; знаменник {} не змінюється.",
                c.params[0], c.params[1], c.params[2]
            )],
            final_prompt: "Тепер ти: допиши відповідь у цьому прикладі.".to_string(),
        })
    }
}
