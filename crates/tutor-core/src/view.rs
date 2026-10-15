//! Те, що бачить інтерфейс дитини. Ці типи не містять жодного поля з відповіддю, проміжними
//! результатами чи типовими помилками: шар відображення фізично не може їх показати.
//! Рушій тримає `Task` і `WorkedExample` у собі й віддає назовні лише ці подання.

use crate::session::HintOutput;
use crate::task::Task;

/// Умова задачі для дитини.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildTaskView {
    pub skill: String,
    pub variant: u64,
    pub statement: String,
}

/// Аналогічний приклад для дитини: умова, показані кроки й прохання дописати кінець.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChildAnalogView {
    pub statement: String,
    pub shown_steps: Vec<String>,
    pub final_prompt: String,
}

/// Результат запиту підказки в такому вигляді, у якому його можна показувати.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChildHint {
    Text(String),
    Analog(ChildAnalogView),
    ReturnToTask,
    OfferHint,
    AskAdult,
}

pub fn task_view(task: &Task) -> ChildTaskView {
    ChildTaskView {
        skill: task.skill.to_string(),
        variant: task.variant,
        statement: task.statement.clone(),
    }
}

/// Перетворює вихід драбини на подання для дитини. Поля з відповідями відкидаються тут.
pub fn hint_view(out: HintOutput) -> ChildHint {
    match out {
        HintOutput::Text(t) => ChildHint::Text(t),
        HintOutput::Analog(ex) => ChildHint::Analog(ChildAnalogView {
            statement: ex.task.statement.clone(),
            shown_steps: ex.shown_steps.clone(),
            final_prompt: ex.final_prompt.clone(),
        }),
        HintOutput::ReturnToTask => ChildHint::ReturnToTask,
        HintOutput::OfferHint => ChildHint::OfferHint,
        HintOutput::AskAdult => ChildHint::AskAdult,
    }
}
