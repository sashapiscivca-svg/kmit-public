//! F-SUB-DIFF: віднімання дробів з різними знаменниками, де один знаменник кратний іншому (різниця додатна).

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct FSubDiff;

/// Менший знаменник; більший (кратний йому) не перевищує 20.
const SMALL_DENOMINATORS: [i64; 7] = [2, 3, 4, 5, 6, 8, 10];

impl SkillGenerator for FSubDiff {
    fn skill_id(&self) -> &'static str {
        "F-SUB-DIFF"
    }

    fn generate(&self, variant: u64) -> Task {
        build("F-SUB-DIFF", variant, |rng| {
            let small =
                SMALL_DENOMINATORS[rng.range(0, SMALL_DENOMINATORS.len() as i64 - 1) as usize];
            let k = rng.range(2, 4);
            let big = small * k;
            if big > 20 {
                return None;
            }
            // Зменшуване з більшим знаменником чи з меншим: обираємо випадково.
            let big_first = rng.range(0, 1) == 1;
            // numerator — чисельник різниці над більшим знаменником.
            let (n1, d1, n2, d2, numerator) = if big_first {
                // n1/big - n2/small, треба n1 > n2 * k.
                let n2 = rng.range(1, small - 1);
                let n1 = rng.range(n2 * k + 1, big - 1);
                (n1, big, n2, small, n1 - n2 * k)
            } else {
                // n1/small - n2/big, треба n1 * k > n2.
                let n1 = rng.range(1, small - 1);
                let n2 = rng.range(1, n1 * k - 1);
                (n1, small, n2, big, n1 * k - n2)
            };
            let a = Frac::new(n1, d1)?;
            let b = Frac::new(n2, d2)?;
            let task = Task {
                skill: "F-SUB-DIFF",
                variant,
                statement: format!("Обчисли: {n1}/{d1} - {n2}/{d2}"),
                operands: vec![a, b],
                operand_labels: vec![format!("{n1}/{d1}"), format!("{n2}/{d2}")],
                params: vec![n1, d1, n2, d2, k],
                answer: a.checked_sub(b)?,
                steps: vec![Frac::int(numerator)],
                // Помилка: чисельники й знаменники віднімають окремо (від більшого менше).
                bugs: vec![BugAnswer {
                    id: "subtract_all_parts",
                    value: Frac::new((n1 - n2).abs(), (d1 - d2).abs())?,
                }],
            };
            Some((task, Frac::new(numerator, big)?))
        })
    }

    fn figures(&self, task: &Task) -> Vec<tutor_core::figure::Figure> {
        crate::figures::for_task(task)
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Знаменники різні: у дробів {} і {} частини різного розміру. Чи можна їх віднімати одразу?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Спочатку треба зробити однакові знаменники. Більший знаменник ділиться на менший.".to_string(),
            ScriptedLevel::R3 => "Зведи дріб з меншим знаменником до більшого, помноживши чисельник і знаменник на одне й те саме число. Потім відніми чисельники, а знаменник залиш.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| {
            // Дріб із меншим знаменником зводимо до більшого.
            let (n, d) = if c.params[1] < c.params[3] {
                (c.params[0], c.params[1])
            } else {
                (c.params[2], c.params[3])
            };
            WorkedExample {
                task: c.clone(),
                shown_steps: vec![format!(
                    "Зводимо {}/{} до більшого знаменника: множимо чисельник і знаменник на {}.",
                    n, d, c.params[4]
                )],
                final_prompt:
                    "Тепер ти: відніми дроби з однаковими знаменниками й допиши відповідь."
                        .to_string(),
            }
        })
    }
}
