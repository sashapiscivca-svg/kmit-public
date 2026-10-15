//! Інваріанти І1-І3: відповідь на ту саму задачу не потрапляє в підказку. Забезпечує код, а не промпт.

use crate::frac::Frac;
use crate::generator::{ScriptedLevel, SkillGenerator, WorkedExample};
use crate::numtok::numeric_tokens;
use crate::task::Task;
use std::collections::BTreeSet;

pub const SAFE_FALLBACK: &str =
    "Спробуй ще раз повільно, крок за кроком. Якщо хочеш, запитай дорослого.";

#[derive(Debug, PartialEq, Eq)]
pub struct Leak {
    pub value: Frac,
}

/// І1: чи містить текст захищене значення задачі (відповідь або проміжний результат).
pub fn find_leak(text: &str, task: &Task) -> Option<Leak> {
    let protected = task.protected_values();
    numeric_tokens(text)
        .into_iter()
        .find(|t| protected.contains(t))
        .map(|value| Leak { value })
}

/// Скриптована підказка, яка гарантовано пройшла перевірку на витік; інакше безпечний запасний текст.
pub fn safe_scripted_hint(gen: &dyn SkillGenerator, task: &Task, level: ScriptedLevel) -> String {
    let text = gen.hint(task, level);
    if find_leak(&text, task).is_some() {
        SAFE_FALLBACK.to_string()
    } else {
        text
    }
}

/// І2: аналог придатний лише якщо числа інші, відповідь інша, а показані тексти не містять
/// ні захищених значень вихідної задачі, ні фінальної відповіді самого аналога.
/// Відповідь аналога також не збігається з числом з умови, з відомою типовою помилкою задачі
/// і з відповідями, які дитина вже ввела (`rejected`): інакше дитина «отримала б» щойно відхилене число.
pub fn analog_is_safe(task: &Task, ex: &WorkedExample, rejected: &[Frac]) -> bool {
    if ex.task.skill != task.skill {
        return false;
    }
    let ops: BTreeSet<Frac> = task.operands.iter().copied().collect();
    if ex.task.operands.iter().any(|o| ops.contains(o)) {
        return false;
    }
    let prot: BTreeSet<Frac> = task.protected_values().into_iter().collect();
    if ex.task.protected_values().iter().any(|p| prot.contains(p)) {
        return false;
    }
    if ex.task.answer == task.answer
        || task.operands.contains(&ex.task.answer)
        || task.bugs.iter().any(|b| b.value == ex.task.answer)
        || rejected.contains(&ex.task.answer)
    {
        return false;
    }
    ex.texts()
        .into_iter()
        .all(|t| find_leak(t, task).is_none() && !numeric_tokens(t).contains(&ex.task.answer))
}

/// І3: якщо безпечного аналога немає, повертається `None`, а не розв'язок.
pub fn guarded_analog(
    gen: &dyn SkillGenerator,
    task: &Task,
    rejected: &[Frac],
) -> Option<WorkedExample> {
    let ex = gen.analog(task, rejected)?;
    analog_is_safe(task, &ex, rejected).then_some(ex)
}
