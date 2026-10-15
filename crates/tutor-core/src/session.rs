//! З'єднання драбини, генератора й перевірок витоку: єдине місце, де формується вивід підказки.

use crate::frac::Frac;
use crate::generator::{SkillGenerator, WorkedExample};
use crate::guard::{guarded_analog, safe_scripted_hint};
use crate::ladder::{Ladder, LadderAction};
use crate::task::Task;

#[derive(Debug)]
pub enum HintOutput {
    Text(String),
    Analog(Box<WorkedExample>),
    ReturnToTask,
    OfferHint,
    AskAdult,
}

/// Дитина просить підказку: драбина вирішує рівень, а вивід проходить перевірку на витік.
/// `rejected` — відповіді, які дитина вже ввела для цієї задачі.
pub fn next_hint(
    ladder: &mut Ladder,
    gen: &dyn SkillGenerator,
    task: &Task,
    rejected: &[Frac],
) -> HintOutput {
    match ladder.request_hint() {
        LadderAction::ShowScripted(level) => HintOutput::Text(safe_scripted_hint(gen, task, level)),
        LadderAction::ShowAnalog => match guarded_analog(gen, task, rejected) {
            Some(ex) => HintOutput::Analog(Box::new(ex)),
            None => {
                ladder.analog_unavailable();
                HintOutput::AskAdult
            }
        },
        LadderAction::ReturnToTask => HintOutput::ReturnToTask,
        LadderAction::OfferHint => HintOutput::OfferHint,
        LadderAction::AskAdult => HintOutput::AskAdult,
    }
}
