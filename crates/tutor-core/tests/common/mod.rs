#![allow(dead_code)]
//! Спільні тестові допоміжні речі: фіксована задача та «зловмисний» генератор, який намагається злити відповідь.

use tutor_core::frac::Frac;
use tutor_core::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use tutor_core::task::{BugAnswer, Task};

pub fn f(n: i64, d: i64) -> Frac {
    Frac::new(n, d).unwrap()
}

/// 3/8 + 2/8 = 5/8; захищені значення: 5/8 і 5.
pub fn base_task() -> Task {
    Task {
        skill: "T",
        variant: 1,
        statement: "Обчисли: 3/8 + 2/8".to_string(),
        operands: vec![f(3, 8), f(2, 8)],
        operand_labels: vec!["3/8".into(), "2/8".into()],
        params: vec![3, 2, 8],
        answer: f(5, 8),
        steps: vec![Frac::int(5)],
        bugs: vec![BugAnswer {
            id: "add_denominators",
            value: f(5, 16),
        }],
    }
}

/// 1/5 + 2/5 = 3/5; захищені значення: 3/5 і 3. Числа та відповідь інші, ніж у `base_task`.
pub fn other_task() -> Task {
    Task {
        skill: "T",
        variant: 2,
        statement: "Обчисли: 1/5 + 2/5".to_string(),
        operands: vec![f(1, 5), f(2, 5)],
        operand_labels: vec!["1/5".into(), "2/5".into()],
        params: vec![1, 2, 5],
        answer: f(3, 5),
        steps: vec![Frac::int(3)],
        bugs: vec![BugAnswer {
            id: "add_denominators",
            value: f(3, 10),
        }],
    }
}

pub fn good_example() -> WorkedExample {
    WorkedExample {
        task: other_task(),
        shown_steps: vec!["Додаємо чисельники 1 і 2.".to_string()],
        final_prompt: "Тепер ти: допиши відповідь.".to_string(),
    }
}

#[derive(Clone, Copy)]
pub enum AnalogMode {
    Missing,
    Good,
    SameAnswerAsTask,
    OverlappingOperands,
    ShowsTaskAnswer,
    ShowsOwnAnswer,
}

pub struct FakeGen {
    pub leaky_hint: bool,
    pub analog_mode: AnalogMode,
}

impl SkillGenerator for FakeGen {
    fn skill_id(&self) -> &'static str {
        "T"
    }

    fn generate(&self, _variant: u64) -> Task {
        base_task()
    }

    fn hint(&self, task: &Task, _level: ScriptedLevel) -> String {
        if self.leaky_hint {
            format!("Відповідь {}", task.answer)
        } else {
            "Подумай про знаменники.".to_string()
        }
    }

    fn analog(&self, _task: &Task, _avoid: &[Frac]) -> Option<WorkedExample> {
        let mut ex = good_example();
        match self.analog_mode {
            AnalogMode::Missing => return None,
            AnalogMode::Good => {}
            AnalogMode::SameAnswerAsTask => {
                ex.task.answer = f(5, 8);
            }
            AnalogMode::OverlappingOperands => {
                ex.task.operands = vec![f(3, 8), f(1, 8)];
            }
            AnalogMode::ShowsTaskAnswer => {
                ex.shown_steps.push("Це дає 5/8.".to_string());
            }
            AnalogMode::ShowsOwnAnswer => {
                ex.shown_steps.push("Отже 3/5.".to_string());
            }
        }
        Some(ex)
    }
}
