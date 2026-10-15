//! F-ADD-DIFF: додавання дробів з різними знаменниками, де один знаменник кратний іншому (сума менша за 1).

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct FAddDiff;

/// Менший знаменник; більший (кратний йому) не перевищує 20.
const SMALL_DENOMINATORS: [i64; 7] = [2, 3, 4, 5, 6, 8, 10];

impl SkillGenerator for FAddDiff {
    fn skill_id(&self) -> &'static str {
        "F-ADD-DIFF"
    }

    fn generate(&self, variant: u64) -> Task {
        build("F-ADD-DIFF", variant, |rng| {
            let d1 = SMALL_DENOMINATORS[rng.range(0, SMALL_DENOMINATORS.len() as i64 - 1) as usize];
            let k = rng.range(2, 4);
            let d2 = d1 * k;
            if d2 > 20 {
                return None;
            }
            let n1 = rng.range(1, d1 - 1);
            let n2 = rng.range(1, d2 - 1);
            if n1 * k + n2 >= d2 {
                return None;
            }
            let a = Frac::new(n1, d1)?;
            let b = Frac::new(n2, d2)?;
            let task = Task {
                skill: "F-ADD-DIFF",
                variant,
                statement: format!("Обчисли: {n1}/{d1} + {n2}/{d2}"),
                operands: vec![a, b],
                operand_labels: vec![format!("{n1}/{d1}"), format!("{n2}/{d2}")],
                params: vec![n1, d1, n2, d2, k],
                answer: a.checked_add(b)?,
                steps: vec![Frac::int(n1 * k + n2)],
                bugs: vec![BugAnswer {
                    id: "add_all_parts",
                    value: Frac::new(n1 + n2, d1 + d2)?,
                }],
            };
            Some((task, Frac::new(n1 * k + n2, d2)?))
        })
    }

    fn figures(&self, task: &Task) -> Vec<tutor_core::figure::Figure> {
        crate::figures::for_task(task)
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Знаменники різні: у дробів {} і {} частини різного розміру. Чи можна їх додавати одразу?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Спочатку треба зробити однакові знаменники. Більший знаменник ділиться на менший.".to_string(),
            ScriptedLevel::R3 => "Зведи дріб з меншим знаменником до більшого, помноживши чисельник і знаменник на одне й те саме число. Потім додай чисельники.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![format!(
                "Зводимо {}/{} до знаменника {}: множимо чисельник і знаменник на {}.",
                c.params[0], c.params[1], c.params[3], c.params[4]
            )],
            final_prompt: "Тепер ти: додай дроби з однаковими знаменниками й допиши відповідь."
                .to_string(),
        })
    }
}
