//! D-ADD: додавання десяткових дробів з одним знаком після коми.

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct DAdd;

/// Число в десятих (37) як десятковий запис з комою («3,7»).
fn dec(tenths: i64) -> String {
    format!("{},{}", tenths / 10, tenths % 10)
}

impl SkillGenerator for DAdd {
    fn skill_id(&self) -> &'static str {
        "D-ADD"
    }

    fn generate(&self, variant: u64) -> Task {
        build("D-ADD", variant, |rng| {
            let a = rng.range(11, 99);
            let b = rng.range(11, 99);
            if a % 10 == 0 || b % 10 == 0 {
                return None;
            }
            let (fa, fb) = (Frac::new(a, 10)?, Frac::new(b, 10)?);
            let ones = a / 10 + b / 10;
            let tenths = a % 10 + b % 10;
            // Навичка про перенесення з десятих: сума десятих має бути двоцифровою.
            if tenths < 10 {
                return None;
            }
            let task = Task {
                skill: "D-ADD",
                variant,
                statement: format!("Обчисли: {} + {}", dec(a), dec(b)),
                operands: vec![fa, fb],
                operand_labels: vec![dec(a), dec(b)],
                params: vec![a, b],
                answer: fa.checked_add(fb)?,
                steps: vec![Frac::int(tenths)],
                // Помилка: десяті додано правильно, але їх суму записано як соті.
                bugs: vec![BugAnswer {
                    id: "tenths_as_hundredths",
                    value: Frac::new(ones * 100 + tenths, 100)?,
                }],
            };
            Some((task, Frac::new(a + b, 10)?))
        })
    }

    fn figures(&self, task: &Task) -> Vec<tutor_core::figure::Figure> {
        crate::figures::for_task(task)
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Ми додаємо десяткові дроби {} і {}. Що означає цифра після коми?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Запиши число під числом так, щоб кома стояла під комою. Додавай так, як додаєш натуральні числа.".to_string(),
            ScriptedLevel::R3 => "Спочатку додай десяті, потім цілі. Якщо десятих набралося більше, ніж вміщає одна цифра, перенеси зайве до цілих.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![format!(
                "Записуємо {} під {}, кома під комою.",
                c.operand_labels[0], c.operand_labels[1]
            )],
            final_prompt: "Тепер ти: додай десяті, потім цілі й допиши відповідь.".to_string(),
        })
    }
}
