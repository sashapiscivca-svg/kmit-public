//! D-SUB: віднімання десяткових дробів із різною кількістю знаків після коми (сотих від десятих).

use crate::common::{build, find_analog};
use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub struct DSub;

/// Число в сотих (645) як десятковий запис з комою («6,45»).
fn dec2(hundredths: i64) -> String {
    format!("{},{:02}", hundredths / 100, hundredths % 100)
}

/// Число в десятих (27) як десятковий запис з комою («2,7»).
fn dec1(tenths: i64) -> String {
    format!("{},{}", tenths / 10, tenths % 10)
}

impl SkillGenerator for DSub {
    fn skill_id(&self) -> &'static str {
        "D-SUB"
    }

    fn generate(&self, variant: u64) -> Task {
        build("D-SUB", variant, |rng| {
            let whole_a = rng.range(3, 30);
            let whole_b = rng.range(1, whole_a - 1);
            let hundredths = rng.range(11, 99);
            let tenths = rng.range(1, 9);
            // Зменшуване має два знаки після коми (другий ненульовий), від'ємник один.
            if hundredths % 10 == 0 {
                return None;
            }
            let a = whole_a * 100 + hundredths;
            let b = whole_b * 10 + tenths;
            let diff = a - b * 10;
            let task = Task {
                skill: "D-SUB",
                variant,
                statement: format!("Обчисли: {} - {}", dec2(a), dec1(b)),
                operands: vec![Frac::new(a, 100)?, Frac::new(b, 10)?],
                operand_labels: vec![dec2(a), dec1(b)],
                params: vec![a, b],
                answer: Frac::new(diff, 100)?,
                // Різниця в сотих після вирівнювання розрядів.
                steps: vec![Frac::int(diff)],
                // Помилка: дробові частини віднято як цілі числа (45 - 7 = 38), без вирівнювання розрядів.
                bugs: vec![BugAnswer {
                    id: "fraction_digits_as_integers",
                    value: Frac::new((whole_a - whole_b) * 100 + hundredths - tenths, 100)?,
                }],
            };
            // Оракул: різниця цілих частин і різниця дробових частин у сотих окремо.
            let oracle = Frac::new((whole_a - whole_b) * 100 + (hundredths - tenths * 10), 100)?;
            Some((task, oracle))
        })
    }

    fn figures(&self, task: &Task) -> Vec<tutor_core::figure::Figure> {
        crate::figures::for_task(task)
    }

    fn hint(&self, task: &Task, level: ScriptedLevel) -> String {
        match level {
            ScriptedLevel::R1 => format!(
                "Ми віднімаємо десяткові дроби {} і {}. Скільки знаків після коми в кожного числа?",
                task.operand_labels[0], task.operand_labels[1]
            ),
            ScriptedLevel::R2 => "Запиши число під числом так, щоб кома стояла під комою. Там, де знаків після коми бракує, можна дописати нуль у кінці.".to_string(),
            ScriptedLevel::R3 => "Після вирівнювання віднімай, як натуральні числа, справа наліво, з позичанням. Кому постав у відповіді під комами.".to_string(),
        }
    }

    fn analog(&self, task: &Task, avoid: &[Frac]) -> Option<WorkedExample> {
        find_analog(self, task, avoid, |c| WorkedExample {
            task: c.clone(),
            shown_steps: vec![
                format!(
                    "Записуємо {} над {}, кома під комою.",
                    c.operand_labels[0], c.operand_labels[1]
                ),
                "Дописуємо нуль у кінці числа з меншою кількістю знаків після коми.".to_string(),
            ],
            final_prompt: "Тепер ти: відніми й допиши відповідь.".to_string(),
        })
    }
}
